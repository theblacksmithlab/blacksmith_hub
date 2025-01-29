use strum_macros::EnumIter;

#[derive(Hash, Eq, PartialEq, Debug, Clone, EnumIter)]
pub enum ProbiotBotCollections {
    BigData1,
    BigData2,
    Qa,
    Products,
    IllnessAndRelief,
    ProbioCollection,
}

impl ProbiotBotCollections {
    pub fn as_str(&self) -> &str {
        match self {
            ProbiotBotCollections::BigData1 => "probiot_big_data_1",
            ProbiotBotCollections::BigData2 => "probiot_big_data_2",
            ProbiotBotCollections::Qa => "probio_qa",
            ProbiotBotCollections::Products => "products",
            ProbiotBotCollections::IllnessAndRelief => "illness_and_relief",
            ProbiotBotCollections::ProbioCollection => "probio_collection",
        }
    }
}
