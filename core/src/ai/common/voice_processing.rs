use crate::models::common::system_messages::{AppsSystemMessages, TheViperRoomBotMessages};
use crate::state::llm_client_init_trait::OpenAIClientInit;
use crate::utils::common::{get_message, split_text_into_chunks};
use anyhow::anyhow;
use async_openai::types::{CreateSpeechRequestArgs, CreateSpeechResponse, SpeechModel, Voice};
use base64::{engine::general_purpose, Engine as _};
use chrono::{Duration, Utc};
use reqwest::multipart;
use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Instant;
use tracing::{error, info, warn};

pub async fn podcast_tts_via_openai<T: OpenAIClientInit + Send + Sync>(
    text: String,
    user_tmp_dir: String,
    app_state: Arc<T>,
) -> anyhow::Result<PathBuf> {
    info!("Starting recording podcast via OpenAI TTS model...");

    let now = Utc::now();
    let utc_plus_3 = now + Duration::hours(3);
    let date_only = utc_plus_3.date_naive();
    let file_name = format!("The_Viper_Podcast_({})", date_only);

    let llm_client = app_state.get_llm_client().clone();

    const MAX_TTS_CHARS: usize = 4095;

    let char_count = text.chars().count();

    if char_count <= MAX_TTS_CHARS {
        let char_count = text.chars().count();
        info!(
            "Podcast text length is: {} characters. There is not need to split text into chunks.",
            char_count
        );
        let request = CreateSpeechRequestArgs::default()
            .input(&text)
            .voice(Voice::Onyx)
            .model(SpeechModel::Tts1Hd)
            // .model(SpeechModel::Other(LlmModel::TTS.to_string()))
            .speed(1.3)
            .build()?;

        info!("Starting OpenAI voice generation...");
        let response = llm_client.audio().speech(request).await?;
        let audio_file_path = format!("{}/{}.mp3", user_tmp_dir, file_name);
        response.save(&audio_file_path).await?;

        info!("Podcast generated as single file");
        return Ok(PathBuf::from(audio_file_path));
    }

    let chunks = split_text_into_chunks(&text, MAX_TTS_CHARS);
    info!("Text split into {} chunks", chunks.len());

    let mut audio_parts = Vec::new();

    for (i, chunk) in chunks.iter().enumerate() {
        info!(
            "Processing podcast chunk: {}/{}, podcast length: {} chars",
            i + 1,
            chunks.len(),
            chunk.chars().count()
        );

        let request = CreateSpeechRequestArgs::default()
            .input(chunk)
            .voice(Voice::Onyx)
            .model(SpeechModel::Tts1Hd)
            // .model(SpeechModel::Other(LlmModel::TTS.to_string()))
            .speed(1.3)
            .build()?;

        let response = llm_client.audio().speech(request).await?;
        let part_path = format!("{}/part_{}.mp3", user_tmp_dir, i);
        response.save(&part_path).await?;
        audio_parts.push(part_path);
    }

    let final_path = format!("{}/{}.mp3", user_tmp_dir, file_name);

    let mut command = Command::new("ffmpeg");
    command
        .arg("-i")
        .arg(format!("concat:{}", audio_parts.join("|")))
        .arg("-acodec")
        .arg("copy")
        .arg(&final_path);

    let status = command.status()?;
    if !status.success() {
        return Err(anyhow::anyhow!("Failed to merge audio files"));
    }

    for part in audio_parts {
        if let Err(e) = fs::remove_file(&part) {
            warn!("Could not delete temporary file {}: {}", part, e);
        }
    }

    info!("Complete podcast successfully generated via OpenAI API");

    Ok(PathBuf::from(final_path))
}

// pub async fn podcast_tts_via_openai_new<T: OpenAIClientInit + Send + Sync>(
//     text: String,
//     user_tmp_dir: String,
//     app_state: Arc<T>,
// ) -> anyhow::Result<PathBuf> {
//     info!("Starting recording podcast via OpenAI TTS model...");
//
//     let now = Utc::now();
//     let utc_plus_3 = now + Duration::hours(3);
//     let date_only = utc_plus_3.date_naive();
//     let file_name = format!("The_Viper_Podcast_({})", date_only);
//
//     let llm_client = app_state.get_llm_client().clone();
//
//     const MAX_TTS_CHARS: usize = 4095;
//
//     let char_count = text.chars().count();
//
//     if char_count <= MAX_TTS_CHARS {
//         let char_count = text.chars().count();
//         info!(
//             "Podcast text length is: {} characters. There is not need to split text into chunks.",
//             char_count
//         );
//         let request = CreateSpeechRequestArgs::default()
//             .input(&text)
//             .voice(Voice::Onyx)
//             .model(SpeechModel::Other(LlmModel::TTS.to_string()))
//             .speed(1.3)
//             .build()?;
//
//         info!("Starting OpenAI voice generation...");
//         let response = llm_client.audio().speech(request).await?;
//         let audio_file_path = format!("{}/{}.mp3", user_tmp_dir, file_name);
//         response.save(&audio_file_path).await?;
//
//         info!("Podcast generated as single file");
//         return Ok(PathBuf::from(audio_file_path));
//     }
//
//     let chunks = split_text_into_chunks(&text, MAX_TTS_CHARS);
//     info!("Text split into {} chunks", chunks.len());
//
//     let mut audio_parts = Vec::new();
//
//     for (i, chunk) in chunks.iter().enumerate() {
//         info!(
//             "Processing podcast chunk: {}/{}, podcast length: {} chars",
//             i + 1,
//             chunks.len(),
//             chunk.chars().count()
//         );
//
//         let request = CreateSpeechRequestArgs::default()
//             .input(chunk)
//             .voice(Voice::Onyx)
//             .model(SpeechModel::Other(LlmModel::TTS.to_string()))
//             .speed(1.3)
//             .build()?;
//
//         let response = llm_client.audio().speech(request).await?;
//         let part_path = format!("{}/part_{}.mp3", user_tmp_dir, i);
//         response.save(&part_path).await?;
//         audio_parts.push(part_path);
//     }
//
//     let final_path = format!("{}/{}.mp3", user_tmp_dir, file_name);
//
//     let mut command = Command::new("ffmpeg");
//     command
//         .arg("-i")
//         .arg(format!("concat:{}", audio_parts.join("|")))
//         .arg("-acodec")
//         .arg("copy")
//         .arg(&final_path);
//
//     let status = command.status()?;
//     if !status.success() {
//         return Err(anyhow::anyhow!("Failed to merge audio files"));
//     }
//
//     for part in audio_parts {
//         if let Err(e) = fs::remove_file(&part) {
//             warn!("Could not delete temporary file {}: {}", part, e);
//         }
//     }
//
//     info!("Complete podcast successfully generated via OpenAI API");
//
//     Ok(PathBuf::from(final_path))
// }

pub async fn simple_openai_tts<T: OpenAIClientInit + Send + Sync>(
    text: &str,
    app_state: Arc<T>,
) -> anyhow::Result<CreateSpeechResponse> {
    let llm_client = app_state.get_llm_client().clone();

    let request = CreateSpeechRequestArgs::default()
        .input(text)
        .voice(Voice::Onyx)
        .model(SpeechModel::Tts1Hd)
        .speed(1.3)
        .build()?;

    let response = llm_client.audio().speech(request).await?;

    Ok(response)
}

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

pub async fn podcast_tts_via_elevenlabs(
    text: String,
    user_tmp_dir: String,
) -> anyhow::Result<PathBuf> {
    info!("Starting ElevenLabs podcast recording via ElevenLabs TTS model...");

    let api_key = std::env::var("ELEVEN_LABS_API_KEY")
        .map_err(|_| anyhow::anyhow!("ELEVEN_LABS_API_KEY not found"))?;

    let now = Utc::now();
    let utc_plus_3 = now + Duration::hours(3);
    let date_only = utc_plus_3.date_naive();
    let file_name = format!("The_Viper_Podcast_({})", date_only);
    const ELEVEN_LABS_MAX_CHARS: usize = 9900;

    let char_count = text.chars().count();

    if char_count <= ELEVEN_LABS_MAX_CHARS {
        info!(
            "Podcast text length: {} chars. Generating single file.",
            char_count
        );

        let audio_data = generate_elevenlabs_speech(&text, &api_key).await?;
        let audio_file_path = format!("{}/{}.mp3", user_tmp_dir, file_name);
        fs::write(&audio_file_path, audio_data)?;

        info!("Podcast generated as single file");
        return Ok(PathBuf::from(audio_file_path));
    }

    let chunks = split_text_into_chunks(&text, ELEVEN_LABS_MAX_CHARS);
    info!("Text split into {} chunks", chunks.len());

    let mut audio_parts = Vec::new();

    for (i, chunk) in chunks.iter().enumerate() {
        info!(
            "Processing chunk {}/{}, length: {} chars",
            i + 1,
            chunks.len(),
            chunk.chars().count()
        );

        let audio_data = generate_elevenlabs_speech(chunk, &api_key).await?;
        let part_path = format!("{}/part_{}.mp3", user_tmp_dir, i);
        fs::write(&part_path, audio_data)?;
        audio_parts.push(part_path);

        if i < chunks.len() - 1 {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    }

    let final_path = format!("{}/{}.mp3", user_tmp_dir, file_name);

    let mut command = Command::new("ffmpeg");
    command
        .arg("-i")
        .arg(format!("concat:{}", audio_parts.join("|")))
        .arg("-acodec")
        .arg("copy")
        .arg(&final_path);

    let status = command.status()?;
    if !status.success() {
        return Err(anyhow::anyhow!("Failed to merge audio files"));
    }

    for part in audio_parts {
        if let Err(e) = fs::remove_file(&part) {
            warn!("Could not delete temporary file {}: {}", part, e);
        }
    }

    info!("Complete podcast successfully generated via ElevenLabs API");
    Ok(PathBuf::from(final_path))
}

async fn generate_elevenlabs_speech(text: &str, api_key: &str) -> anyhow::Result<Vec<u8>> {
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

pub async fn podcast_tts_via_google(text: String, user_tmp_dir: String) -> anyhow::Result<PathBuf> {
    info!("Starting Google Gemini TTS podcast recording...");

    let api_key =
        std::env::var("GOOGLE_API_KEY").map_err(|_| anyhow::anyhow!("GOOGLE_API_KEY not found"))?;

    let tts_instruction = get_message(AppsSystemMessages::TheViperRoomBot(
        TheViperRoomBotMessages::GeminiTTSInstruction,
    ))
    .await?;

    let now = Utc::now();
    let utc_plus_3 = now + Duration::hours(3);
    let date_only = utc_plus_3.date_naive();
    let file_name = format!("The_Viper_Podcast_({})", date_only);

    const GEMINI_MAX_CHARS: usize = 24000;

    let char_count = text.chars().count();

    if char_count <= GEMINI_MAX_CHARS {
        info!(
            "Podcast text length: {} chars. Generating single file.",
            char_count
        );

        // Gemini TTS API
        let audio_data = generate_gemini_speech(&text, &api_key, &tts_instruction).await?;
        // Google Cloud TTS API
        // let audio_data = generate_google_cloud_speech(&text, &api_key).await?;
        let pcm_path = format!("{}/{}_pcm.wav", user_tmp_dir, file_name);
        let mp3_path = format!("{}/{}.mp3", user_tmp_dir, file_name);

        fs::write(&pcm_path, audio_data)?;

        convert_pcm_to_mp3(&pcm_path, &mp3_path)?;

        if let Err(e) = fs::remove_file(&pcm_path) {
            warn!("Could not delete temporary PCM file {}: {}", pcm_path, e);
        }

        info!("Podcast generated as single file");
        return Ok(PathBuf::from(mp3_path));
    }

    let chunks = split_text_into_chunks(&text, GEMINI_MAX_CHARS);
    info!("Text split into {} chunks", chunks.len());

    let mut audio_parts = Vec::new();

    for (i, chunk) in chunks.iter().enumerate() {
        info!(
            "Processing chunk {}/{}, length: {} chars",
            i + 1,
            chunks.len(),
            chunk.chars().count()
        );

        // Gemini TTS API
        let audio_data = generate_gemini_speech(chunk, &api_key, &tts_instruction).await?;
        // Google Cloud TTS API
        // let audio_data = generate_google_cloud_speech(chunk, &api_key).await?;
        let pcm_part_path = format!("{}/part_{}_pcm.wav", user_tmp_dir, i);
        let mp3_part_path = format!("{}/part_{}.mp3", user_tmp_dir, i);

        fs::write(&pcm_part_path, audio_data)?;
        convert_pcm_to_mp3(&pcm_part_path, &mp3_part_path)?;

        if let Err(e) = fs::remove_file(&pcm_part_path) {
            warn!(
                "Could not delete temporary PCM file {}: {}",
                pcm_part_path, e
            );
        }

        audio_parts.push(mp3_part_path);

        if i < chunks.len() - 1 {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    }

    let final_path = format!("{}/{}.mp3", user_tmp_dir, file_name);

    let mut command = Command::new("ffmpeg");
    command
        .arg("-i")
        .arg(format!("concat:{}", audio_parts.join("|")))
        .arg("-acodec")
        .arg("copy")
        .arg(&final_path);

    let status = command.status()?;
    if !status.success() {
        return Err(anyhow::anyhow!("Failed to merge audio files"));
    }

    for part in audio_parts {
        if let Err(e) = fs::remove_file(&part) {
            warn!("Could not delete temporary file {}: {}", part, e);
        }
    }

    info!("Complete podcast successfully generated via Google Gemini API");
    Ok(PathBuf::from(final_path))
}

async fn generate_gemini_speech(
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

fn convert_pcm_to_mp3(input_path: &str, output_path: &str) -> anyhow::Result<()> {
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

/// Generate a single audio part for OpenAI TTS
pub async fn generate_single_part_openai<T: OpenAIClientInit + Send + Sync>(
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

    let llm_client = app_state.get_llm_client().clone();

    let request = CreateSpeechRequestArgs::default()
        .input(text)
        .voice(Voice::Onyx)
        .model(SpeechModel::Tts1Hd)
        .speed(1.3)
        .build()?;

    let response = llm_client.audio().speech(request).await?;
    let part_path = format!("{}/part_{}.mp3", user_tmp_dir, part_index);
    response.save(&part_path).await?;

    info!("Part {} saved to {}", part_index, part_path);
    Ok(PathBuf::from(part_path))
}

/// Generate a single audio part for ElevenLabs TTS
pub async fn generate_single_part_elevenlabs(
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

    let audio_data = generate_elevenlabs_speech(text, &api_key).await?;
    let part_path = format!("{}/part_{}.mp3", user_tmp_dir, part_index);
    fs::write(&part_path, audio_data)?;

    info!("Part {} saved to {}", part_index, part_path);
    Ok(PathBuf::from(part_path))
}

/// Generate a single audio part for Google Gemini TTS
pub async fn generate_single_part_google(
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
    let audio_data = generate_gemini_speech(text, &api_key, &tts_instruction).await?;
    // Google Cloud TTS API
    // let audio_data = generate_google_cloud_speech(text, &api_key).await?;
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

pub async fn generate_parts_batched_google(
    parts: &[String],
    user_tmp_dir: &str,
) -> anyhow::Result<Vec<PathBuf>> {
    const BATCH_SIZE: usize = 9;
    const MAX_RETRIES: usize = 3;
    const BATCH_DELAY_SECS: u64 = 60;

    info!(
        "Starting batched Google TTS generation for {} parts (batch size: {}, max retries: {})",
        parts.len(),
        BATCH_SIZE,
        MAX_RETRIES
    );

    let mut all_audio_parts = Vec::new();

    for (batch_idx, batch) in parts.chunks(BATCH_SIZE).enumerate() {
        let batch_start_idx = batch_idx * BATCH_SIZE;
        info!(
            "Processing batch {}/{}: parts {}-{}",
            batch_idx + 1,
            (parts.len() + BATCH_SIZE - 1) / BATCH_SIZE,
            batch_start_idx,
            batch_start_idx + batch.len() - 1
        );

        let mut tasks = Vec::new();
        for (i, part) in batch.iter().enumerate() {
            let part_index = batch_start_idx + i;
            let part_text = part.clone();
            let tmp_dir = user_tmp_dir.to_string();

            let task = tokio::spawn(async move {
                let mut last_error = None;

                for attempt in 1..=MAX_RETRIES {
                    match generate_single_part_google(&part_text, &tmp_dir, part_index).await {
                        Ok(path) => {
                            if attempt > 1 {
                                info!(
                                    "Part {} succeeded on attempt {}/{}",
                                    part_index, attempt, MAX_RETRIES
                                );
                            }
                            return Ok((part_index, path));
                        }
                        Err(e) => {
                            last_error = Some(e);
                            if attempt < MAX_RETRIES {
                                warn!(
                                    "Part {} failed on attempt {}/{}: {}. Retrying...",
                                    part_index,
                                    attempt,
                                    MAX_RETRIES,
                                    last_error.as_ref().unwrap()
                                );
                                tokio::time::sleep(tokio::time::Duration::from_secs(
                                    2u64.pow(attempt as u32),
                                ))
                                .await;
                            } else {
                                error!("Part {} failed after {} attempts", part_index, MAX_RETRIES);
                            }
                        }
                    }
                }

                Err(anyhow::anyhow!(
                    "Part {} failed after {} retries: {}",
                    part_index,
                    MAX_RETRIES,
                    last_error.unwrap()
                ))
            });

            tasks.push(task);
        }

        let results = futures::future::join_all(tasks).await;

        let mut batch_results: Vec<(usize, PathBuf)> = Vec::new();
        for result in results {
            match result {
                Ok(Ok(part_result)) => {
                    batch_results.push(part_result);
                }
                Ok(Err(e)) => {
                    error!("Task returned error: {}", e);
                    return Err(e);
                }
                Err(e) => {
                    error!("Task panicked: {}", e);
                    return Err(anyhow::anyhow!("Task panicked: {}", e));
                }
            }
        }

        batch_results.sort_by_key(|(idx, _)| *idx);

        for (_, path) in batch_results {
            all_audio_parts.push(path);
        }

        info!("Batch {} completed successfully", batch_idx + 1);

        if batch_idx < (parts.len() + BATCH_SIZE - 1) / BATCH_SIZE - 1 {
            info!(
                "Waiting {} seconds before next batch to respect rate limits...",
                BATCH_DELAY_SECS
            );
            tokio::time::sleep(tokio::time::Duration::from_secs(BATCH_DELAY_SECS)).await;
        }
    }

    info!(
        "All {} parts generated successfully with batched approach",
        parts.len()
    );
    Ok(all_audio_parts)
}

pub async fn merge_audio_parts(
    audio_parts: Vec<PathBuf>,
    user_tmp_dir: &str,
    final_filename: &str,
) -> anyhow::Result<PathBuf> {
    info!(
        "Merging {} audio parts into final podcast",
        audio_parts.len()
    );

    if audio_parts.is_empty() {
        return Err(anyhow::anyhow!("No audio parts to merge"));
    }

    if audio_parts.len() == 1 {
        info!("Only one part, no merging needed");
        return Ok(audio_parts[0].clone());
    }

    let final_path = format!("{}/{}.mp3", user_tmp_dir, final_filename);

    const PAUSE_DURATION_SEC: f32 = 1.5;

    let mut filter_parts = Vec::new();
    let mut input_args = Vec::new();

    for (idx, part) in audio_parts.iter().enumerate() {
        input_args.push("-i".to_string());
        input_args.push(part.to_string_lossy().to_string());
        filter_parts.push(format!("[{}:a]", idx));
    }

    let silence_filter = format!(
        "anullsrc=channel_layout=stereo:sample_rate=44100:duration={}",
        PAUSE_DURATION_SEC
    );

    let mut concat_inputs = Vec::new();
    for (idx, _) in audio_parts.iter().enumerate() {
        concat_inputs.push(format!("[{}:a]", idx));
        if idx < audio_parts.len() - 1 {
            concat_inputs.push(format!("[silence{}]", idx));
        }
    }

    let mut silence_streams = Vec::new();
    for idx in 0..audio_parts.len() - 1 {
        silence_streams.push(format!("{}[silence{}]", silence_filter, idx));
    }

    let filter_complex = if silence_streams.is_empty() {
        format!(
            "{}[s0];{}concat=n=3:v=0:a=1[out]",
            silence_filter,
            concat_inputs.join("")
        )
    } else {
        format!(
            "{};{}concat=n={}:v=0:a=1[out]",
            silence_streams.join(";"),
            concat_inputs.join(""),
            audio_parts.len() * 2 - 1
        )
    };

    info!(
        "Merging {} parts with {} sec pauses between them",
        audio_parts.len(),
        PAUSE_DURATION_SEC
    );

    let mut command = Command::new("ffmpeg");
    for arg in input_args {
        command.arg(arg);
    }

    command
        .arg("-filter_complex")
        .arg(&filter_complex)
        .arg("-map")
        .arg("[out]")
        .arg("-codec:a")
        .arg("libmp3lame")
        .arg("-q:a")
        .arg("2")
        .arg("-y")
        .arg(&final_path);

    let status = command.status()?;
    if !status.success() {
        return Err(anyhow::anyhow!("Failed to merge audio files"));
    }

    for part in audio_parts {
        if let Err(e) = fs::remove_file(&part) {
            warn!("Could not delete temporary file {:?}: {}", part, e);
        }
    }

    info!("Audio parts merged successfully into {}", final_path);
    Ok(PathBuf::from(final_path))
}
