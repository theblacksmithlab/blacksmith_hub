use crate::models::common::system_messages::{AppsSystemMessages, TheViperRoomBotMessages};
use crate::state::llm_client_init_trait::OpenAIClientInit;
use crate::utils::common::get_message;
use anyhow::anyhow;
use async_openai::types::{CreateSpeechRequestArgs, CreateSpeechResponse, SpeechModel, Voice};
use base64::{engine::general_purpose, Engine as _};
use reqwest::multipart;
use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::fs::remove_file;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Instant;
use tracing::{info, warn};

// ============ Speech to text utilities ============
#[derive(Serialize, Deserialize, Debug)]
struct WhisperTranscribeResponse {
    text: String,
    duration_ms: u128,
}

pub async fn speech_to_text(file_path: &Path) -> anyhow::Result<String> {
    let start = Instant::now();

    if !file_path.exists() {
        return Err(anyhow!(
            "Voice message file not found: {}",
            file_path.display()
        ));
    }

    let whisper_url = std::env::var("WHISPER_SERVICE_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:9000".to_string());

    info!("Using whisper service at: {}", whisper_url);

    let audio_data = fs::read(file_path)?;

    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("audio.ogg");

    let part = multipart::Part::bytes(audio_data)
        .file_name(file_name.to_string())
        .mime_str("audio/ogg")?;

    let form = multipart::Form::new().part("audio", part);

    let client = ReqwestClient::new();
    let response = client
        .post(format!("{}/transcribe", whisper_url))
        .multipart(form)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(anyhow!("Whisper service error: {}", error_text));
    }

    let transcribe_response: WhisperTranscribeResponse = response.json().await?;

    info!(
        "Transcription completed in {:?} (service: {}ms)",
        start.elapsed(),
        transcribe_response.duration_ms
    );

    if transcribe_response.text.trim().is_empty() {
        Ok("Empty text".to_string())
    } else {
        Ok(transcribe_response.text)
    }
}

// ============ Text to speech utilities ============
pub async fn openai_base_tts<T: OpenAIClientInit + Send + Sync>(
    text: &str,
    app_state: Arc<T>,
    speed: f32,
) -> anyhow::Result<CreateSpeechResponse> {
    let openai_client = app_state.get_openai_client().clone();

    let request = CreateSpeechRequestArgs::default()
        .input(text)
        .voice(Voice::Onyx)
        .model(SpeechModel::Tts1Hd)
        .speed(speed)
        .build()?;

    let response = openai_client.audio().speech(request).await?;

    Ok(response)
}

pub(crate) async fn elevenlabs_base_tts(text: &str, api_key: &str) -> anyhow::Result<Vec<u8>> {
    let client = ReqwestClient::new();

    let voice_id = "vpUqfpCIn34tjFW4KHjt";

    let response = client
        .post(format!(
            "https://api.elevenlabs.io/v1/text-to-speech/{}",
            voice_id
        ))
        .header("xi-api-key", api_key)
        .header("Content-Type", "application/json")
        .json(&json!({
            "text": text,
            "model_id": "eleven_multilingual_v2",
            "voice_settings": {
                "stability": 0.5,
                "similarity_boost": 0.5,
                "style": 0.5,
                "speed": 1.05,
                "use_speaker_boost": true
            }
        }))
        .send()
        .await?
        .bytes()
        .await?
        .to_vec();

    Ok(response)
}

pub async fn gemini_base_tts(
    text: &str,
    api_key: &str,
    tts_instruction: &str,
) -> anyhow::Result<Vec<u8>> {
    let client = ReqwestClient::new();

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash-preview-tts:generateContent?key={}",
        api_key
    );

    let full_text = format!("{}\n{}", tts_instruction, text);

    let payload = json!({
        "contents": [{
            "role": "user",
            "parts": [{
                "text": full_text
            }]
        }],
        "generationConfig": {
            "responseModalities": ["AUDIO"],
            "seed": 42,
            "speechConfig": {
                "voiceConfig": {
                    "prebuiltVoiceConfig": {
                        "voiceName": "Charon"
                    }
                }
            }
        }
    });

    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(anyhow::anyhow!("Gemini API error: {}", error_text));
    }

    let response_json: serde_json::Value = response.json().await?;

    let audio_base64 = response_json
        .get("candidates")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("content"))
        .and_then(|c| c.get("parts"))
        .and_then(|p| p.get(0))
        .and_then(|p| p.get("inlineData"))
        .and_then(|d| d.get("data"))
        .and_then(|d| d.as_str())
        .ok_or_else(|| anyhow::anyhow!("Failed to extract audio data from Gemini response"))?;

    let audio_bytes = general_purpose::STANDARD
        .decode(audio_base64)
        .map_err(|e| anyhow::anyhow!("Failed to decode base64 audio: {}", e))?;

    let wav_data = pcm_to_wav(&audio_bytes, 24000, 1, 16)?;

    Ok(wav_data)
}

fn pcm_to_wav(
    pcm_data: &[u8],
    sample_rate: u32,
    channels: u16,
    bits_per_sample: u16,
) -> anyhow::Result<Vec<u8>> {
    let mut wav_data = Vec::new();

    wav_data.extend_from_slice(b"RIFF");
    let file_size = (36 + pcm_data.len()) as u32;
    wav_data.extend_from_slice(&(file_size - 8).to_le_bytes());
    wav_data.extend_from_slice(b"WAVE");

    wav_data.extend_from_slice(b"fmt ");
    wav_data.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    wav_data.extend_from_slice(&1u16.to_le_bytes()); // audio format (PCM)
    wav_data.extend_from_slice(&channels.to_le_bytes());
    wav_data.extend_from_slice(&sample_rate.to_le_bytes());
    let byte_rate = sample_rate * u32::from(channels) * u32::from(bits_per_sample) / 8;
    wav_data.extend_from_slice(&byte_rate.to_le_bytes());
    let block_align = channels * bits_per_sample / 8;
    wav_data.extend_from_slice(&block_align.to_le_bytes());
    wav_data.extend_from_slice(&bits_per_sample.to_le_bytes());

    wav_data.extend_from_slice(b"data");
    wav_data.extend_from_slice(&(pcm_data.len() as u32).to_le_bytes());
    wav_data.extend_from_slice(pcm_data);

    Ok(wav_data)
}

pub fn convert_pcm_to_mp3(input_path: &str, output_path: &str) -> anyhow::Result<()> {
    let status = Command::new("ffmpeg")
        .arg("-i")
        .arg(input_path)
        .arg("-acodec")
        .arg("libmp3lame")
        .arg("-b:a")
        .arg("128k")
        .arg("-y")
        .arg(output_path)
        .status()?;

    if !status.success() {
        return Err(anyhow::anyhow!("Failed to convert PCM to MP3"));
    }

    Ok(())
}

/// Generate a single audio part via OpenAI TTS
pub async fn generate_single_part_via_openai<T: OpenAIClientInit + Send + Sync>(
    text: &str,
    user_tmp_dir: &str,
    part_index: usize,
    app_state: Arc<T>,
) -> anyhow::Result<PathBuf> {
    info!(
        "Generating OpenAI TTS for part {}: {} chars",
        part_index,
        text.chars().count()
    );

    let openai_client = app_state.get_openai_client().clone();

    let request = CreateSpeechRequestArgs::default()
        .input(text)
        .voice(Voice::Onyx)
        .model(SpeechModel::Tts1Hd)
        .speed(1.3)
        .build()?;

    let response = openai_client.audio().speech(request).await?;
    let part_path = format!("{}/part_{}.mp3", user_tmp_dir, part_index);
    response.save(&part_path).await?;

    info!("Part {} saved to {}", part_index, part_path);
    Ok(PathBuf::from(part_path))
}

/// Generate a single audio part via ElevenLabs TTS
pub async fn generate_single_part_via_elevenlabs(
    text: &str,
    user_tmp_dir: &str,
    part_index: usize,
) -> anyhow::Result<PathBuf> {
    info!(
        "Generating ElevenLabs TTS for part {}: {} chars",
        part_index,
        text.chars().count()
    );

    let api_key = std::env::var("ELEVEN_LABS_API_KEY")
        .map_err(|_| anyhow::anyhow!("ELEVEN_LABS_API_KEY not found"))?;

    let audio_data = elevenlabs_base_tts(text, &api_key).await?;
    let part_path = format!("{}/part_{}.mp3", user_tmp_dir, part_index);
    fs::write(&part_path, audio_data)?;

    info!("Part {} saved to {}", part_index, part_path);
    Ok(PathBuf::from(part_path))
}

/// Generate a single audio part for via Google Gemini TTS
pub async fn generate_single_part_via_gemini(
    text: &str,
    user_tmp_dir: &str,
    part_index: usize,
) -> anyhow::Result<PathBuf> {
    info!(
        "Generating Google Gemini TTS for part {}: {} chars",
        part_index,
        text.chars().count()
    );

    let api_key =
        std::env::var("GOOGLE_API_KEY").map_err(|_| anyhow::anyhow!("GOOGLE_API_KEY not found"))?;

    let tts_instruction = get_message(AppsSystemMessages::TheViperRoomBot(
        TheViperRoomBotMessages::GeminiTTSInstruction,
    ))
    .await?;

    // Gemini TTS API
    let audio_data = gemini_base_tts(text, &api_key, &tts_instruction).await?;
    let pcm_path = format!("{}/part_{}_pcm.wav", user_tmp_dir, part_index);
    let mp3_path = format!("{}/part_{}.mp3", user_tmp_dir, part_index);

    fs::write(&pcm_path, audio_data)?;
    convert_pcm_to_mp3(&pcm_path, &mp3_path)?;

    if let Err(e) = fs::remove_file(&pcm_path) {
        warn!("Could not delete temporary PCM file {}: {}", pcm_path, e);
    }

    info!("Part {} saved to {}", part_index, mp3_path);
    Ok(PathBuf::from(mp3_path))
}

pub async fn transcribe_voice_message(file_path: &Path) -> anyhow::Result<Option<String>> {
    let transcription = speech_to_text(file_path).await?;

    remove_file(file_path).ok();
    info!("Successfully removed temp file: {:?}", file_path);

    if transcription.trim().is_empty() {
        info!(
            "Voice message transcription is empty, looks like user sent empty message by mistake"
        );
        Ok(None)
    } else {
        Ok(Some(transcription))
    }
}
