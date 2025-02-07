use teloxide::macros::BotCommands;

#[derive(BotCommands, Clone, PartialEq, Debug)]
#[command(rename_rule = "lowercase")]
pub enum GrootBotCommands {
    Start,
    About,
    Resources,
    Manual,
    Logs,
    Ask,
    Backup,
    Results,
}
