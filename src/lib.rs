use flowsnet_platform_sdk::logger;
use openai_flows::{
    chat::{ChatModel, ChatOptions},
    OpenAIFlows,
};
use serde_json::json;
use store_flows::{get, set};
use tg_flows::{listen_to_update, update_handler, Message, Telegram, Update, UpdateKind};

#[no_mangle]
#[tokio::main(flavor = "current_thread")]
pub async fn on_deploy() {
    let telegram_token = std::env::var("telegram_token").unwrap();
    listen_to_update(telegram_token).await;
}

trait TelegramExtension {
    fn reply_to_message<T>(&self, msg: &Message, text: T) -> anyhow::Result<Message>
    where
        T: Into<String>;
}

impl TelegramExtension for Telegram {
    fn reply_to_message<T>(&self, msg: &Message, text: T) -> anyhow::Result<Message>
    where
        T: Into<String>,
    {
        let text: String = text.into();
        let body = serde_json::json!({
            "chat_id": msg.chat.id,
            "reply_to_message_id": msg.id,
            "text": text,
        });
        log::info!("reply message: {}", body);
        self.request(tg_flows::Method::SendMessage, body.to_string().as_bytes())
    }
}

#[update_handler]
async fn handler(update: Update) {
    logger::init();
    let telegram_token = std::env::var("telegram_token").unwrap();
    let placeholder_text = std::env::var("placeholder").unwrap_or("Typing ...".to_string());
    let system_prompt = std::env::var("system_prompt")
        .unwrap_or("You are a helpful assistant answering questions on Telegram.".to_string());
    let help_mesg = std::env::var("help_mesg").unwrap_or(
        r#"
        I am your assistant on Telegram. Ask me any question!
        To start a new conversation, type the /restart command.
        To print this message, type /help.
        "#
        .to_string(),
    );

    let tele = Telegram::new(telegram_token.to_string());

    if let UpdateKind::Message(msg) = update.kind {
        let chat_id = msg.chat.id;
        log::info!("Received message from {}", chat_id);

        let mut openai = OpenAIFlows::new();
        openai.set_retry_times(3);
        let mut co = ChatOptions::default();
        // co.model = ChatModel::GPT4;
        co.model = ChatModel::GPT35Turbo16K;
        co.restart = false;
        co.system_prompt = Some(&system_prompt);

        let text = msg.text().unwrap_or("");
        if text.eq_ignore_ascii_case("/help") {
            _ = tele.send_message(chat_id, &help_mesg);
        } else if text.eq_ignore_ascii_case("/start") {
            _ = tele.send_message(chat_id, &help_mesg);
            set(&chat_id.to_string(), json!(true), None);
            log::info!("Started converstion for {}", chat_id);
        } else if text.eq_ignore_ascii_case("/restart") {
            _ = tele.send_message(chat_id, "Ok, I am starting a new conversation.");
            set(&chat_id.to_string(), json!(true), None);
            log::info!("Restarted converstion for {}", chat_id);
        } else {
            let placeholder = tele
                .reply_to_message(&msg, &placeholder_text)
                .expect("Error occurs when sending Message to Telegram");

            let restart = match get(&chat_id.to_string()) {
                Some(v) => v.as_bool().unwrap_or_default(),
                None => false,
            };
            if restart {
                log::info!("Detected restart = true");
                set(&chat_id.to_string(), json!(false), None);
                co.restart = true;
            }

            match openai
                .chat_completion(&chat_id.to_string(), &text, &co)
                .await
            {
                Ok(r) => {
                    _ = tele.edit_message_text(chat_id, placeholder.id, r.choice);
                }
                Err(e) => {
                    _ = tele.edit_message_text(
                        chat_id,
                        placeholder.id,
                        "Sorry, an error has occured. Please try again later!",
                    );
                    log::error!("OpenAI returns error: {}", e);
                }
            }
        }
    }
}
