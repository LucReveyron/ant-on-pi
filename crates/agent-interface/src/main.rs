use teloxide::prelude::Requester;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting bot...");

    let bot = teloxide::Bot::from_env();

    teloxide::repl(bot, |bot: teloxide::Bot, msg: teloxide::prelude::Message| async move {
        if let Some(text) = msg.text() {
            bot.send_message(msg.chat.id, format!("Agent Echo: {}", text))
                .await?;
        }
        Ok(())
    })
    .await;
}