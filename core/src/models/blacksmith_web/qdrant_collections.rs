use strum_macros::EnumIter;

#[derive(Hash, Eq, PartialEq, Debug, Clone, EnumIter)]
pub enum BlacksmithLabCollections {
    Test,
}

impl BlacksmithLabCollections {
    pub fn as_str(&self) -> &str {
        match self {
            BlacksmithLabCollections::Test => "test",
        }
    }
}