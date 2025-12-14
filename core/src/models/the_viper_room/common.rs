use serde::Deserialize;

pub enum TTSProvider {
    OpenAI,
    ElevenLabs,
    Google,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PodcastStructure {
    pub intro: String,
    pub body: Vec<String>,
    pub outro: String,
}
