mod tgbot;
mod tgext;
use flowsnet_platform_sdk::logger;

use tg_flows::{listen_to_update, update_handler, Update};

use tgbot::TgBot;

#[no_mangle]
#[tokio::main(flavor = "current_thread")]
pub async fn on_deploy() {
    logger::init();
    let res = TgBot::default().set_bot_commands();
    if res.is_err() {
        log::error!("failed to set bot commands: {:?}", res.err())
    }

    let telegram_token = std::env::var("telegram_token").unwrap();
    listen_to_update(telegram_token).await;
}

#[update_handler]
async fn handler(update: Update) {
    TgBot::default().handle_update(update).await.unwrap();
}
