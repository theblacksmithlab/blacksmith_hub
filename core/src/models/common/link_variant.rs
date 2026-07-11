/// Which lesson-link set the RAG answer should carry.
///
/// `Default` -> primary Qdrant collection, `Alt` -> the alternative collection
/// holding the same content with alternative lesson links. The frontend signals
/// this per deployment; an absent/unknown value falls back to `Default`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum LinkVariant {
    #[default]
    Default,
    Alt,
}

impl LinkVariant {
    pub fn from_opt(value: Option<&str>) -> Self {
        match value.map(str::trim) {
            Some("alt") => LinkVariant::Alt,
            _ => LinkVariant::Default,
        }
    }
}
