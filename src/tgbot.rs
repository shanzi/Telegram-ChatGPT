use std::fmt;

use crate::tgext::TgExt;
use anyhow::bail;
use flowsnet_platform_sdk::logger;
use openai_flows::{
    chat::{ChatModel, ChatOptions},
    OpenAIFlows,
};
use tg_flows::{
    BotCommand, CallbackQuery, ChatId, ForceReply, InlineKeyboardButton, InlineKeyboardMarkup,
    Message, ReplyMarkup, Telegram, Update, UpdateKind,
};

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

#[derive(Clone, Debug)]
enum TgBotInlineButton {
    // nihongo
    NihongoTranslate,
    NihongoExplain,
    NihongoSceneMock,
    NihongoSceneMockRestaurant,
    NihongoSceneMockCafe,
    NihongoSceneMockClothesShop,
    NihongoSceneMockStreet,
    NihongoSceneMockSmallTalk,
    NihongoSceneMockGoBack,
    // settings
    SettingsLMGPT35Turbo,
    SettingsLMGPT35Turbo16K,
    SettingsLMGPT4,
}

impl TgBotInlineButton {
    fn id(&self) -> String {
        // nihongo
        match self {
            TgBotInlineButton::NihongoTranslate => "NihongoTranslate",
            TgBotInlineButton::NihongoExplain => "NihongoExplain",
            TgBotInlineButton::NihongoSceneMock => "NihongoExplain",
            TgBotInlineButton::NihongoSceneMockRestaurant => "NihongoSceneMock",
            TgBotInlineButton::NihongoSceneMockCafe => "NihongoSceneMockRestaurant",
            TgBotInlineButton::NihongoSceneMockClothesShop => "NihongoSceneMockCafe",
            TgBotInlineButton::NihongoSceneMockStreet => "NihongoSceneMockClothesShop",
            TgBotInlineButton::NihongoSceneMockSmallTalk => "NihongoSceneMockStreet",
            TgBotInlineButton::NihongoSceneMockGoBack => "NihongoSceneMockGoBack",
            // settings
            TgBotInlineButton::SettingsLMGPT35Turbo => "NihongoSceneMockSmallTalk",
            TgBotInlineButton::SettingsLMGPT35Turbo16K => "SettingsLMGPT35Turbo",
            TgBotInlineButton::SettingsLMGPT4 => "SettingsLMGPT4",
        }
        .to_owned()
    }

    fn title(&self) -> String {
        match self {
            TgBotInlineButton::NihongoTranslate => "翻訳",
            TgBotInlineButton::NihongoExplain => "説明",
            TgBotInlineButton::NihongoSceneMock => "模擬会話",
            TgBotInlineButton::NihongoSceneMockRestaurant => "レストラン",
            TgBotInlineButton::NihongoSceneMockCafe => "カフェ",
            TgBotInlineButton::NihongoSceneMockClothesShop => "服屋",
            TgBotInlineButton::NihongoSceneMockStreet => "街",
            TgBotInlineButton::NihongoSceneMockSmallTalk => "自由",
            TgBotInlineButton::NihongoSceneMockGoBack => "戻る",
            TgBotInlineButton::SettingsLMGPT35Turbo => "gpt3.5-turbo",
            TgBotInlineButton::SettingsLMGPT35Turbo16K => "gpt3.5-turbo-16k",
            TgBotInlineButton::SettingsLMGPT4 => "gpt4",
        }
        .to_string()
    }
}

impl TryFrom<&str> for TgBotInlineButton {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> anyhow::Result<Self> {
        match value {
            // nihongo
            "NihongoTranslate" => Ok(Self::NihongoTranslate),
            "NihongoExplain" => Ok(Self::NihongoExplain),
            "NihongoSceneMock" => Ok(Self::NihongoExplain),
            "NihongoSceneMockRestaurant" => Ok(Self::NihongoSceneMock),
            "NihongoSceneMockCafe" => Ok(Self::NihongoSceneMockRestaurant),
            "NihongoSceneMockClothesShop" => Ok(Self::NihongoSceneMockCafe),
            "NihongoSceneMockStreet" => Ok(Self::NihongoSceneMockClothesShop),
            "NihongoSceneMockSmallTalk" => Ok(Self::NihongoSceneMockStreet),
            "NihongoSceneMockGoBack" => Ok(Self::NihongoSceneMockGoBack),
            // settings
            "SettingsLMGPT35Turbo" => Ok(Self::NihongoSceneMockSmallTalk),
            "SettingsLMGPT35Turbo16K" => Ok(Self::SettingsLMGPT35Turbo),
            "SettingsLMGPT4" => Ok(Self::SettingsLMGPT4),
            // unknown
            unknown => anyhow::bail!("unknown id: {}", unknown),
        }
    }
}

impl From<TgBotInlineButton> for InlineKeyboardButton {
    fn from(tg_kb: TgBotInlineButton) -> Self {
        InlineKeyboardButton::new(
            tg_kb.title(),
            tg_flows::InlineKeyboardButtonKind::CallbackData(tg_kb.id()),
        )
    }
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
    pub async fn handle_update(&self, update: Update) -> anyhow::Result<()> {
        logger::init();
        match update.kind {
            UpdateKind::Message(msg) => {
                let chat_id = msg.chat.id;
                match msg.text() {
                    Some(_) if msg.reply_to_message().is_some() => self.handle_ask(&msg).await,
                    Some(text) if text.starts_with("/ask") => self.handle_ask(&msg).await,
                    Some(text) if text.starts_with("/nihongo") => self.handle_nihongo(&msg, false),
                    Some(text) if text.starts_with("/settings") => self.handle_settings(&msg),
                    _ => self.show_help_message(chat_id),
                }
                .map(|_| ())
            }
            UpdateKind::CallbackQuery(cq) => self.handle_callback_query(&cq).map(|_| ()),
            _ => Ok(()),
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

            let lm = store_flows::get("settings.language.model")
                .unwrap_or(serde_json::Value::String("gpt4".to_string()));

            match lm.as_str() {
                Some("gpt4") => copt.model = ChatModel::GPT4,
                Some("gpt3.5-turbo") => copt.model = ChatModel::GPT35Turbo,
                _ => copt.model = ChatModel::GPT35Turbo16K,
            }
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
                Ok(resp) => {
                    self.tg
                        .edit_message_text_ext(msg.chat.id, placeholder.id, resp.choice, None)
                }
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

    fn handle_nihongo(&self, msg: &Message, edit: bool) -> anyhow::Result<tg_flows::Message> {
        let keyboard = tg_flows::InlineKeyboardMarkup::default()
            .append_row(vec![
                TgBotInlineButton::NihongoExplain.into(),
                TgBotInlineButton::NihongoTranslate.into(),
            ])
            .append_row(vec![TgBotInlineButton::NihongoSceneMock.into()]);

        if edit {
            self.tg.edit_message_text_ext(
                msg.chat.id,
                msg.id,
                "どのようにおてつだいでくますか？",
                Some(tg_flows::ReplyMarkup::InlineKeyboard(keyboard)),
            )
        } else {
            self.tg.send_message_ext(
                msg.chat.id,
                Some(&msg.id),
                "どのようにおてつだいでくますか？",
                Some(tg_flows::ReplyMarkup::InlineKeyboard(keyboard)),
            )
        }
    }

    fn handle_settings(&self, msg: &Message) -> anyhow::Result<tg_flows::Message> {
        self.tg.send_message_ext(
            msg.chat.id,
            Some(&msg.id),
            "Choose your language model.",
            Some(ReplyMarkup::InlineKeyboard(
                InlineKeyboardMarkup::default()
                    .append_row(vec![TgBotInlineButton::SettingsLMGPT35Turbo.into()])
                    .append_row(vec![TgBotInlineButton::SettingsLMGPT35Turbo16K.into()])
                    .append_row(vec![TgBotInlineButton::SettingsLMGPT4.into()]),
            )),
        )
    }

    fn handle_callback_query(&self, cq: &CallbackQuery) -> anyhow::Result<tg_flows::Message> {
        if let Some(ref data) = cq.data {
            let button: TgBotInlineButton = data.as_str().try_into()?;
            match button {
                TgBotInlineButton::NihongoTranslate
                | TgBotInlineButton::NihongoExplain
                | TgBotInlineButton::NihongoSceneMock => {
                    self.handle_nihongo_button(cq.message.as_ref().unwrap(), &button)
                }
                TgBotInlineButton::NihongoSceneMockRestaurant
                | TgBotInlineButton::NihongoSceneMockCafe
                | TgBotInlineButton::NihongoSceneMockClothesShop
                | TgBotInlineButton::NihongoSceneMockStreet
                | TgBotInlineButton::NihongoSceneMockSmallTalk
                | TgBotInlineButton::NihongoSceneMockGoBack => {
                    self.handle_nihongo_scene_mock_button(cq.message.as_ref().unwrap(), &button)
                }
                TgBotInlineButton::SettingsLMGPT35Turbo
                | TgBotInlineButton::SettingsLMGPT35Turbo16K
                | TgBotInlineButton::SettingsLMGPT4 => {
                    self.handle_settings_button(cq.message.as_ref().unwrap(), &button)
                }
            }
        } else {
            bail!("can't handle callback query without data")
        }
    }

    fn handle_nihongo_button(
        &self,
        msg: &Message,
        button: &TgBotInlineButton,
    ) -> anyhow::Result<tg_flows::Message> {
        match button {
            TgBotInlineButton::NihongoTranslate => self.tg.edit_message_text_ext(
                msg.chat.id,
                msg.id,
                "Ok, I can translate text from and to japanese for you. Please give your input.",
                Some(ReplyMarkup::ForceReply(ForceReply::default())),
            ),
            TgBotInlineButton::NihongoExplain => self.tg.edit_message_text_ext(
                msg.chat.id,
                msg.id,
                "Ok, I can explain text about japanese for you. Please give your input.",
                Some(ReplyMarkup::ForceReply(ForceReply::default())),
            ),
            TgBotInlineButton::NihongoSceneMock => self.tg.edit_message_text_ext(
                msg.chat.id,
                msg.id,
                "Ok, I can mock a conversation scene for you, choose your scene.",
                Some(ReplyMarkup::InlineKeyboard(
                    InlineKeyboardMarkup::default()
                        .append_row(vec![TgBotInlineButton::NihongoSceneMockCafe.into()])
                        .append_row(vec![TgBotInlineButton::NihongoSceneMockRestaurant.into()])
                        .append_row(vec![TgBotInlineButton::NihongoSceneMockClothesShop.into()])
                        .append_row(vec![TgBotInlineButton::NihongoSceneMockStreet.into()])
                        .append_row(vec![TgBotInlineButton::NihongoSceneMockSmallTalk.into()])
                        .append_row(vec![TgBotInlineButton::NihongoSceneMockGoBack.into()]),
                )),
            ),
            _ => bail!("wrong button"),
        }
    }

    fn handle_nihongo_scene_mock_button(
        &self,
        msg: &Message,
        button: &TgBotInlineButton,
    ) -> anyhow::Result<tg_flows::Message> {
        match button {
            TgBotInlineButton::NihongoSceneMockRestaurant => self.tg.edit_message_text_ext(
                msg.chat.id,
                msg.id,
                "You are at a restaurant!",
                Some(ReplyMarkup::ForceReply(ForceReply::default())),
            ),
            TgBotInlineButton::NihongoSceneMockCafe => self.tg.edit_message_text_ext(
                msg.chat.id,
                msg.id,
                "You are at a cafe!",
                Some(ReplyMarkup::ForceReply(ForceReply::default())),
            ),
            TgBotInlineButton::NihongoSceneMockClothesShop => self.tg.edit_message_text_ext(
                msg.chat.id,
                msg.id,
                "You are at a clothes shop!",
                Some(ReplyMarkup::ForceReply(ForceReply::default())),
            ),
            TgBotInlineButton::NihongoSceneMockStreet => self.tg.edit_message_text_ext(
                msg.chat.id,
                msg.id,
                "You are at a street!",
                Some(ReplyMarkup::ForceReply(ForceReply::default())),
            ),
            TgBotInlineButton::NihongoSceneMockSmallTalk => self.tg.edit_message_text_ext(
                msg.chat.id,
                msg.id,
                "You are having a small talk!",
                Some(ReplyMarkup::ForceReply(ForceReply::default())),
            ),
            TgBotInlineButton::NihongoSceneMockGoBack => self.handle_nihongo(msg, true),
            _ => bail!("wrong button"),
        }
    }

    fn handle_settings_button(
        &self,
        msg: &Message,
        button: &TgBotInlineButton,
    ) -> anyhow::Result<tg_flows::Message> {
        let lm = match button {
            TgBotInlineButton::SettingsLMGPT35Turbo => "gpt3.5-turbo",
            TgBotInlineButton::SettingsLMGPT35Turbo16K => "gpt3.5-turbo-16k",
            TgBotInlineButton::SettingsLMGPT4 => "gpt4",
            _ => bail!("wrong button"),
        };

        store_flows::set(
            "settings.language.model",
            serde_json::Value::String(lm.to_string()),
            None,
        );

        self.tg
            .edit_message_text(msg.chat.id, msg.id, format!("Using language model: {}", lm))
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
