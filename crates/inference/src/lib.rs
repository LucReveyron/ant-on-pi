use llama_cpp_2::{
    context::params::LlamaContextParams,
    llama_backend::LlamaBackend,
    llama_batch::LlamaBatch,
    model::{params::LlamaModelParams, AddBos, LlamaModel},
    token::LlamaToken,
    context::LlamaContext,
    sampling::LlamaSampler,
};
use hf_hub::{api::sync::Api, Repo, RepoType};
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use thiserror::Error;

// ── Constants ─────────────────────────────────────────────────────────────────

const MODEL_DIR: &str = "models";
const MODEL_FILE: &str = "Llama-3.2-1B-Instruct-Q8_0.gguf";
const HF_REPO_ID: &str = "bartowski/Llama-3.2-1B-Instruct-GGUF";
const CONTEXT_SIZE: u32 = 512;
const CPU_THREADS: i32 = 4;
const MAX_NEW_TOKENS: usize = 256;
const TEMPERATURE: f32 = 0.7;
const TOP_P: f32 = 0.9;

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum LlmError {
    #[error("Model download failed: {0}")]
    Download(String),
    #[error("Backend init failed: {0}")]
    Backend(String),
    #[error("Model load failed: {0}")]
    ModelLoad(String),
    #[error("Inference failed: {0}")]
    Inference(String),
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Holds the backend and model weights.
/// Create once with `LlmEngine::new()`, then call `ask_llm()` as many times as needed.
/// If the model file is not found locally, it is downloaded automatically from Hugging Face.
pub struct LlmEngine {
    backend: LlamaBackend,
    model: LlamaModel,
}

impl LlmEngine {
    /// Load the model from disk, downloading it first if not present.
    /// Call this once at startup.
    pub fn new() -> Result<Self, LlmError> {
        let model_path = ensure_model_available()?;
        let backend = init_backend()?;
        let model = load_model(&backend, &model_path)?;
        Ok(Self { backend, model })
    }

    /// Feed `text` directly to the model and return the generated response.
    pub fn ask_llm(&self, text: String) -> Result<String, LlmError> {
        // Format the input to follow the instruction/response structure
        let formatted_prompt = format!(
            "### Instruction:\n{} \n ### Response: \n",
            text
        );

        let mut ctx = create_context(&self.backend, &self.model)?;
        let tokens = tokenize(&self.model, &formatted_prompt)?;

        let mut batch = LlamaBatch::new(tokens.len(), 1);
        let last_index = (tokens.len() - 1) as i32;

        for (i, &tok) in tokens.iter().enumerate() {
            batch.add(tok, i as i32, &[0], i as i32 == last_index)
                .map_err(|e| LlmError::Inference(e.to_string()))?;
        }

        ctx.decode(&mut batch)
            .map_err(|e| LlmError::Inference(e.to_string()))?;

        let response = generate_tokens(&mut ctx, &self.model, &mut batch, tokens.len())?;
        
        // Post-process to extract only the response after "### Response:"
        let clean_response = response
            .split("### Response:")
            .last()
            .unwrap_or(&response)
            .trim()
            .to_string();

        Ok(clean_response)
    }
}

// ── Model availability ────────────────────────────────────────────────────────

/// Returns the local model path, downloading from Hugging Face if necessary.
fn ensure_model_available() -> Result<PathBuf, LlmError> {
    let local_path = PathBuf::from(MODEL_DIR).join(MODEL_FILE);

    if local_path.exists() {
        println!("Model found at: {}", local_path.display());
        return Ok(local_path);
    }

    println!("Model not found locally, downloading from Hugging Face...");
    println!("  Repo : {}", HF_REPO_ID);
    println!("  File : {}", MODEL_FILE);

    let downloaded_path = download_model_from_hub()?;
    copy_model_to_local_dir(&downloaded_path, &local_path)?;

    println!("Model ready at: {}", local_path.display());
    Ok(local_path)
}

fn download_model_from_hub() -> Result<PathBuf, LlmError> {
    let api = Api::new()
        .map_err(|e| LlmError::Download(e.to_string()))?;

    let repo = api.repo(Repo::new(HF_REPO_ID.to_string(), RepoType::Model));

    let path = repo
        .get(MODEL_FILE)
        .map_err(|e| LlmError::Download(format!("Failed to download '{}': {}", MODEL_FILE, e)))?;

    Ok(path)
}

fn copy_model_to_local_dir(src: &Path, dest: &PathBuf) -> Result<(), LlmError> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| LlmError::Download(format!("Failed to create models/ dir: {}", e)))?;
    }

    std::fs::copy(src, dest)
        .map_err(|e| LlmError::Download(format!("Failed to copy model to models/: {}", e)))?;

    Ok(())
}

// ── Init helpers ──────────────────────────────────────────────────────────────

fn init_backend() -> Result<LlamaBackend, LlmError> {
    LlamaBackend::init()
        .map_err(|e| LlmError::Backend(e.to_string()))
}

fn load_model(backend: &LlamaBackend, path: &Path) -> Result<LlamaModel, LlmError> {
    LlamaModel::load_from_file(
        backend,
        path,
        &LlamaModelParams::default().with_n_gpu_layers(0),
    )
    .map_err(|e| LlmError::ModelLoad(e.to_string()))
}

// ── Inference helpers ─────────────────────────────────────────────────────────

fn create_context<'a>(backend: &'a LlamaBackend, model: &'a LlamaModel) -> Result<LlamaContext<'a>, LlmError> {
    model
        .new_context(
            backend,
            LlamaContextParams::default()
                .with_n_ctx(Some(NonZeroU32::new(CONTEXT_SIZE).unwrap()))  // fix: wrap in Some()
                .with_n_threads(CPU_THREADS)                                // fix: i32 not u32
                .with_n_threads_batch(CPU_THREADS),
        )
        .map_err(|e| LlmError::Inference(e.to_string()))
}

fn tokenize(model: &LlamaModel, text: &str) -> Result<Vec<LlamaToken>, LlmError> {
    model
        .str_to_token(text, AddBos::Always)
        .map_err(|e| LlmError::Inference(e.to_string()))
}

fn build_sampler() -> LlamaSampler {
    LlamaSampler::chain_simple([
        LlamaSampler::temp(TEMPERATURE),
        LlamaSampler::top_p(TOP_P, 1),
        LlamaSampler::greedy(),
    ])
}

fn decode_token_to_string(model: &LlamaModel, token: LlamaToken) -> Result<String, LlmError> {
    let mut decoder = encoding_rs::UTF_8.new_decoder();
    model
        .token_to_piece(token, &mut decoder, false, None)
        .map_err(|e| LlmError::Inference(e.to_string()))
}

fn advance_context(ctx: &mut LlamaContext, batch: &mut LlamaBatch, token: LlamaToken, pos: i32) -> Result<(), LlmError> {
    batch.clear();
    batch.add(token, pos, &[0], true)
        .map_err(|e| LlmError::Inference(e.to_string()))?;
    ctx.decode(batch)
        .map_err(|e| LlmError::Inference(e.to_string()))
}

fn generate_tokens(
    ctx: &mut LlamaContext,
    model: &LlamaModel,
    batch: &mut LlamaBatch,
    prompt_len: usize,
) -> Result<String, LlmError> {
    let mut sampler = build_sampler();
    let mut response = String::new();
    let mut n_past = prompt_len as i32;

    for _ in 0..MAX_NEW_TOKENS {
        // fix: sample() now takes the context and last token index
        let next_token = sampler.sample(ctx, batch.n_tokens() - 1);

        if next_token == model.token_eos() {
            break;
        }

        response.push_str(&decode_token_to_string(model, next_token)?);
        advance_context(ctx, batch, next_token, n_past)?;
        n_past += 1;
    }

    Ok(response.trim().to_string())
}