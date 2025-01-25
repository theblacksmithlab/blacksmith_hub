use crate::models::tg_bot::probiot::qdrant_collections::ProbiotCollection;

pub struct ApplicationManager {
    probiot_collections: Vec<ProbiotCollection>,
}

impl ApplicationManager {
    pub fn new() -> Self {
        Self {
            probiot_collections: vec![
                ProbiotCollection::BigData1,
                ProbiotCollection::BigData2,
                ProbiotCollection::Qa,
                ProbiotCollection::Products,
                ProbiotCollection::IllnessAndRelief,
            ],
        }
    }

    pub fn get_probiot_collections(&self) -> &Vec<ProbiotCollection> {
        &self.probiot_collections
    }

    pub fn get_probiot_collection(
        &self,
        collection: ProbiotCollection,
    ) -> Option<&ProbiotCollection> {
        self.probiot_collections
            .iter()
            .find(|&col| *col == collection)
    }
}
