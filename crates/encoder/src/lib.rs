pub mod encoder;
pub mod tools_embedding;

pub use encoder::*;
pub use tools_embedding::*;

pub fn offline_embedding() -> anyhow::Result<()>{
    let result = offline_tools_embedding();
    result
}