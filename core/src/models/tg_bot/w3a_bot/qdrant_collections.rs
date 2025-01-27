use strum_macros::EnumIter;

#[derive(Hash, Eq, PartialEq, Debug, Clone, EnumIter)]
pub enum W3ABotCollections {
    Base1,
    Base2,
    Base3,
    Base4,
    Base5,
    Base6,
}

impl W3ABotCollections {
    pub fn as_str(&self) -> &str {
        match self {
            W3ABotCollections::Base1 => "base_v2_1",
            W3ABotCollections::Base2 => "base_v2_2",
            W3ABotCollections::Base3 => "base_v2_3",
            W3ABotCollections::Base4 => "base_v2_4",
            W3ABotCollections::Base5 => "base_v2_5",
            W3ABotCollections::Base6 => "base_v2_6",
        }
    }
}
