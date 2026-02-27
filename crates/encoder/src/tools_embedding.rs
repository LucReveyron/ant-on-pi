// tools_embedding.rs

use anyhow::Result;
use crate::encoder::{Encoder, cosine_similarity};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

#[derive(Deserialize)]
struct ToolsFile {
    tools: Vec<Tool>,
}

#[derive(Deserialize)]
struct Tool {
    name: String,
    description: String,
    arguments: HashMap<String, Argument>,
    examples: Vec<String>,
}

#[derive(Deserialize, Serialize, Clone)]
struct Argument {
    #[serde(rename = "type")]
    arg_type: String,
    description: String,
}

#[derive(Deserialize, Serialize)]
struct ToolWithEmbedding {
    name: String,
    description: String,
    arguments: HashMap<String, Argument>,
    examples: Vec<String>,
    embedding: Vec<f32>,
}

/// Build rich semantic text for embedding
fn build_embedding_text(tool: &Tool) -> String {
    let mut text = String::new();

    text.push_str(&format!("Tool name: {}\n", tool.name));
    text.push_str(&format!("Description: {}\n", tool.description));

    text.push_str("Arguments:\n");
    for (arg_name, arg) in &tool.arguments {
        text.push_str(&format!(
            "- {} ({}): {}\n",
            arg_name, arg.arg_type, arg.description
        ));
    }

    text.push_str("Examples:\n");
    for example in &tool.examples {
        text.push_str(&format!("- {}\n", example));
    }

    text
}

pub fn offline_tools_embedding() -> Result<()> {
    // 1️⃣ Read tools_list.json
    let data = fs::read_to_string("../tools/tools_list.json")?;
    let tools_file: ToolsFile = serde_json::from_str(&data)?;

    // 2️⃣ Create encoder
    let encoder = Encoder::new()?;

    // 3️⃣ Compute embeddings tool by tool
    let mut enriched_tools = Vec::new();

    for tool in tools_file.tools {
        let text = build_embedding_text(&tool);
        let embedding = encoder.encode(&text)?;

        enriched_tools.push(ToolWithEmbedding {
            name: tool.name,
            description: tool.description,
            arguments: tool.arguments,
            examples: tool.examples,
            embedding,
        });
    }

    // 4️⃣ Save result
    let output_json = serde_json::to_string_pretty(&enriched_tools)?;
    fs::write("../encoder/tools_embeddings.json", output_json)?;

    println!("Embeddings saved to tools_embeddings.json");

    Ok(())
}

pub fn find_top_n_tools(
    encoder: &Encoder,
    user_input: &str,
    n: usize,
) -> Result<Vec<(String, f32)>> {
    // 1️⃣ Add BGE query prefix
    let prefixed_query = format!(
        "Represent this sentence for searching relevant passages: {}",
        user_input
    );

    // 2️⃣ Encode query
    let query_embedding = encoder.encode(&prefixed_query)?;

    // 3️⃣ Load stored embeddings
    let data = fs::read_to_string("../encoder/tools_embeddings.json")?;
    let tools: Vec<ToolWithEmbedding> = serde_json::from_str(&data)?;

    // 4️⃣ Compute similarity scores
    let mut scored: Vec<(String, f32)> = tools
        .iter()
        .map(|tool| {
            let score = cosine_similarity(&query_embedding, &tool.embedding);
            (tool.name.clone(), score)
        })
        .collect();

    // 5️⃣ Sort descending
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    // 6️⃣ Return top N
    Ok(scored.into_iter().take(n).collect())
}