/*use agent_interface::TelegramInterface;

#[tokio::main]
async fn main() {
    let mut interface = TelegramInterface::start().await;
    let admin_id: &str = "8729566228";

    println!("Agent is monitoring for commands...");

    loop {
        tokio::select! {
            // Check for incoming messages
            Some((chat_id, demand)) = interface.rx.recv() => {

                if demand == "/shutdown" {
                    // SECURITY: Check if the sender is actually YOU
                    if chat_id.to_string() != admin_id { continue; } 

                    println!("Shutdown command received from Telegram.");
                    
                    // 1. Send a "Goodbye" message
                    let _ = interface.tx.send((chat_id, "System shutting down... 🔌".to_string())).await;
                    
                    // 2. Break the loop
                    break;
                }

                // Normal processing for everything else
                let response = core_logic_process(demand);
                let _ = interface.tx.send((chat_id, response)).await;
            }

            // Also keep the local Ctrl+C escape
            _ = tokio::signal::ctrl_c() => {
                println!("Local shutdown (Ctrl+C) triggered.");
                break;
            }
        }
    }

    println!("Agent has successfully exited.");
}

fn core_logic_process(input: String) -> String {
    format!("Agent analysis complete for: '{}'", input)
}*/

use task_scheduler::scheduler::JobScheduler;
use task_scheduler::store::RedbJobStore;
use agent_interface::TelegramInterface;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    let mut interface = TelegramInterface::start().await;

    let store = Arc::new(RedbJobStore::new("scheduler.redb"));
    let scheduler = Arc::new(JobScheduler::new(store));

    let (signal_tx, mut signal_rx) = tokio::sync::mpsc::channel(32);

    loop {
        tokio::select! {

            // 1️⃣ User sends message
            Some((chat_id, text)) = interface.rx.recv() => {
                scheduler.enqueue(chat_id, text);
                let _ = interface.tx.send((chat_id, "Queued...".into())).await;
            }

            // 2️⃣ Scheduler tick
            _ = sleep(Duration::from_millis(500)) => {
                if let Some(job) = scheduler.next_job() {
                    let tx = signal_tx.clone();

                    tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_secs(3)).await;
                        let _ = tx.send(job).await;
                    });
                }
            }

            // 3️⃣ Job finished
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