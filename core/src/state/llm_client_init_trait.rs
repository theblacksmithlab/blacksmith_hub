use crate::ai::anthropic_client::AnthropicClient;
use crate::ai::google_client::GoogleClient;
use crate::state::blacksmith_web::app_state::BlacksmithWebAppState;
use crate::state::tg_agent::app_state::AgentAppState;
use crate::state::tg_bot::{GrootBotState, ProbiotBotState, StatBotState, TheViperRoomBotState};
use crate::state::the_viper_room::app_state::TheViperRoomAppState;
use crate::state::uniframe_studio::app_state::UniframeStudioAppState;
use async_openai::config::OpenAIConfig;
use async_openai::Client as OpenAIClient;

// ============ OpenAI Client Trait ============

pub trait OpenAIClientInit {
    fn get_openai_client(&self) -> &OpenAIClient<OpenAIConfig>;
}

impl OpenAIClientInit for TheViperRoomAppState {
    fn get_openai_client(&self) -> &OpenAIClient<OpenAIConfig> {
        &self.openai_client
    }
}

impl OpenAIClientInit for BlacksmithWebAppState {
    fn get_openai_client(&self) -> &OpenAIClient<OpenAIConfig> {
        &self.openai_client
    }
}

impl OpenAIClientInit for UniframeStudioAppState {
    fn get_openai_client(&self) -> &OpenAIClient<OpenAIConfig> {
        &self.openai_client
    }
}
impl OpenAIClientInit for AgentAppState {
    fn get_openai_client(&self) -> &OpenAIClient<OpenAIConfig> {
        &self.openai_client
    }
}

impl OpenAIClientInit for ProbiotBotState {
    fn get_openai_client(&self) -> &OpenAIClient<OpenAIConfig> {
        &self.core.openai_client
    }
}

impl OpenAIClientInit for GrootBotState {
    fn get_openai_client(&self) -> &OpenAIClient<OpenAIConfig> {
        &self.core.openai_client
    }
}

impl OpenAIClientInit for TheViperRoomBotState {
    fn get_openai_client(&self) -> &OpenAIClient<OpenAIConfig> {
        &self.core.openai_client
    }
}

impl OpenAIClientInit for StatBotState {
    fn get_openai_client(&self) -> &OpenAIClient<OpenAIConfig> {
        &self.core.openai_client
    }
}

// ============ Anthropic Client Trait ============

pub trait AnthropicClientInit {
    fn get_anthropic_client(&self) -> &AnthropicClient;
}

impl AnthropicClientInit for TheViperRoomAppState {
    fn get_anthropic_client(&self) -> &AnthropicClient {
        &self.anthropic_client
    }
}

impl AnthropicClientInit for BlacksmithWebAppState {
    fn get_anthropic_client(&self) -> &AnthropicClient {
        &self.anthropic_client
    }
}

impl AnthropicClientInit for UniframeStudioAppState {
    fn get_anthropic_client(&self) -> &AnthropicClient {
        &self.anthropic_client
    }
}

impl AnthropicClientInit for AgentAppState {
    fn get_anthropic_client(&self) -> &AnthropicClient {
        &self.anthropic_client
    }
}

impl AnthropicClientInit for ProbiotBotState {
    fn get_anthropic_client(&self) -> &AnthropicClient {
        &self.core.anthropic_client
    }
}

impl AnthropicClientInit for GrootBotState {
    fn get_anthropic_client(&self) -> &AnthropicClient {
        &self.core.anthropic_client
    }
}

impl AnthropicClientInit for TheViperRoomBotState {
    fn get_anthropic_client(&self) -> &AnthropicClient {
        &self.core.anthropic_client
    }
}

impl AnthropicClientInit for StatBotState {
    fn get_anthropic_client(&self) -> &AnthropicClient {
        &self.core.anthropic_client
    }
}

// ============ Google Client Trait ============

pub trait GoogleClientInit {
    fn get_google_client(&self) -> &GoogleClient;
}

impl GoogleClientInit for TheViperRoomAppState {
    fn get_google_client(&self) -> &GoogleClient {
        &self.google_client
    }
}

impl GoogleClientInit for BlacksmithWebAppState {
    fn get_google_client(&self) -> &GoogleClient {
        &self.google_client
    }
}

impl GoogleClientInit for UniframeStudioAppState {
    fn get_google_client(&self) -> &GoogleClient {
        &self.google_client
    }
}

impl GoogleClientInit for AgentAppState {
    fn get_google_client(&self) -> &GoogleClient {
        &self.google_client
    }
}

impl GoogleClientInit for ProbiotBotState {
    fn get_google_client(&self) -> &GoogleClient {
        &self.core.google_client
    }
}

impl GoogleClientInit for GrootBotState {
    fn get_google_client(&self) -> &GoogleClient {
        &self.core.google_client
    }
}

impl GoogleClientInit for TheViperRoomBotState {
    fn get_google_client(&self) -> &GoogleClient {
        &self.core.google_client
    }
}

impl GoogleClientInit for StatBotState {
    fn get_google_client(&self) -> &GoogleClient {
        &self.core.google_client
    }
}
