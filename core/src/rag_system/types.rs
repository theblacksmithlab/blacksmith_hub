#[derive(Clone, Debug)]
pub struct Document {
    pub content: String,
    pub metadata: Option<DocumentMetadata>,
    pub score: Option<f32>,
}

#[derive(Clone, Debug)]
pub struct DocumentMetadata {
    pub source: String,
    pub timestamp: Option<i64>,
}

pub struct RAGConfig {
    pub max_documents: usize,
    pub similarity_threshold: f32,
}

pub struct RetrievedContext {
    pub context: String,
    pub documents: Vec<Document>,
}
