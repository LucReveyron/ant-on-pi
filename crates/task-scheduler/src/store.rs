use crate::job::{Job, JobStatus};
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use std::sync::Arc;
use uuid::Uuid;

pub const JOBS_TABLE: TableDefinition<[u8; 16], &[u8]> = TableDefinition::new("jobs");
pub const SEQUENCE_TABLE: TableDefinition<u64, [u8; 16]> = TableDefinition::new("sequence_index");
pub const META_TABLE: TableDefinition<&str, u64> = TableDefinition::new("meta"); // ✅ &str not str

pub struct RedbJobStore {
    pub db: Arc<Database>,
}

impl RedbJobStore {
    pub fn new(path: &str) -> Self {
        let db = Database::create(path).expect("Failed to open database");
        let store = Self { db: Arc::new(db) };
        store.init();
        store
    }

    /// Creates all tables if they don't exist yet.
    /// Must be called once before any read transactions.
    fn init(&self) {
        let txn = self.db.begin_write().expect("Failed to begin write");
        txn.open_table(JOBS_TABLE).expect("Failed to init jobs table");
        txn.open_table(SEQUENCE_TABLE).expect("Failed to init sequence table");
        txn.open_table(META_TABLE).expect("Failed to init meta table");
        txn.commit().expect("Failed to commit init");
    }

    /// Atomically increment and return the next sequence number
    pub fn next_sequence(&self) -> u64 {
        let txn = self.db.begin_write().expect("Failed to begin write");
        let next = {
            let mut table = txn.open_table(META_TABLE).expect("Failed to open meta table");
            let current = table
                .get("last_sequence")
                .unwrap_or(None)
                .map(|g| g.value())
                .unwrap_or(0);
            let next = current + 1;
            table
                .insert("last_sequence", &next)
                .expect("Failed to update sequence");
            next
        };
        txn.commit().expect("Failed to commit");
        next
    }

    pub fn insert_job(&self, job: &Job) {
        let txn = self.db.begin_write().expect("Failed to begin write");
        {
            let mut jobs = txn.open_table(JOBS_TABLE).expect("Failed to open jobs table");
            let mut seq_index = txn
                .open_table(SEQUENCE_TABLE)
                .expect("Failed to open seq table");

            let bytes = serde_json::to_vec(job).expect("Failed to serialize job");

            jobs.insert(&job.id.into_bytes(), bytes.as_slice())
                .expect("Failed to insert job");

            // Store raw UUID bytes in SEQUENCE_TABLE — no serialization needed
            seq_index
                .insert(&job.sequence, &job.id.into_bytes())
                .expect("Failed to insert sequence entry");
        }
        txn.commit().expect("Failed to commit");
    }

    pub fn update_status(&self, id: Uuid, status: JobStatus) {
        let txn = self.db.begin_write().expect("Failed to begin write");
        {
            let mut jobs = txn.open_table(JOBS_TABLE).expect("Failed to open jobs table");

            // Extract owned bytes immediately, releasing the borrow on `jobs`
            // before we need to mutate it
            let existing: Option<Job> = {
                let guard = jobs
                    .get(&id.into_bytes())
                    .expect("Failed to get job");

                guard.map(|g| {
                    serde_json::from_slice(g.value()).expect("Failed to deserialize job")
                })
                // guard is dropped here — borrow on `jobs` fully released
            };

            if let Some(mut job) = existing {
                if status == JobStatus::Completed {
                    jobs.remove(&id.into_bytes())
                        .expect("Failed to remove job");

                    let mut seq_index = txn
                        .open_table(SEQUENCE_TABLE)
                        .expect("Failed to open seq table");
                    seq_index
                        .remove(&job.sequence)
                        .expect("Failed to remove sequence entry");
                } else {
                    job.status = status;
                    let new_bytes =
                        serde_json::to_vec(&job).expect("Failed to serialize updated job");
                    jobs.insert(&id.into_bytes(), new_bytes.as_slice())
                        .expect("Failed to update job");
                }
            }
        }
        txn.commit().expect("Failed to commit");
    }

    pub fn fetch_next_pending(&self) -> Option<Job> {
        // ReadableDatabase must be in scope for begin_read()
        let txn = self.db.begin_read().expect("Failed to begin read");
        let jobs_table = txn.open_table(JOBS_TABLE).expect("Failed to open jobs table");
        let seq_table = txn
            .open_table(SEQUENCE_TABLE)
            .expect("Failed to open seq table");

        // Block if any job is already processing
        for entry in jobs_table.iter().expect("Failed to iterate jobs") {
            let (_, v) = entry.expect("Failed to read entry");
            let job: Job =
                serde_json::from_slice(v.value()).expect("Failed to deserialize job");
            if job.status == JobStatus::Processing {
                return None;
            }
        }

        // Walk in sequence order (FIFO), return first Pending
        for entry in seq_table.iter().expect("Failed to iterate sequences") {
            let (_, id_bytes) = entry.expect("Failed to read sequence entry");
            let id = Uuid::from_bytes(id_bytes.value());
            if let Some(job_bytes) = jobs_table
                .get(&id.into_bytes())
                .expect("Failed to get job")
            {
                let job: Job =
                    serde_json::from_slice(job_bytes.value()).expect("Failed to deserialize job");
                if job.status == JobStatus::Pending {
                    return Some(job);
                }
            }
        }

        None
    }
}