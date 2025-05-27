use crate::state::blacksmith_web::app_state::BlacksmithWebAppState;
use crate::state::tg_bot::app_state::BotAppState;
use crate::state::the_viper_room::app_state::TheViperRoomAppState;
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

impl OpenAIClientInit for BotAppState {
    fn get_llm_client(&self) -> &LLM_Client<OpenAIConfig> {
        &self.llm_client
    }
}

impl OpenAIClientInit for BlacksmithWebAppState {
    fn get_llm_client(&self) -> &LLM_Client<OpenAIConfig> {
        &self.llm_client
    }
}
