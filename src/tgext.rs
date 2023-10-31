use tg_flows::{BotCommand, ChatId, Message, ReplyMarkup, Telegram};

pub trait TgExt {
    fn reply_to_message<T>(&self, msg: &Message, text: T) -> anyhow::Result<Message>
    where
        T: Into<String>;

    fn set_my_commands<T>(&self, cmds: T) -> anyhow::Result<Message>
    where
        T: IntoIterator,
        T::Item: Into<BotCommand>;

    fn send_message_with_reply_markup<T>(
        &self,
        chat_id: ChatId,
        text: T,
        reply_markup: ReplyMarkup,
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

    fn set_my_commands<T>(&self, cmds: T) -> anyhow::Result<Message>
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

    fn send_message_with_reply_markup<T>(
        &self,
        chat_id: ChatId,
        text: T,
        reply_markup: ReplyMarkup,
    ) -> anyhow::Result<Message>
    where
        T: Into<String>,
    {
        let body = serde_json::json!({
            "chat_id": chat_id,
            "text": text.into(),
            "reply_markup": reply_markup,
        });
        self.request(tg_flows::Method::SetMyCommands, body.to_string().as_bytes())
    }
}
