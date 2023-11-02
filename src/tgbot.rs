use std::fmt;

use crate::tgext::TgExt;
use flowsnet_platform_sdk::logger;
use openai_flows::{
    chat::{ChatModel, ChatOptions},
    OpenAIFlows,
};
use tg_flows::{BotCommand, ChatId, ForceReply, Message, Telegram, Update, UpdateKind};

const DEFAULT_PROMPT: &str = r#"
Your name is "Cheese" and you are working as a jotting pal to help on Telegram. 
You can answer questions, help clients learn japanese and show a help message.
your creator is Chase Zhang, you are based on OpenAI's ChatGPT.
You should double check the fact of your answer carefully before replying a message
and make sure it is acurate.
You should format your answers into markdown format if necessary.
If you answer includes codeblocks, please make sure you will specify the name
of the programming language with proper syntax in markdown format.
"#;

#[derive(Clone)]
enum TgBotCommand {
    Ask,
    Nihongo,
    Help,
}

impl TgBotCommand {
    fn root_commands() -> Vec<TgBotCommand> {
        vec![TgBotCommand::Ask, TgBotCommand::Nihongo, TgBotCommand::Help]
    }
}

impl From<TgBotCommand> for BotCommand {
    fn from(val: TgBotCommand) -> Self {
        match val {
            TgBotCommand::Ask => BotCommand::new("ask", "ask any questions"),
            TgBotCommand::Nihongo => {
                BotCommand::new("nihongo", "learn japanese by sentences and questions")
            }
            TgBotCommand::Help => BotCommand::new("help", "show help messages"),
        }
    }
}

impl fmt::Display for TgBotCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let cmd: BotCommand = self.clone().into();
        write!(f, "/{} {}", cmd.command, cmd.description)
    }
}

pub struct TgBot {
    tg: Telegram,
    openai: OpenAIFlows,
    help_msg: String,
}

impl Default for TgBot {
    fn default() -> Self {
        let telegram_token = std::env::var("telegram_token").unwrap();
        let mut openai = OpenAIFlows::new();
        openai.set_retry_times(3);

        Self {
            tg: Telegram::new(telegram_token),
            openai,
            help_msg: "Hi! I'm you jotting pal.".into(),
        }
    }
}

impl TgBot {
    pub fn set_root_commands(&self) -> anyhow::Result<tg_flows::Message> {
        let bot_cmds: Vec<BotCommand> = TgBotCommand::root_commands()
            .into_iter()
            .map(TgBotCommand::into)
            .collect();
        self.tg.set_my_commands(bot_cmds)
    }

    pub async fn handle_update(&self, update: Update) -> anyhow::Result<()> {
        logger::init();
        if let UpdateKind::Message(msg) = update.kind {
            let chat_id = msg.chat.id;
            match msg.text() {
                Some(_) if msg.reply_to_message().is_some() => self.handle_ask(&msg).await,
                Some(text) if text.starts_with("/ask") => self.handle_ask(&msg).await,
                Some(text) if text.starts_with("/nihongo") => self.handle_nihongo(&msg),
                _ => self.show_help_message(chat_id),
            }
            .map(|_| ())
        } else {
            Ok(())
        }
    }

    fn set_typing(&self, chat_id: ChatId) -> anyhow::Result<tg_flows::Message> {
        self.tg.send_chat_action(chat_id, "typing".to_string())
    }

    fn show_help_message(&self, chat_id: ChatId) -> anyhow::Result<tg_flows::Message> {
        self.tg.send_message(
            chat_id,
            format!(
                "{} Available commands:\n{}",
                self.help_msg,
                TgBotCommand::root_commands()
                    .iter()
                    .map(|cmd| cmd.to_string())
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
        )
    }

    async fn handle_ask(&self, msg: &Message) -> anyhow::Result<tg_flows::Message> {
        let text = msg.text().unwrap();
        log::info!("handle ask: {}", text);

        if msg.reply_to_message().is_some() || text.starts_with("/ask ") {
            let question = text.strip_prefix("/ask ").unwrap_or(text);

            log::info!("reply to message: {}", msg.id);
            let placeholder = self.tg.reply_to_message(msg, "typing...")?;

            log::info!("set to typing, chat id: {}", msg.chat.id);
            // ignore callback result
            let _ = self.set_typing(msg.chat.id);

            let mut copt = ChatOptions::default();

            copt.model = ChatModel::GPT4;
            copt.restart = false;
            copt.system_prompt = Some(DEFAULT_PROMPT);

            let root = TgBot::get_root_message(msg);
            let root_ptr = TgBot::get_message_ptr(root);
            let chat_ctx =
                store_flows::get(&root_ptr).unwrap_or(serde_json::Value::String(root_ptr));

            TgBot::set_message_context(&placeholder, &chat_ctx);

            let chat_ptr = chat_ctx.as_str().unwrap();
            let chat_ctx_id = format!("ctx--{}", chat_ptr);
            log::info!(
                "placeholder: {} root: {}, chat_ctx_id: {}, chat_ctx: {}",
                placeholder.id,
                root.id,
                chat_ctx_id,
                store_flows::get(&chat_ctx_id).unwrap_or("None".into())
            );

            match self
                .openai
                .chat_completion(&chat_ctx_id, question, &copt)
                .await
            {
                Ok(resp) => self
                    .tg
                    .edit_message_markdown(msg.chat.id, placeholder.id, resp.choice),
                Err(_) => self.tg.edit_message_text(
                    msg.chat.id,
                    placeholder.id,
                    "Sorry, an error has occured. Please try again later.",
                ),
            }
        } else {
            log::info!("force reply: {}", msg.chat.id);
            self.tg.send_message_ext(
                msg.chat.id,
                None,
                "How can I help you?",
                Some(tg_flows::ReplyMarkup::ForceReply(ForceReply::new())),
            )
        }
    }

    fn handle_nihongo(&self, msg: &Message) -> anyhow::Result<tg_flows::Message> {
        let mut keyboard = tg_flows::InlineKeyboardMarkup::default();
        keyboard = keyboard.append_row(vec![
            tg_flows::InlineKeyboardButton::new(
                "翻訳",
                tg_flows::InlineKeyboardButtonKind::CallbackData("translate".into()),
            ),
            tg_flows::InlineKeyboardButton::new(
                "説明",
                tg_flows::InlineKeyboardButtonKind::CallbackData("explain".into()),
            ),
        ]);
        keyboard = keyboard.append_row(vec![tg_flows::InlineKeyboardButton::new(
            "戻る",
            tg_flows::InlineKeyboardButtonKind::CallbackData("cancel".into()),
        )]);
        self.tg.send_message_ext(
            msg.chat.id,
            Some(&msg.id),
            "どのようにおてつだいでくますか？",
            Some(tg_flows::ReplyMarkup::InlineKeyboard(keyboard)),
        )
    }

    fn get_message_ptr(msg: &Message) -> String {
        format!("ptr--{}-{}", msg.chat.id, msg.id)
    }

    fn get_root_message(msg: &Message) -> &Message {
        let mut root = msg;
        while root.reply_to_message().is_some() {
            root = root.reply_to_message().unwrap();
        }
        root
    }

    fn set_message_context(msg: &Message, ctx: &serde_json::Value) {
        let mut root = msg;
        loop {
            let key = &TgBot::get_message_ptr(root);
            log::info!("set context, key: {}, value: {}", key, ctx);
            store_flows::set(key, ctx.clone(), None);
            if let Some(reply) = root.reply_to_message() {
                root = reply;
            } else {
                return;
            }
        }
    }
}
