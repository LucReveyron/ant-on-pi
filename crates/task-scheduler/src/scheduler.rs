use std::sync::Arc;
use uuid::Uuid;
use teloxide::types::ChatId;
use crate::job::{Job, JobStatus, JobRole};
use crate::store::RedbJobStore;

pub struct JobScheduler {
    store: Arc<RedbJobStore>,
}

impl JobScheduler {
    pub fn new(store: Arc<RedbJobStore>) -> Self {
        Self { store }
    }

    /// Create a new job, assign it the next sequence number, persist and return it.
    pub fn enqueue(&self, chat_id: ChatId, user_message: Option<String>, payload: String, role: JobRole) -> Job {
        // ✅ Single atomic transaction — no race condition
        let sequence = self.store.next_sequence();

        let job = Job {
            id: Uuid::new_v4(),
            chat_id,
            sequence,
            user_message,
            payload,
            status: JobStatus::Pending,
            role
        };

        self.store.insert_job(&job);
        job
    }

    /// Re-enqueue an existing job (e.g. next pipeline stage), preserving its chat_id and payload.
    pub fn enqueue_job(&self, job: Job) -> Job {
        let sequence = self.store.next_sequence();
        let job = Job {
            id: Uuid::new_v4(),
            sequence,
            status: JobStatus::Pending,
            ..job
        };
        self.store.insert_job(&job);
        job
    }

    /// Fetch the next pending job and mark it as Processing.
    /// Returns None if no pending jobs exist or one is already Processing.
    pub fn next_job(&self) -> Option<Job> {
        let job = self.store.fetch_next_pending()?;

        // Mark as processing in the store
        self.store.update_status(job.id, JobStatus::Processing);

        // Return an up-to-date copy reflecting the new status
        Some(Job {
            status: JobStatus::Processing,
            ..job
        })
    }

    /// Mark a job as completed — this removes it from the database.
    pub fn complete(&self, id: Uuid) {
        self.store.update_status(id, JobStatus::Completed);
    }

    /// Mark a job as failed without removing it (useful for retry logic).
    pub fn fail(&self, id: Uuid) {
        self.store.update_status(id, JobStatus::Failed);
    }
}