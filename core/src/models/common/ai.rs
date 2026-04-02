use async_openai::types::ReasoningEffort;
use strum_macros::Display;

// ============ OpenAI Models ============

#[derive(Debug, Display, Clone, Copy)]
pub enum OpenAIModel {
    GPT5hr,           // gpt-5.4 (high reasoning)
    GPT5mr,            // gpt-5.4 (medium reasoning)
    GPT5lr,          // gpt-5.4 (low reasoning)
    Embedding3Large, // OpenAI embedding generative model
    TTS,             // TTS model
}

impl OpenAIModel {
    pub fn as_str(&self) -> &'static str {
        match self {
            OpenAIModel::GPT5hr | OpenAIModel::GPT5mr |OpenAIModel::GPT5lr => "gpt-5.4",
            OpenAIModel::Embedding3Large => "text-embedding-3-large",
            OpenAIModel::TTS => "gpt-4o-mini-tts",
        }
    }

    pub fn is_gpt5_model(&self) -> bool {
        matches!(
            self,
            OpenAIModel::GPT5mr | OpenAIModel::GPT5lr | OpenAIModel::GPT5hr
        )
    }

    pub fn reasoning_effort(&self) -> Option<ReasoningEffort> {
        match self {
            OpenAIModel::GPT5lr  => Some(ReasoningEffort::Low),
            OpenAIModel::GPT5mr => Some(ReasoningEffort::Medium),
            OpenAIModel::GPT5hr => Some(ReasoningEffort::High),
            _ => None,
        }
    }
}

// ============ Anthropic Models ============

#[derive(Debug, Display, Clone, Copy)]
pub enum AnthropicModel {
    Sonnet, // claude-sonnet-4-5 (latest)
    Opus,   // claude-opus-4-5 (latest)
}

impl AnthropicModel {
    pub fn as_str(&self) -> &'static str {
        match self {
            AnthropicModel::Sonnet => "claude-sonnet-4-5",
            AnthropicModel::Opus => "claude-opus-4-5",
        }
    }
}

// ============ Google Models ============

#[derive(Debug, Display, Clone, Copy)]
pub enum GoogleModel {
    Flash, // gemini-3-flash (latest)
    Pro,   // gemini-3-pro (latest)
}

impl GoogleModel {
    pub fn as_str(&self) -> &'static str {
        match self {
            GoogleModel::Flash => "gemini-3-flash-preview",
            GoogleModel::Pro => "gemini-3-pro-preview",
        }
    }
}
