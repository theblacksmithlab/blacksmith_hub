use crate::models::common::app_name::AppName;
use crate::models::tg_bot::probiot_bot::qdrant_collections::ProbiotBotCollections;
use crate::models::tg_bot::w3a_bot::qdrant_collections::W3ABotCollections;
use strum::IntoEnumIterator;

#[derive(Debug, Clone, PartialEq)]
pub enum AppsCollections {
    ProbiotBot(ProbiotBotCollections),
    W3ABot(W3ABotCollections)
}

impl AppsCollections {
    pub fn as_str(&self) -> &str {
        match self {
            AppsCollections::ProbiotBot(collection) => collection.as_str(),
            AppsCollections::W3ABot(collection) => collection.as_str(),
        }
    }

    pub fn all_collections_for_app(app_name: AppName) -> Vec<Self> {
        match app_name {
            AppName::ProbiotBot => ProbiotBotCollections::iter()
                .map(AppsCollections::ProbiotBot)
                .collect(),
            AppName::W3ABot => W3ABotCollections::iter()
                .map(AppsCollections::W3ABot)
                .collect(),
            _ => vec![], // Return an empty vector if no collections are defined for the application
        }
    }
}
