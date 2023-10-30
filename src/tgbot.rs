use std::fmt;

use crate::tgext::TgExt;
use openai_flows::OpenAIFlows;
use tg_flows::{BotCommand, ChatId, Telegram, Update, UpdateKind};

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

    pub fn handle_update(&self, update: Update) -> anyhow::Result<()> {
        // self.set_root_commands()?;
        if let UpdateKind::Message(msg) = update.kind {
            // self.set_typing(msg.chat.id)?;
            self.show_help_message(msg.chat.id).map(|_| ())
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
                "{}\nAvailable commands:\n{}",
                self.help_msg,
                TgBotCommand::root_commands()
                    .iter()
                    .map(|cmd| cmd.to_string())
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
        )
    }
}
