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
pub enum JobRole{
    Embed,
    Interpret,
    Call,
    Respond
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: uuid::Uuid,
    pub chat_id: ChatId,
    pub sequence: u64,
    pub user_message: Option<String>,
    pub payload: String,
    pub status: JobStatus,
    pub role: JobRole
}

impl Job {
    /// Advance to the next stage in the pipeline
    pub fn next_role(&self) -> Option<Job> {
        let next = match self.role {
            JobRole::Embed     => Some(JobRole::Interpret),
            JobRole::Interpret => Some(JobRole::Call),
            JobRole::Call      => Some(JobRole::Respond),
            JobRole::Respond   => None, // Pipeline complete
        };
        next.map(|role| Job { role, ..self.clone() })
    }
}
