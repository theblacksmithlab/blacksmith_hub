use std::env;
use std::path::PathBuf;
use std::str::FromStr;
use strum_macros::Display;

#[derive(Debug, Clone, PartialEq, Display)]
pub enum AppName {
    ProbiotBot,
    TheViperRoom,
    TheViperRoomBot,
    W3AWeb,
    BlacksmithWeb,
    GrootBot,
    UniframeStudio,
    AgentDavon,
}

impl AppName {
    pub fn as_str(&self) -> &str {
        match self {
            AppName::ProbiotBot => "probiot_bot",
            AppName::TheViperRoom => "the_viper_room",
            AppName::TheViperRoomBot => "the_viper_room_bot",
            AppName::W3AWeb => "w3a_web",
            AppName::BlacksmithWeb => "bls_web",
            AppName::GrootBot => "groot_bot",
            AppName::UniframeStudio => "uniframe_studio",
            AppName::AgentDavon => "agent_davon",
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
            "w3a_web" => Ok(AppName::W3AWeb),
            "blacksmith_web" | "bls_web" => Ok(AppName::BlacksmithWeb),
            "groot_bot" => Ok(AppName::GrootBot),
            "uniframe_studio" => Ok(AppName::UniframeStudio),
            "agent_davon" => Ok(AppName::AgentDavon),
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
