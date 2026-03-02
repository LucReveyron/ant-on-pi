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
        Ok(enc) => Arc::new(enc),
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

                            scheduler.enqueue(chat_id, Some(text),  "".to_string(), JobRole::Embed);

                            // Acknowledge the user
                            let _ = interface.tx.send((chat_id, "Queued...".into())).await;

                            // If nothing is running, kick off immediately
                            if let Some(job) = scheduler.next_job() {
                                let tx = signal_tx.clone();
                                let encoder = encoder.clone();
                                tokio::spawn(async move {
                                    let result = process_job(&job, &encoder).await;
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
                        // Update job
                        if let Some(next_stage) = job.next_role() {
                            scheduler.enqueue_job(next_stage);
                        }
                    }
                }

                // Always try to dispatch the next queued job
                if let Some(next_job) = scheduler.next_job() {
                    let tx = signal_tx.clone();
                    let encoder = encoder.clone();
                    tokio::spawn(async move {
                        let result = process_job(&next_job, &encoder).await;
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
async fn process_job(job: &Job, encoder: &Arc<Encoder>) -> Job {
    match job.role {
        JobRole::Embed => {
            // Embed user message is available, else treat directly payload
            let text = match &job.user_message {
                Some(t) => t,
                None => &job.payload
            };

            // TODO: 03/02/26 - Generalise to make a liste of closer tools if needed
            // Find closer tool
            let result = find_top_n_tools(&encoder, &text, 1);
            let (name, _) = &result.unwrap()[0];
            let payload = format!("Task: {}", name);
            Job { payload: payload, ..job.clone() }
        }
        JobRole::Interpret => {
            tokio::time::sleep(Duration::from_millis(300)).await;
            // TODO: 03/02/26 - Implement and call ask_llm
            // e.g. use LLM to interprete messages using embedding info. 
            Job { payload: format!("[interpreted] {}", job.payload), ..job.clone() }
        }
        JobRole::Call => {
            tokio::time::sleep(Duration::from_millis(500)).await;
            // TODO: 03/02/26 - Call function caller to dispatch call to the correct tool if don't find, 
            // recall ask_llm to re-interprete calling info.
            // e.g. LLM call or tool use
            Job { payload: format!("[called] {}", job.payload), ..job.clone() }
        }
        JobRole::Respond => {
            // TODO: 03/02/26 - Based on result, save result in file, update a database or generate a message for the user
            // Final formatting before reply
            Job { payload: format!("[response] {}", job.payload), ..job.clone() }
        }
    }
}