use teloxide::macros::BotCommands;

#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase")]
pub enum TheViperRoomBotCommands {
    Start,
    Podcast,
    Test,
    Schedule,
    Stop,
    Menu
}
