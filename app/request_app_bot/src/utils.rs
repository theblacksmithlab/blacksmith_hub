use teloxide::Bot;
use teloxide::prelude::{Message, Requester};

pub(crate) async fn check_username(bot: Bot, msg: Message) -> bool {
    if let Some(_username) = msg.chat.username() {
        true
    } else {
        let error_message = "Извините, но для использования приложения необходимо установить username в Telegram.\nПожалуйста, установите username в настройках что бы получить доступ к приложению";
        let _ = bot.send_message(msg.chat.id, error_message).await;
        false
    }
}
