#[derive(Debug, Clone, PartialEq)]
pub enum AppName {
    ProbiotBot,
    TheViperRoom,
    TheViperRoomBot,
    RequestApp,
    RequestAppBot,
    TesterBot,
    W3ABot,
}

impl AppName {
    pub fn as_str(&self) -> &str {
        match self {
            AppName::ProbiotBot => "probiot_bot",
            AppName::TheViperRoom => "the_viper_room",
            AppName::TheViperRoomBot => "the_viper_room_bot",
            AppName::RequestApp => "request_app",
            AppName::RequestAppBot => "request_app_bot",
            AppName::TesterBot => "tester_bot",
            AppName::W3ABot => "w3a_bot",
        }
    }
}
