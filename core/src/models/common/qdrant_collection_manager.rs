use crate::models::common::app_name::AppName;
use crate::models::tg_bot::probiot_bot::qdrant_collections::ProbiotCollections;
use crate::models::tg_bot::w3a_bot::qdrant_collections::W3ACollections;
use strum::IntoEnumIterator;

#[derive(Debug, Clone, PartialEq)]
pub enum AppsCollections {
    Probiot(ProbiotCollections),
    W3A(W3ACollections),
}

impl AppsCollections {
    pub fn as_str(&self) -> &str {
        match self {
            AppsCollections::Probiot(collection) => collection.as_str(),
            AppsCollections::W3A(collection) => collection.as_str(),
        }
    }

    pub fn all_collections_for_app(app_name: AppName) -> Vec<Self> {
        match app_name {
            AppName::ProbiotBot => ProbiotCollections::iter()
                .map(AppsCollections::Probiot)
                .collect(),
            AppName::W3ABot => W3ACollections::iter().map(AppsCollections::W3A).collect(),
            AppName::W3AWeb => W3ACollections::iter().map(AppsCollections::W3A).collect(),
            _ => vec![],
        }
    }
}
