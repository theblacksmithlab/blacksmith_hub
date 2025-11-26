pub enum LlmModel {
    Light,               // OpenAI gpt-4o-mini
    ComplexMini,             // OpenAI gpt-5-mini
    ComplexPro,            // OpenAI gpt-5.1
    TextEmbedding3Large, // OpenAI embedding generative model
}

impl LlmModel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LlmModel::Light => "gpt-4o-mini",
            LlmModel::ComplexMini => "gpt-5-mini",
            LlmModel::ComplexPro => "gpt-5.1",
            LlmModel::TextEmbedding3Large => "text-embedding-3-large",
        }
    }

    pub fn is_gpt5_model(&self) -> bool {
        matches!(
            self,
            LlmModel::ComplexMini | LlmModel::ComplexPro
        )
    }
}
