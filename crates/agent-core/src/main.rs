mod command;

use task_scheduler::scheduler::JobScheduler;
use task_scheduler::store::RedbJobStore;
use agent_interface::TelegramInterface;
use encoder::Encoder;
use encoder::find_top_n_tools;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

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
                        if is_empty { // TODO: 02/27/26 Improve that by adding a new state to LoopControl

                            let result = find_top_n_tools(&encoder, &text, 1);
                            let (name, _) = &result.unwrap()[0];
                            scheduler.enqueue(chat_id, (&name).to_string());
                            let _ = interface.tx.send((chat_id, "Queued...".into())).await;
                        }
                    }
                    command::LoopControl::Break => {
                        let _ = interface.tx.send((chat_id, "System shutting down... 🔌".to_string())).await;
                        break;
                    }
                }
            }

            // Scheduler tick
            _ = sleep(Duration::from_millis(500)) => {
                if let Some(job) = scheduler.next_job() {
                    let tx = signal_tx.clone();

                    tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_secs(3)).await;
                        let _ = tx.send(job).await;
                    });
                }
            }

            // Job finished
            Some(job) = signal_rx.recv() => {
                scheduler.complete(job.id);
                let reply = format!("Echo: {}", job.payload);
                let _ = interface.tx.send((job.chat_id, reply)).await;
            }

            // Also keep the local Ctrl+C escape
            _ = tokio::signal::ctrl_c() => {
                println!("Local shutdown (Ctrl+C) triggered.");
                break;
            }
        }
    }
}