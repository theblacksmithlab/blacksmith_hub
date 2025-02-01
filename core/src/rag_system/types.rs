#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum PointId {
    Num(u64),
    Uuid(String),
}

#[derive(Clone, Debug)]
pub struct Document {
    pub point_id: PointId,
    pub content: String,
    pub score: Option<f32>,
    pub metadata: Option<DocumentMetadata>,
    pub vector: Option<Vec<f32>>,
}

#[derive(Clone, Debug)]
pub struct DocumentMetadata {
    pub source: String,
    pub timestamp: Option<i64>,
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
