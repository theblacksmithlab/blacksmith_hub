pub enum LlmModel {
    Light,               // OpenAI gpt-4o-mini
    Complex,             // OpenAI gpt-4o
    TextEmbedding3Large, // OpenAI embedding generative model
}

impl LlmModel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LlmModel::Light => "gpt-4o-mini",
            LlmModel::Complex => "gpt-4o",
            LlmModel::TextEmbedding3Large => "text-embedding-3-large",
        }
    }
}
