use std::env;
use std::path::PathBuf;
use std::str::FromStr;
use strum_macros::Display;

#[derive(Debug, Clone, PartialEq, Display)]
pub enum AppName {
    ProbiotBot,
    TheViperRoom,
    TheViperRoomBot,
    RequestApp,
    RequestAppBot,
    TesterBot,
    W3ABot,
    W3AWeb,
    BlacksmithWeb
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
            AppName::W3AWeb => "w3a_web",
            AppName::BlacksmithWeb => "blacksmith_web"
        }
    }
}

impl FromStr for AppName {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "probiot_bot" => Ok(AppName::ProbiotBot),
            "the_viper_room" => Ok(AppName::TheViperRoom),
            "the_viper_room_bot" => Ok(AppName::TheViperRoomBot),
            "request_app" => Ok(AppName::RequestApp),
            "request_app_bot" => Ok(AppName::RequestAppBot),
            "tester_bot" => Ok(AppName::TesterBot),
            "w3a_bot" => Ok(AppName::W3ABot),
            "w3a_web" => Ok(AppName::W3AWeb),
            "blacksmith_web" => Ok(AppName::BlacksmithWeb),
            _ => Err(()),
        }
    }
}

impl AppName {
    pub fn temp_dir(&self) -> PathBuf {
        let base_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        base_dir.join("tmp").join(self.as_str())
    }
}
