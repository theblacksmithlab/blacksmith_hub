use crate::models::blacksmith_web::qdrant_collections::BlacksmithLabCollections;
use crate::models::common::app_name::AppName;
use crate::models::common::link_variant::LinkVariant;
use crate::models::tg_bot::probiot_bot::qdrant_collections::ProbiotCollections;
use crate::models::w3a_web::qdrant_collections::W3ACollections;
use strum::IntoEnumIterator;

/// Suffix of the alternative collection that mirrors a base collection with
/// alternative lesson links (e.g. `w3a_main` -> `w3a_main_alt`).
const ALT_COLLECTION_SUFFIX: &str = "_alt";

#[derive(Debug, Clone, PartialEq)]
pub enum AppsCollections {
    Probiot(ProbiotCollections),
    W3A(W3ACollections),
    BlacksmithLab(BlacksmithLabCollections),
}

impl AppsCollections {
    pub fn as_str(&self) -> &str {
        match self {
            AppsCollections::Probiot(collection) => collection.as_str(),
            AppsCollections::W3A(collection) => collection.as_str(),
            AppsCollections::BlacksmithLab(collection) => collection.as_str(),
        }
    }

    pub fn all_collections_for_app(app_name: AppName) -> Vec<Self> {
        match app_name {
            AppName::ProbiotBot => ProbiotCollections::iter()
                .map(AppsCollections::Probiot)
                .collect(),
            AppName::W3AWeb => W3ACollections::iter().map(AppsCollections::W3A).collect(),
            AppName::BlacksmithWeb => BlacksmithLabCollections::iter()
                .map(AppsCollections::BlacksmithLab)
                .collect(),
            _ => vec![],
        }
    }
}

/// Resolve the concrete collection names to search for an app, honoring the
/// link variant. Only W3A has an alternative collection today; for every other
/// app the variant is a no-op and the base names are returned unchanged.
pub fn collection_names_for_app(app_name: AppName, variant: LinkVariant) -> Vec<String> {
    let base: Vec<String> = AppsCollections::all_collections_for_app(app_name.clone())
        .iter()
        .map(|collection| collection.as_str().to_string())
        .collect();

    match (app_name, variant) {
        (AppName::W3AWeb, LinkVariant::Alt) => base
            .into_iter()
            .map(|name| format!("{name}{ALT_COLLECTION_SUFFIX}"))
            .collect(),
        _ => base,
    }
}
