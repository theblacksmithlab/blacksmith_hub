use async_openai::config::OpenAIConfig;
use async_openai::Client as LLM_Client;
use qdrant_client::qdrant::ScoredPoint;
use qdrant_client::Qdrant;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use teloxide::prelude::ChatId;
use tokio::sync::Mutex;

pub struct RequestAppState {
    pub qdrant_client: Arc<Qdrant>,
    pub llm_client: LLM_Client<OpenAIConfig>,
    pub user_states: Mutex<HashMap<ChatId, UserStates>>,
    pub user_profile: Mutex<HashMap<ChatId, UserProfile>>,
    pub user_request: Mutex<HashMap<ChatId, String>>,
    pub last_request_result_author: Mutex<HashMap<ChatId, String>>,
    pub user_search_results: Arc<Mutex<HashMap<ChatId, UserSearchResults>>>,
    pub local_db_pool: Mutex<Option<Pool<Sqlite>>>,
}

impl RequestAppState {
    pub fn new(qdrant_client: Arc<Qdrant>, llm_client: LLM_Client<OpenAIConfig>) -> Self {
        Self {
            qdrant_client,
            llm_client,
            user_states: Mutex::new(HashMap::new()),
            user_profile: Mutex::new(HashMap::new()),
            user_request: Mutex::new(HashMap::new()),
            last_request_result_author: Mutex::new(HashMap::new()),
            user_search_results: Arc::new(Mutex::new(HashMap::new())),
            local_db_pool: Mutex::new(None),
        }
    }
}

#[derive(Default, Clone)]
pub struct UserStates {
    pub start_window: bool,
    pub main_menu: bool,
    pub profile_menu: bool,
    pub checking_profile: bool,
    pub editing_profile: bool,
    pub creating_profile: bool,
    pub creating_profile_process: bool,
    pub request_menu: bool,
    pub checking_request: bool,
    pub editing_request: bool,
    pub editing_request_process: bool,
    pub creating_request: bool,
    pub creating_request_process: bool,
    pub request_actuality_menu: bool,
    pub request_actuality: bool,
    pub request_search_result: bool,
    pub request_search_result_exploring: bool,
    pub current_result_index: Option<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UserProfile {
    pub registration_info: RegistrationInfo,
    pub additional_info: AdditionalInfo,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RegistrationInfo {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub age: Option<u8>,
    pub gender: Option<String>,
    pub city_of_residence: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AdditionalInfo {
    pub interests: Option<Vec<String>>,
}

impl UserProfile {
    pub fn new() -> Self {
        UserProfile {
            registration_info: RegistrationInfo {
                first_name: None,
                last_name: None,
                age: None,
                gender: None,
                city_of_residence: None,
            },
            additional_info: AdditionalInfo { interests: None },
        }
    }
}

#[derive(Debug, Clone)]
pub struct UserSearchResults {
    pub points: BTreeMap<usize, ScoredPoint>,
    pub order: Vec<usize>,
}
