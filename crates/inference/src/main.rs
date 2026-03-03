use inference::{LlmEngine, LlmError};

fn main() -> Result<(), LlmError> {
    let engine = LlmEngine::new()?;

    let questions = [
        "You are a deterministic temporal extractor.

Task:
Extract only the **exact text substrings** that refer to dates and times from the message. Return Output only not reasonning.

Do NOT:
- Convert relative dates into absolute calendar dates.
- Infer missing components.
- Add any information not explicitly in the message.

Message: I see Alice next monday at 14:00",
    ];

    for question in &questions {
        println!("Q: {}", question);
        let answer = engine.ask_llm(question.to_string())?;
        println!("A: {}\n", answer);
    }

    Ok(())
}