use teloxide::prelude::*;
use tokio::sync::mpsc;

pub struct TelegramInterface {
    pub tx: mpsc::Sender<(ChatId, String)>, // Agent sends TO this
    pub rx: mpsc::Receiver<(ChatId, String)>, // Agent receives FROM this
}

impl TelegramInterface {
    pub async fn start() -> Self {
        let (inbound_tx, inbound_rx) = mpsc::channel::<(ChatId, String)>(100);
        let (outbound_tx, mut outbound_rx) = mpsc::channel::<(ChatId, String)>(100);

        let bot = Bot::from_env();
        let bot_for_repl = bot.clone();

        // Spawn the Telegram event loop in the background
        tokio::spawn(async move {
            teloxide::repl(bot_for_repl, move |_bot: Bot, msg: Message| {
                let inbound_tx = inbound_tx.clone();
                async move {
                    if let Some(text) = msg.text() {
                        let _ = inbound_tx.send((msg.chat.id, text.to_string())).await;
                    }
                    respond(())
                }
            })
            .await;
        });

        // Spawn a second task to listen for outgoing messages from your agent
        let bot_for_sending = bot.clone();
        tokio::spawn(async move {
            while let Some((chat_id, text)) = outbound_rx.recv().await {
                let _ = bot_for_sending.send_message(chat_id, text).await;
            }
        });

        Self {
            tx: outbound_tx,
            rx: inbound_rx,
        }
    }
}
