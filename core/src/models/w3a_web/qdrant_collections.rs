use strum_macros::EnumIter;

#[derive(Hash, Eq, PartialEq, Debug, Clone, EnumIter)]
pub enum W3ACollections {
    W3AMain,
}

impl W3ACollections {
    pub fn as_str(&self) -> &str {
        match self {
            W3ACollections::W3AMain => "w3a_main",
        }
    }
}
