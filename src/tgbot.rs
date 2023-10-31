use std::fmt;

use crate::tgext::TgExt;
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
        if let UpdateKind::Message(msg) = update.kind {
            let chat_id = msg.chat.id;
            match msg.text() {
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
        let question = match msg.text() {
            Some(t) if !t.trim_start_matches("/ask ").is_empty() => {
                Some(t.trim_start_matches("/ask "))
            }
            _ => None,
        };
        if let Some(question) = question {
            let placeholder = self.tg.reply_to_message(msg, "...")?;
            self.set_typing(msg.chat.id)?;

            let mut copt = ChatOptions::default();

            copt.model = ChatModel::GPT35Turbo16K;
            copt.restart = true;
            copt.system_prompt = Some(DEFAULT_PROMPT);

            let par = format!("tp-{}-{}", msg.chat.id, msg.id);
            let cur = format!("tp-{}-{}", msg.chat.id, placeholder.id);

            let root = match store_flows::get(&par) {
                Some(p) => p.as_str().unwrap().to_owned(),
                None => par,
            };

            let chat_ctx_id = format!("ctx--{}", root);

            store_flows::set(&cur, serde_json::Value::String(root), None);

            match self
                .openai
                .chat_completion(&chat_ctx_id, question, &copt)
                .await
            {
                Ok(resp) => self
                    .tg
                    .edit_message_text(msg.chat.id, placeholder.id, resp.choice),
                Err(_) => self.tg.edit_message_text(
                    msg.chat.id,
                    placeholder.id,
                    "Sorry, an error has occured. Please try again later",
                ),
            }
        } else {
            self.tg.send_message_with_reply_markup(
                msg.chat.id,
                "May I help you?",
                tg_flows::ReplyMarkup::ForceReply(ForceReply::new()),
            )
        }
    }

    fn handle_nihongo(&self, msg: &Message) -> anyhow::Result<tg_flows::Message> {
        self.tg
            .reply_to_message(msg, "すみません、この機能はまだ使えません")
    }
}
