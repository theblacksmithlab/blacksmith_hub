use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QueryComplexity {
    Base,
    Complex,
}

impl QueryComplexity {
    pub fn as_str(&self) -> &'static str {
        match self {
            QueryComplexity::Base => "base",
            QueryComplexity::Complex => "complex",
        }
    }
}

impl AsRef<str> for QueryComplexity {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedAspects {
    pub aspects: Vec<String>,
}
