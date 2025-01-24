#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub enum ProbiotCollection {
    BigData1,
    BigData2,
    Qa,
    Products,
    IllnessAndRelief,
}

impl ProbiotCollection {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProbiotCollection::BigData1 => "probiot_big_data_1",
            ProbiotCollection::BigData2 => "probiot_big_data_2",
            ProbiotCollection::Qa => "probio_qa",
            ProbiotCollection::Products => "products",
            ProbiotCollection::IllnessAndRelief => "illness_and_relief",
        }
    }
}
