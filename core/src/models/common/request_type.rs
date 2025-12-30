use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RequestType {
    Common,
    Special,
    Invalid,
}

impl RequestType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RequestType::Common => "common",
            RequestType::Special => "special",
            RequestType::Invalid => "invalid",
        }
    }
}

impl AsRef<str> for RequestType {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
