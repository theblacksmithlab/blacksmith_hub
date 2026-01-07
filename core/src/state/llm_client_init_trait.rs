use crate::state::blacksmith_web::app_state::BlacksmithWebAppState;
use crate::state::tg_agent::app_state::AgentAppState;
use crate::state::tg_bot::{GrootBotState, ProbiotBotState, StatBotState, TheViperRoomBotState};
use crate::state::the_viper_room::app_state::TheViperRoomAppState;
use crate::state::uniframe_studio::app_state::UniframeStudioAppState;
use async_openai::config::OpenAIConfig;
use async_openai::Client as LLM_Client;

pub trait OpenAIClientInit {
    fn get_llm_client(&self) -> &LLM_Client<OpenAIConfig>;
}

impl OpenAIClientInit for TheViperRoomAppState {
    fn get_llm_client(&self) -> &LLM_Client<OpenAIConfig> {
        &self.llm_client
    }
}

impl OpenAIClientInit for BlacksmithWebAppState {
    fn get_llm_client(&self) -> &LLM_Client<OpenAIConfig> {
        &self.llm_client
    }
}

impl OpenAIClientInit for UniframeStudioAppState {
    fn get_llm_client(&self) -> &LLM_Client<OpenAIConfig> {
        &self.llm_client
    }
}
impl OpenAIClientInit for AgentAppState {
    fn get_llm_client(&self) -> &LLM_Client<OpenAIConfig> {
        &self.llm_client
    }
}

impl OpenAIClientInit for ProbiotBotState {
    fn get_llm_client(&self) -> &LLM_Client<OpenAIConfig> {
        &self.core.llm_client
    }
}

impl OpenAIClientInit for GrootBotState {
    fn get_llm_client(&self) -> &LLM_Client<OpenAIConfig> {
        &self.core.llm_client
    }
}

impl OpenAIClientInit for TheViperRoomBotState {
    fn get_llm_client(&self) -> &LLM_Client<OpenAIConfig> {
        &self.core.llm_client
    }
}

impl OpenAIClientInit for StatBotState {
    fn get_llm_client(&self) -> &LLM_Client<OpenAIConfig> {
        &self.core.llm_client
    }
}
