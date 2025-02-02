use strum_macros::EnumIter;

#[derive(Hash, Eq, PartialEq, Debug, Clone, EnumIter)]
pub enum W3ACollections {
    Base1,
    Base2,
    Base3,
    Base4,
    Base5,
    Base6,
}

impl W3ACollections {
    pub fn as_str(&self) -> &str {
        match self {
            W3ACollections::Base1 => "base_v2_1",
            W3ACollections::Base2 => "base_v2_2",
            W3ACollections::Base3 => "base_v2_3",
            W3ACollections::Base4 => "base_v2_4",
            W3ACollections::Base5 => "base_v2_5",
            W3ACollections::Base6 => "base_v2_6",
        }
    }
}
