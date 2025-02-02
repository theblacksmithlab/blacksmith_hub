use strum_macros::EnumIter;

#[derive(Hash, Eq, PartialEq, Debug, Clone, EnumIter)]
pub enum ProbiotCollections {
    BigData1,
    BigData2,
    Qa,
    Products,
    IllnessAndRelief,
    ProbioCollection,
}

impl ProbiotCollections {
    pub fn as_str(&self) -> &str {
        match self {
            ProbiotCollections::BigData1 => "probiot_big_data_1",
            ProbiotCollections::BigData2 => "probiot_big_data_2",
            ProbiotCollections::Qa => "probio_qa",
            ProbiotCollections::Products => "products",
            ProbiotCollections::IllnessAndRelief => "illness_and_relief",
            ProbiotCollections::ProbioCollection => "probio_collection",
        }
    }
}
