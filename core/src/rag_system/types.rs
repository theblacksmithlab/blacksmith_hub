use crate::rag_system::hybrid_search_types::HybridSearchDocument;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum PointId {
    Num(u64),
    Uuid(String),
}

#[derive(Clone, Debug)]
pub struct Document {
    pub point_id: PointId,
    pub text: String,
    pub score: f32,
    pub vector: Option<Vec<f32>>,
}

#[derive(Clone, Debug)]
pub struct W3ADocument {
    pub point_id: PointId,
    pub text: String,
    pub score: Option<f32>,
    pub vector: Option<Vec<f32>>,
    pub module: String,
    pub block_title: String,
    pub lesson_title: String,
    pub segment_id: i64,
}

#[derive(Clone, Debug)]
pub enum DocumentType {
    Default(Document),
    W3A(W3ADocument),
    HybridSearch(HybridSearchDocument),
}

#[derive(Clone)]
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
    PayloadKeyBased {
        max_documents: usize,
        similarity_threshold: f32,
    },
    HybridSearch {
        top_k_chunks: usize,
        chunks_similarity_threshold: f32,

        top_k_descriptions: usize,
        descriptions_similarity_threshold: f32,

        // RRF or WeightedSum
        ranking_method: RankingMethod,

        // Top-N results
        final_documents_count: usize,
    },
}

#[derive(Clone)]
pub enum RankingMethod {
    RRF {
        k: f32,
    },
    WeightedSum {
        chunk_weight: f32,       // 0.7
        description_weight: f32, // 0.3
    },
}

pub struct RetrievedContext {
    pub context: String,
    pub documents: Vec<DocumentType>,
}
