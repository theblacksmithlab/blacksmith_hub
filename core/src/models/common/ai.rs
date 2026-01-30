use async_openai::types::ReasoningEffort;
use strum_macros::Display;

// ============ OpenAI Models ============

#[derive(Debug, Display, Clone, Copy)]
pub enum OpenAIModel {
    GPT4o, // gpt-4o
    GPT5, // gpt-5.2
    GPT5Fast, // gpt-5.2 (low reasoning)
    Embedding3Large, // OpenAI embedding generative model
    TTS, // TTS model
}

impl OpenAIModel {
    pub fn as_str(&self) -> &'static str {
        match self {
            OpenAIModel::GPT4o => "gpt-4o",
            OpenAIModel::GPT5 | OpenAIModel::GPT5Fast => "gpt-5.2-chat",
            OpenAIModel::Embedding3Large => "text-embedding-3-large",
            OpenAIModel::TTS => "gpt-4o-mini-tts",
        }
    }

    pub fn is_gpt5_model(&self) -> bool {
        matches!(self, OpenAIModel::GPT5 | OpenAIModel::GPT5Fast)
    }

    pub fn reasoning_effort(&self) -> Option<ReasoningEffort> {
        match self {
            OpenAIModel::GPT5Fast => Some(ReasoningEffort::Low),
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
            GoogleModel::Flash => "gemini-3.0-flash-latest",
            GoogleModel::Pro => "gemini-3.0-pro-latest",
        }
    }
}
