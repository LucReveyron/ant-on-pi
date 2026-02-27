use fastembed::{TextEmbedding, InitOptions};
use anyhow::Result;

/// Wrapper around fastembed model
pub struct Encoder {
    model: TextEmbedding,
}

impl Encoder {
    /// Create new encoder (uses default BGE-small-en-v1.5)
    pub fn new() -> Result<Self> {
        let model = TextEmbedding::try_new(InitOptions::default())?;
        Ok(Self { model })
    }

    /// Encode a single string into a normalized embedding
    pub fn encode(&self, text: &str) -> Result<Vec<f32>> {
        let embeddings = self.model.embed(vec![text], None)?;
        let mut embedding = embeddings.into_iter().next().unwrap();

        // L2 normalize (important for cosine similarity)
        let norm = embedding.iter().map(|v| v * v).sum::<f32>().sqrt().max(1e-12);
        for v in &mut embedding {
            *v /= norm;
        }

        Ok(embedding)
    }
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}