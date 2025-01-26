#[derive(Debug, Clone, PartialEq)]
pub enum AppName {
    Probiot,
    TheViperRoom,
    TheViperRoomBot,
    RequestApp,
    RequestAppBot,
    TesterBot,
}

impl AppName {
    pub fn as_str(&self) -> &str {
        match self {
            AppName::Probiot => "probiot",
            AppName::TheViperRoom => "the_viper_room",
            AppName::TheViperRoomBot => "the_viper_room_bot",
            AppName::RequestApp => "request_app",
            AppName::RequestAppBot => "request_app_bot",
            AppName::TesterBot => "tester_bot",
        }
    }
}
