use serde::{Deserialize, Serialize};
use strum_macros::Display;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Display)]
#[serde(rename_all = "lowercase")]
pub enum QueryType {
    Common,
    Special,
    Invalid,
    Support,
}

impl QueryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            QueryType::Common => "common",
            QueryType::Special => "special",
            QueryType::Invalid => "invalid",
            QueryType::Support => "support",
        }
    }
}

impl AsRef<str> for QueryType {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
