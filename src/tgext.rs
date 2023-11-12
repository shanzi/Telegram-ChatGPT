use crate::markdown::escape_markdown;
use tg_flows::{BotCommand, ChatId, Message, MessageId, ReplyMarkup, Telegram};

pub trait TgExt {
    fn reply_to_message<T>(&self, msg: &Message, text: T) -> anyhow::Result<Message>
    where
        T: Into<String>;

    fn set_my_commands<T>(&self, cmds: T) -> anyhow::Result<bool>
    where
        T: IntoIterator,
        T::Item: Into<BotCommand>;

    fn send_message_ext<T>(
        &self,
        chat_id: ChatId,
        reploy_to: Option<&MessageId>,
        text: T,
        reply_markup: Option<ReplyMarkup>,
    ) -> anyhow::Result<Message>
    where
        T: Into<String>;

    fn edit_message_text_ext<T>(
        &self,
        chat_id: ChatId,
        message_id: MessageId,
        text: T,
        reply_markup: Option<ReplyMarkup>,
    ) -> anyhow::Result<Message>
    where
        T: Into<String>;
}

impl TgExt for Telegram {
    fn reply_to_message<T>(&self, msg: &Message, text: T) -> anyhow::Result<Message>
    where
        T: Into<String>,
    {
        let text: String = text.into();
        let body = serde_json::json!({
            "chat_id": msg.chat.id,
            "reply_to_message_id": msg.id.0,
            "text": text,
        });
        log::info!("reply message: {}", body);
        self.request(tg_flows::Method::SendMessage, body.to_string().as_bytes())
    }

    fn set_my_commands<T>(&self, cmds: T) -> anyhow::Result<bool>
    where
        T: IntoIterator,
        T::Item: Into<BotCommand>,
    {
        let commands: Vec<BotCommand> = cmds.into_iter().map(|cmd| cmd.into()).collect();
        let body = serde_json::json!({
            "commands": commands,
        });
        log::info!("set bot command: {}", body);
        self.request(tg_flows::Method::SetMyCommands, body.to_string().as_bytes())
    }

    fn send_message_ext<T>(
        &self,
        chat_id: ChatId,
        reply_to: Option<&MessageId>,
        text: T,
        reply_markup: Option<ReplyMarkup>,
    ) -> anyhow::Result<Message>
    where
        T: Into<String>,
    {
        let markup_value = match reply_markup {
            Some(markup) => serde_json::to_value(markup)?,
            _ => serde_json::Value::Null,
        };
        let message_id = match reply_to {
            Some(id) => serde_json::to_value(id)?,
            _ => serde_json::Value::Null,
        };
        let body = serde_json::json!({
            "chat_id": chat_id,
            "reply_to_message_id": message_id,
            "parse_mode": "MarkdownV2",
            "text": escape_markdown(text.into())?,
            "reply_markup": markup_value,
        });
        log::info!("send message ext: {}", &body);
        self.request(tg_flows::Method::SendMessage, body.to_string().as_bytes())
    }

    fn edit_message_text_ext<T>(
        &self,
        chat_id: ChatId,
        message_id: MessageId,
        text: T,
        reply_markup: Option<ReplyMarkup>,
    ) -> anyhow::Result<Message>
    where
        T: Into<String>,
    {
        let text: String = text.into();
        let body = match reply_markup {
            Some(markup) => serde_json::json!({
                "chat_id": chat_id,
                "message_id": message_id.0,
                "parse_mode": "MarkdownV2",
                "text": escape_markdown(&text)?,
                "reply_markup": serde_json::to_value(markup)?,
            }),
            _ => serde_json::json!({
                "chat_id": chat_id,
                "message_id": message_id.0,
                "parse_mode": "MarkdownV2",
                "text": escape_markdown(&text)?,
            }),
        };
        match self.request(
            tg_flows::Method::EditMessageText,
            body.to_string().as_bytes(),
        ) {
            Err(_) => {
                log::error!("wrong escape_markdown: {}", body);
                self.edit_message_text(chat_id, message_id, text)
            }
            res => res,
        }
    }
}
