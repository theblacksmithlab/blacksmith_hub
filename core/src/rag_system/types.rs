#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum PointId {
    Num(u64),
    Uuid(String),
}

#[derive(Clone, Debug)]
pub struct Document {
    pub point_id: PointId,
    pub content: String,
    pub score: f32,
    pub vector: Option<Vec<f32>>,
}

pub enum RAGConfig {
    Default {
        max_documents: usize,
        similarity_threshold: f32,
    },
    Advanced {
        base_max_documents: usize,
        base_similarity_threshold: f32,
        related_max_documents: usize,
        related_similarity_threshold: f32,
    },
}

pub struct RetrievedContext {
    pub context: String,
    pub documents: Vec<Document>,
}
