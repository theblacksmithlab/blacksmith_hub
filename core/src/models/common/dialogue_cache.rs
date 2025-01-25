use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{HashMap, VecDeque};

#[derive(Default)]
pub struct DialogueCache {
    messages: VecDeque<UserInteraction>,
    max_size: usize,
    tts_data: HashMap<String, String>,
}

impl DialogueCache {
    pub fn new(max_size: usize) -> Self {
        DialogueCache {
            messages: VecDeque::new(),
            max_size,
            tts_data: HashMap::new(),
        }
    }

    pub(crate) fn add_user_message(&mut self, user_question: String) {
        let timestamp = Utc::now().to_rfc3339();

        let entry = UserInteraction {
            timestamp,
            role: "user".to_string(),
            content: user_question,
        };

        self.messages.push_back(entry);

        if self.messages.len() > self.max_size {
            self.messages.pop_front();
        }
    }

    pub(crate) fn count_user_messages(&self) -> usize {
        self.messages
            .iter()
            .filter(|interaction| interaction.role == "user")
            .count()
    }

    pub(crate) fn add_llm_response_to_cache(&mut self, llm_response: String) {
        let timestamp = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let entry = UserInteraction {
            timestamp,
            role: "assistant".to_string(),
            content: llm_response,
        };

        self.messages.push_back(entry);

        if self.messages.len() > self.max_size {
            self.messages.pop_front();
        }
    }

    pub(crate) fn get_cache_as_string(&self) -> String {
        let formatted_messages: Vec<_> = self
            .messages
            .iter()
            .map(|interaction| {
                json!({
                    "role": interaction.role,
                    "content": interaction.content,
                    "timestamp": interaction.timestamp
                })
            })
            .collect();

        serde_json::to_string_pretty(&formatted_messages).unwrap_or_else(|e| {
            eprintln!("Error serializing temp cache: {}", e);
            "[]".to_string()
        })
    }

    pub fn add_tts_payload(&mut self, message_id: String, tts_payload: String) {
        self.tts_data.insert(message_id, tts_payload);
    }
    
    pub fn get_and_remove_tts_payload(&mut self, message_id: String) -> Option<String> {
        self.tts_data.remove(&message_id)
    }
    
}

#[derive(Debug, Serialize, Deserialize)]
struct UserInteraction {
    timestamp: String,
    role: String,
    content: String,
}
