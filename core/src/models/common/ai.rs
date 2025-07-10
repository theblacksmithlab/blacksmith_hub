pub enum LlmModel {
    Light,               // OpenAI gpt-4o-mini
    Complex,      
    Complex2,// OpenAI gpt-4o
    TextEmbedding3Large, // OpenAI embedding generative model
}

impl LlmModel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LlmModel::Light => "gpt-4o-mini",
            LlmModel::Complex => "gpt-4o",
            LlmModel::Complex2 => "gpt-4.1-2025-04-14",
            LlmModel::TextEmbedding3Large => "text-embedding-3-large",
        }
    }
}
