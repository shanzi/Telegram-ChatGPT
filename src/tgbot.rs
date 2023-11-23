use std::fmt;

use crate::tgext::TgExt;
use anyhow::bail;
use flowsnet_platform_sdk::logger;
use openai_flows::{
    chat::{ChatModel, ChatOptions},
    OpenAIFlows,
};
use serde::{Deserialize, Serialize};
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

const NIHONGO_TRANSLATE_PROMPT: &str = r#"
You are now helping the user to learn Japanese.
You should act as a translate machine and please translate everything the user sent to you into Japanese direcly.
You can provide explanation on keywords in the Japanese translation provide pronunciation in hiragana.
If the user sent you Japanese, you should translate them into English and correct the user if there is any obvious mistake.
When providing pronunciation of Japanese, please use hiragana or katakana instead of romaji.
"#;

const NIHONGO_EXPLAIN_PROMPT: &str = r#"
You are now helping the user to learn Japanese.
If the user sent you a piece of text in Japanese, you should explain the grammar and keywords.
You can explain by break down the sentences and provide pronounce annotation in hiragana.
If the user ask you a question in English, you should translate it into Japanese and explain your translation.
You can also answer the user's chat from your own knowledge.
You are encouraged to provide background information of a famous historical place.
If you feel there is a better way to say something, feel free to correct the user.
"#;

const NIHONGO_MOCK_SCENE_PROMPT: &str = r#"
You are now helping the users to learn Japanese.
You should always speak Japanese in the conversation.
You are now in mock conversation mode, in this mode, you should act as a role in a conversation scene.
When the user send you a message, you should reply based on your role.
If what the user has sent you is obviously not correct in terms of grammar or usage of words, you can first correct the users and provide an explanation.
If you are replying to the user with some rarely used words, please provide the translation of them after the reply.
If the user send you a message in English, please tell the user how to express the same meaning in Japanese before replying under your role.
"#;

const NIHONGO_MOCK_SCENE_CAFE_PROMPT: &str = r#"
Your role is defined as follow:
You are a waiter in a cafe.
The cafe provide all kinds of coffee from espresso to pour over.
The cafe also sell baked whole beans.
You should help the user to order a cup of coffee.
When you are using any Japanese words about origins of coffee, flaver, and other technique about coffee, please emphasize the word with markdown.
"#;

const NIHONGO_MOCK_SCENE_RESTAURANT_PROMPT: &str = r#"
Your role is defined as follow:
You are a waiter in a restaurant.
You are helping the user to order a dish.
You can recommend some dishes to the user.
When you are using any Japanese words about food, vegetables, fruit, dishes, spice, flavor and drinks, please semphasize the word with markdown.
"#;

const NIHONGO_MOCK_SCENE_CLOTHES_SHOP_PROMPT: &str = r#"
Your role is defined as follow:
You are a shopping guide in a clothes shop.
The clothes shop sells all kinds of clothes and shoes.
You can guide the user per your understanding of the fashion in Japan.
You can pretend the shop has a fitting room and let the user try the clothes or shoes.
When you are using any Japanese words about clothes, style, and other fashion related words, please emphasize the word with markdown.
"#;

const NIHONGO_MOCK_SCENE_STREET_PROMPT: &str = r#"
Your role is defined as follow:
You are a passers-by on the street who have just met the user.
You want to help the user know about the city, street and nearby.
You can first ask the user about where the user is at and where the user want to go.
When you are using any Japanese words about location, direction and other motion related words, please emphasize the word with markdown. 
"#;

const NIHONGO_MOCK_SCENE_SMALL_TALK_PROMPT: &str = r#"
Your role is defined as follow:
You are a passers-by who have just met the user.
You and the user are going to have a random small talk.
The topic can vary from weather to habbit.
You can start by picking a random topic.
"#;

#[derive(Clone, Serialize, Deserialize)]
enum TgBotPrompt {
    Default,
    NihongoTranslate,
    NihongoExplain,
    NihongoSceneMockCafe,
    NihongoSceneMockRestaurant,
    NihongoSceneMockClothesShop,
    NihongoSceneMockStreet,
    NihongoSceneMockSmallTalk,
}

impl TgBotPrompt {
    fn id(&self) -> &'static str {
        match self {
            TgBotPrompt::NihongoTranslate => "nihongo-translate",
            TgBotPrompt::NihongoExplain => "nihongo-explain",
            TgBotPrompt::NihongoSceneMockCafe => "nihongo-scene-mock-cafe",
            TgBotPrompt::NihongoSceneMockRestaurant => "nihongo-scene-mock-restaurant",
            TgBotPrompt::NihongoSceneMockClothesShop => "nihongo-scene-mock-clothes-shop",
            TgBotPrompt::NihongoSceneMockStreet => "nihongo-scene-mock-street",
            TgBotPrompt::NihongoSceneMockSmallTalk => "nihongo-scene-mock-small-talk",
            _ => "default",
        }
    }

    fn prompt(&self) -> String {
        match self {
            TgBotPrompt::NihongoTranslate => [DEFAULT_PROMPT, NIHONGO_TRANSLATE_PROMPT].join("\n"),
            TgBotPrompt::NihongoExplain => [DEFAULT_PROMPT, NIHONGO_EXPLAIN_PROMPT].join("\n"),
            TgBotPrompt::NihongoSceneMockCafe => [
                DEFAULT_PROMPT,
                NIHONGO_MOCK_SCENE_PROMPT,
                NIHONGO_MOCK_SCENE_CAFE_PROMPT,
            ]
            .join("\n"),
            TgBotPrompt::NihongoSceneMockRestaurant => [
                DEFAULT_PROMPT,
                NIHONGO_MOCK_SCENE_PROMPT,
                NIHONGO_MOCK_SCENE_RESTAURANT_PROMPT,
            ]
            .join("\n"),
            TgBotPrompt::NihongoSceneMockClothesShop => [
                DEFAULT_PROMPT,
                NIHONGO_MOCK_SCENE_PROMPT,
                NIHONGO_MOCK_SCENE_CLOTHES_SHOP_PROMPT,
            ]
            .join("\n"),
            TgBotPrompt::NihongoSceneMockStreet => [
                DEFAULT_PROMPT,
                NIHONGO_MOCK_SCENE_PROMPT,
                NIHONGO_MOCK_SCENE_STREET_PROMPT,
            ]
            .join("\n"),
            TgBotPrompt::NihongoSceneMockSmallTalk => [
                DEFAULT_PROMPT,
                NIHONGO_MOCK_SCENE_PROMPT,
                NIHONGO_MOCK_SCENE_SMALL_TALK_PROMPT,
            ]
            .join("\n"),
            _ => DEFAULT_PROMPT.to_owned(),
        }
    }
}

impl From<&str> for TgBotPrompt {
    fn from(value: &str) -> Self {
        match value {
            "nihongo-translate" => TgBotPrompt::NihongoTranslate,
            "nihongo-explain" => TgBotPrompt::NihongoExplain,
            "nihongo-scene-mock-cafe" => TgBotPrompt::NihongoSceneMockCafe,
            "nihongo-scene-mock-restaurant" => TgBotPrompt::NihongoSceneMockRestaurant,
            "nihongo-scene-mock-clothes-shop" => TgBotPrompt::NihongoSceneMockClothesShop,
            "nihongo-scene-mock-street" => TgBotPrompt::NihongoSceneMockStreet,
            "nihongo-scene-mock-small-talk" => TgBotPrompt::NihongoSceneMockSmallTalk,
            _ => TgBotPrompt::Default,
        }
    }
}

#[derive(Clone)]
enum TgBotCommand {
    Ask,
    Nihongo,
    Settings,
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
            TgBotInlineButton::NihongoSceneMock => "NihongoSceneMock",
            TgBotInlineButton::NihongoSceneMockRestaurant => "NihongoSceneMockRestaurant",
            TgBotInlineButton::NihongoSceneMockCafe => "NihongoSceneMockCafe",
            TgBotInlineButton::NihongoSceneMockClothesShop => "NihongoSceneMockClothesShop",
            TgBotInlineButton::NihongoSceneMockStreet => "NihongoSceneMockStreet",
            TgBotInlineButton::NihongoSceneMockSmallTalk => "NihongoSceneMockSmallTalk",
            TgBotInlineButton::NihongoSceneMockGoBack => "NihongoSceneMockGoBack",
            // settings
            TgBotInlineButton::SettingsLMGPT35Turbo => "SettingsLMGPT35Turbo",
            TgBotInlineButton::SettingsLMGPT35Turbo16K => "SettingsLMGPT35Turbo16K",
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
            "NihongoSceneMock" => Ok(Self::NihongoSceneMock),
            "NihongoSceneMockCafe" => Ok(Self::NihongoSceneMockCafe),
            "NihongoSceneMockRestaurant" => Ok(Self::NihongoSceneMockRestaurant),
            "NihongoSceneMockClothesShop" => Ok(Self::NihongoSceneMockClothesShop),
            "NihongoSceneMockStreet" => Ok(Self::NihongoSceneMockStreet),
            "NihongoSceneMockSmallTalk" => Ok(Self::NihongoSceneMockSmallTalk),
            "NihongoSceneMockGoBack" => Ok(Self::NihongoSceneMockGoBack),
            // settings
            "SettingsLMGPT35Turbo" => Ok(Self::SettingsLMGPT35Turbo),
            "SettingsLMGPT35Turbo16K" => Ok(Self::SettingsLMGPT35Turbo16K),
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
        vec![
            TgBotCommand::Ask,
            TgBotCommand::Nihongo,
            TgBotCommand::Settings,
            TgBotCommand::Help,
        ]
    }
}

impl From<TgBotCommand> for BotCommand {
    fn from(val: TgBotCommand) -> Self {
        match val {
            TgBotCommand::Ask => BotCommand::new("ask", "ask any questions"),
            TgBotCommand::Nihongo => {
                BotCommand::new("nihongo", "learn japanese by sentences and questions")
            }
            TgBotCommand::Settings => BotCommand::new("settings", "adjust settings of the bot"),
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

#[derive(Clone, Serialize, Deserialize)]
struct TgBotContext {
    id: String,
    prompt: TgBotPrompt,
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

    pub fn set_bot_commands(&self) -> anyhow::Result<bool> {
        self.tg.set_my_commands(TgBotCommand::root_commands())
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

            let root = TgBot::get_root_message(msg);
            let root_ptr = TgBot::get_message_ptr(root);
            let chat_ctx = store_flows::get(&root_ptr)
                .and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or(TgBotContext {
                    id: root_ptr,
                    prompt: TgBotPrompt::Default,
                });

            TgBot::set_message_context(&placeholder, &serde_json::to_value(&chat_ctx).unwrap());

            let chat_ptr = chat_ctx.id.as_str();
            let chat_ctx_id = format!("ctx--{}", chat_ptr);
            log::info!(
                "placeholder: {} root: {}, chat_ctx_id: {}, chat_prompt: {}, chat_ctx: {}",
                placeholder.id,
                root.id,
                chat_ctx_id,
                chat_ctx.prompt.id(),
                store_flows::get(&chat_ctx_id).unwrap_or("None".into())
            );

            let mut copt = ChatOptions::default();

            let lm = store_flows::get("settings.language.model")
                .unwrap_or(serde_json::Value::String("gpt4".to_string()));

            match lm.as_str() {
                Some("gpt4") => copt.model = ChatModel::GPT4,
                Some("gpt3.5-turbo") => copt.model = ChatModel::GPT35Turbo,
                _ => copt.model = ChatModel::GPT35Turbo16K,
            }

            let prompt = chat_ctx.prompt.prompt();
            copt.restart = false;
            copt.system_prompt = Some(prompt.as_str());

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
                TgBotInlineButton::NihongoTranslate.into(),
                TgBotInlineButton::NihongoExplain.into(),
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
            TgBotInlineButton::NihongoTranslate => self
                .tg
                .send_message_ext(
                    msg.chat.id,
                    Some(&msg.id),
                    "日本語に翻訳しています",
                    Some(ReplyMarkup::ForceReply(ForceReply::default())),
                )
                .map(|msg| self.init_message_prompt(msg, TgBotPrompt::NihongoTranslate)),
            TgBotInlineButton::NihongoExplain => self
                .tg
                .send_message_ext(
                    msg.chat.id,
                    Some(&msg.id),
                    "日本語の言葉を説明しています",
                    Some(ReplyMarkup::ForceReply(ForceReply::default())),
                )
                .map(|msg| self.init_message_prompt(msg, TgBotPrompt::NihongoExplain)),
            TgBotInlineButton::NihongoSceneMock => self.tg.send_message_ext(
                msg.chat.id,
                Some(&msg.id),
                "モック会話しています、何な場面をほしいですか",
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
            TgBotInlineButton::NihongoSceneMockCafe => self
                .tg
                .send_message_ext(
                    msg.chat.id,
                    Some(&msg.id),
                    "カフェでいます",
                    Some(ReplyMarkup::ForceReply(ForceReply::default())),
                )
                .map(|msg| self.init_message_prompt(msg, TgBotPrompt::NihongoSceneMockCafe)),
            TgBotInlineButton::NihongoSceneMockRestaurant => self
                .tg
                .send_message_ext(
                    msg.chat.id,
                    Some(&msg.id),
                    "レストランでいます",
                    Some(ReplyMarkup::ForceReply(ForceReply::default())),
                )
                .map(|msg| self.init_message_prompt(msg, TgBotPrompt::NihongoSceneMockRestaurant)),
            TgBotInlineButton::NihongoSceneMockClothesShop => self
                .tg
                .send_message_ext(
                    msg.chat.id,
                    Some(&msg.id),
                    "服屋でいます",
                    Some(ReplyMarkup::ForceReply(ForceReply::default())),
                )
                .map(|msg| self.init_message_prompt(msg, TgBotPrompt::NihongoSceneMockClothesShop)),
            TgBotInlineButton::NihongoSceneMockStreet => self
                .tg
                .send_message_ext(
                    msg.chat.id,
                    Some(&msg.id),
                    "街でいます",
                    Some(ReplyMarkup::ForceReply(ForceReply::default())),
                )
                .map(|msg| self.init_message_prompt(msg, TgBotPrompt::NihongoSceneMockStreet)),
            TgBotInlineButton::NihongoSceneMockSmallTalk => self
                .tg
                .send_message_ext(
                    msg.chat.id,
                    Some(&msg.id),
                    "雑談しています",
                    Some(ReplyMarkup::ForceReply(ForceReply::default())),
                )
                .map(|msg| self.init_message_prompt(msg, TgBotPrompt::NihongoSceneMockSmallTalk)),
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

    fn init_message_prompt(&self, msg: Message, prompt: TgBotPrompt) -> Message {
        let ctx = serde_json::to_value(TgBotContext {
            id: Self::get_message_ptr(&msg),
            prompt,
        })
        .unwrap();
        Self::set_message_context(&msg, &ctx);
        msg
    }
}
