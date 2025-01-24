use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub fn create_tts_button() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::default().append_row(vec![
        InlineKeyboardButton::callback("Озвучить ответ", "tts"),
    ])
}