mod command;

use task_scheduler::Job;
use task_scheduler::JobRole;
use task_scheduler::scheduler::JobScheduler;
use task_scheduler::store::RedbJobStore;
use agent_interface::TelegramInterface;
use encoder::Encoder;
use encoder::find_top_n_tools;
use std::sync::Arc;
use tokio::time::{Duration};

#[tokio::main]
async fn main() {
    let mut interface = TelegramInterface::start().await;

    let store = Arc::new(RedbJobStore::new("scheduler.redb"));
    let scheduler = Arc::new(JobScheduler::new(store));

    let encoder = match Encoder::new() {
        Ok(enc) => enc,
        Err(e) => {
            eprintln!("❌ Failed to initialize encoder: {e}");
            return;
        }
    };

    let (signal_tx, mut signal_rx) = tokio::sync::mpsc::channel(32);

    loop {
        tokio::select! {

            // Treat incoming User messages
            Some((chat_id, text)) = interface.rx.recv() => {
                let (control, responses) = command::resolve_commands(&text);

                // Acknowledge User on commmand result
                for response in responses {
                    let _ = interface.tx.send((chat_id, response)).await;
                }

                // Check if User command shutdown
                match control {
                    command::LoopControl::Continue(is_empty) => {
                        // Only queue the message if it doesn't contain any commands
                        if is_empty { 

                            let result = find_top_n_tools(&encoder, &text, 1);
                            let (name, _) = &result.unwrap()[0];
                            let payload = format!("Task: {}; msg: {}", name, text); 

                            scheduler.enqueue(chat_id, payload, JobRole::Embed);
                            let _ = interface.tx.send((chat_id, "Queued...".into())).await;

                            // If nothing is running, kick off immediately
                            if let Some(job) = scheduler.next_job() {
                                let tx = signal_tx.clone();
                                tokio::spawn(async move {
                                    let result = process_job(&job).await;
                                    let _ = tx.send(result).await;
                                });
                            }
                        }
                    }
                    command::LoopControl::Break => {
                        let _ = interface.tx.send((chat_id, "System shutting down... 🔌".to_string())).await;
                        break;
                    }
                }
            }

            // Job finished
            Some(job) = signal_rx.recv() => {
                scheduler.complete(job.id);

                match job.role {
                    JobRole::Respond => {
                        let reply = format!("Response: {}", job.payload);
                        let _ = interface.tx.send((job.chat_id, reply)).await;
                    }
                    _ => {
                        if let Some(next_stage) = job.next_role() {
                            scheduler.enqueue_job(next_stage);
                        }
                    }
                }

                // Always try to dispatch the next queued job
                if let Some(next_job) = scheduler.next_job() {
                    let tx = signal_tx.clone();
                    tokio::spawn(async move {
                        let result = process_job(&next_job).await;
                        let _ = tx.send(result).await;
                    });
                }
            }

            // Also keep the local Ctrl+C escape
            _ = tokio::signal::ctrl_c() => {
                println!("Local shutdown (Ctrl+C) triggered.");
                break;
            }
        }
    }
}

/// Dispatch the actual work based on the job's current role
async fn process_job(job: &Job) -> Job {
    match job.role {
        JobRole::Embed => {
            tokio::time::sleep(Duration::from_millis(200)).await;
            // e.g. call embedding model, store result in payload
            Job { payload: format!("[embedded] {}", job.payload), ..job.clone() }
        }
        JobRole::Interpret => {
            tokio::time::sleep(Duration::from_millis(300)).await;
            // e.g. semantic analysis
            Job { payload: format!("[interpreted] {}", job.payload), ..job.clone() }
        }
        JobRole::Call => {
            tokio::time::sleep(Duration::from_millis(500)).await;
            // e.g. LLM call or tool use
            Job { payload: format!("[called] {}", job.payload), ..job.clone() }
        }
        JobRole::Respond => {
            // Final formatting before reply
            Job { payload: format!("[response] {}", job.payload), ..job.clone() }
        }
    }
}