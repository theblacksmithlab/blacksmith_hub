use crate::rag_system::types::PointId;

#[derive(Debug, Clone)]
pub struct DocumentMetadata {
    pub title: String,
    pub extra: Option<String>,
    pub hierarchy: Option<String>,
}

#[derive(Clone, Debug)]
pub struct HybridSearchDocument {
    pub point_id: PointId,
    pub text: String,
    pub score: f32,
    pub vector: Option<Vec<f32>>,
    pub document_id: String,
    pub metadata: DocumentMetadata,
    pub matched_chunk_indices: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct ChunkSearchResult {
    pub document_id: String,
    pub chunk_index: u32,
    pub chunk_text: String,
    pub score: f32,
    pub metadata: DocumentMetadata,
}

#[derive(Debug, Clone)]
pub struct DescriptionSearchResult {
    pub document_id: String,
    pub description_text: String,
    pub score: f32,
    pub metadata: DocumentMetadata,
}

#[derive(Debug, Clone)]
pub struct DocumentAggregation {
    pub document_id: String,
    pub metadata: DocumentMetadata,

    // Chunks data
    pub matched_chunks: Vec<ChunkSearchResult>,
    pub max_chunk_score: Option<f32>,
    pub chunk_rank: Option<usize>,

    // Description data
    pub description_score: Option<f32>,
    pub description_rank: Option<usize>,

    // Final score (to count later)
    pub final_score: f32,
}
