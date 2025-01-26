use crate::models::common::app_name::AppName;
use crate::models::tg_bot::probiot::qdrant_collections::ProbiotCollections;
use strum::IntoEnumIterator;

#[derive(Debug, Clone, PartialEq)]
pub enum AppsCollections {
    Probiot(ProbiotCollections),
}

impl AppsCollections {
    pub fn as_str(&self) -> &str {
        match self {
            AppsCollections::Probiot(collection) => collection.as_str(),
        }
    }

    pub fn all_collections_for_app(app_name: AppName) -> Vec<Self> {
        match app_name {
            AppName::Probiot => ProbiotCollections::iter()
                .map(AppsCollections::Probiot)
                .collect(),
            _ => vec![], // Return an empty vector if no collections are defined for the application
        }
    }
}
