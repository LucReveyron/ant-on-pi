use teloxide::types::ChatId; 
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JobStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: uuid::Uuid,
    pub chat_id: ChatId,
    pub sequence: u64,
    pub payload: String,
    pub status: JobStatus,
}
