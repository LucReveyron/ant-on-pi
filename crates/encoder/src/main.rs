use encoder::offline_tools_embedding;
use encoder::tools_embedding::find_top_n_tools;
use encoder::Encoder;

fn main() {
    
    match offline_tools_embedding() {
        Ok(_) => println!("Done."),
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("Caused by: {:?}", e);
            std::process::exit(1);
        }
    }

    // 1️⃣ Create encoder
    let encoder = match Encoder::new() {
        Ok(enc) => enc,
        Err(e) => {
            eprintln!("❌ Failed to initialize encoder: {e}");
            return;
        }
    };

    let user_input = "LLM : convert that swiss phone number in french format : 768417100";

    // 2️⃣ Find closest tools
    match find_top_n_tools(&encoder, user_input, 1) {
        Ok(results) => {
            if results.is_empty() {
                println!("No matching tools found.");
            } else {
                println!("Top matching tools:");
                for (name, score) in results {
                    println!("• {name} (score: {score:.4})");
                }
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to compute similarity: {e}");
        }
    }
}