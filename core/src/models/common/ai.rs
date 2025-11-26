use async_openai::types::ReasoningEffort;

pub enum LlmModel {
    Tiny,
    Light,
    ComplexMini,
    Complex,
    TextEmbedding3Large, // OpenAI embedding generative model
    ComplexFast,      // gpt-5.1 (low reasoning)
}

impl LlmModel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LlmModel::Tiny => "got-4o-mini",
            LlmModel::Light => "gpt-4o",
            LlmModel::ComplexMini => "gpt-5-mini",
            LlmModel::Complex | LlmModel::ComplexFast => "gpt-5.1",
            LlmModel::TextEmbedding3Large => "text-embedding-3-large",
        }
    }

    pub fn is_gpt5_model(&self) -> bool {
        matches!(
            self,
            LlmModel::ComplexMini | LlmModel::Complex | LlmModel::ComplexFast
        )
    }

    pub fn reasoning_effort(&self) -> Option<ReasoningEffort> {
        match self {
            LlmModel::ComplexFast => Some(ReasoningEffort::Low),
            _ => None
        }
    }
}
