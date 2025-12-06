use crate::state::llm_client_init_trait::OpenAIClientInit;
use crate::utils::common::split_text_into_chunks;
use anyhow::anyhow;
use async_openai::types::{CreateSpeechRequestArgs, CreateSpeechResponse, SpeechModel, Voice};
use base64::{engine::general_purpose, Engine as _};
use chrono::{Duration, Utc};
use reqwest::Client as ReqwestClient;
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

pub async fn simple_tts<T: OpenAIClientInit + Send + Sync>(
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

pub async fn speech_to_text(file_path: &Path) -> anyhow::Result<String> {
    let start = Instant::now();

    if !file_path.exists() {
        return Err(anyhow!(
            "Voice message file not found: {}",
            file_path.display()
        ));
    }

    let model_path = std::env::var("WHISPER_MODEL_PATH")
        .unwrap_or_else(|_| "/root/projects/whisper.cpp/models/ggml-base.bin".to_string());

    let output = Command::new("whisper-cli")
        .arg("-m")
        .arg(model_path)
        .arg("-f")
        .arg(file_path)
        .arg("-l")
        .arg("ru")
        .arg("--no-timestamps")
        .output();

    match output {
        Ok(output) if output.status.success() => {
            info!("Transcription took: {:?}", start.elapsed());

            let stdout = String::from_utf8(output.stdout)?;

            if stdout.trim().is_empty() {
                Ok("Empty text".to_string())
            } else {
                Ok(stdout)
            }
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Whisper CLI failed: {}", stderr);
            Err(anyhow!("Whisper CLI failed: {}", stderr))
        }
        Err(err) => {
            error!("Failed to execute Whisper CLI: {}", err);
            Err(anyhow!("Failed to execute Whisper CLI: {}", err))
        }
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

pub async fn podcast_tts_via_google(
    text: String,
    user_tmp_dir: String,
) -> anyhow::Result<PathBuf> {
    info!("Starting Google Gemini TTS podcast recording...");

    let api_key = std::env::var("GOOGLE_API_KEY")
        .map_err(|_| anyhow::anyhow!("GOOGLE_API_KEY not found"))?;

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

        let audio_data = generate_gemini_speech(&text, &api_key).await?;
        let pcm_path = format!("{}/{}_pcm.wav", user_tmp_dir, file_name);
        let mp3_path = format!("{}/{}.mp3", user_tmp_dir, file_name);

        // Save PCM audio
        fs::write(&pcm_path, audio_data)?;

        // Convert PCM to MP3 using ffmpeg
        convert_pcm_to_mp3(&pcm_path, &mp3_path)?;

        // Remove temporary PCM file
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

        let audio_data = generate_gemini_speech(chunk, &api_key).await?;
        let pcm_part_path = format!("{}/part_{}_pcm.wav", user_tmp_dir, i);
        let mp3_part_path = format!("{}/part_{}.mp3", user_tmp_dir, i);

        // Save and convert each chunk
        fs::write(&pcm_part_path, audio_data)?;
        convert_pcm_to_mp3(&pcm_part_path, &mp3_part_path)?;

        // Remove PCM file
        if let Err(e) = fs::remove_file(&pcm_part_path) {
            warn!("Could not delete temporary PCM file {}: {}", pcm_part_path, e);
        }

        audio_parts.push(mp3_part_path);

        // Rate limiting - small delay between requests
        if i < chunks.len() - 1 {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    }

    let final_path = format!("{}/{}.mp3", user_tmp_dir, file_name);

    // Merge all MP3 parts
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

    // Clean up part files
    for part in audio_parts {
        if let Err(e) = fs::remove_file(&part) {
            warn!("Could not delete temporary file {}: {}", part, e);
        }
    }

    info!("Complete podcast successfully generated via Google Gemini API");
    Ok(PathBuf::from(final_path))
}

async fn generate_gemini_speech(text: &str, api_key: &str) -> anyhow::Result<Vec<u8>> {
    let client = ReqwestClient::new();

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash-preview-tts:generateContent?key={}",
        api_key
    );

    let payload = json!({
        "contents": [{
            "role": "user",
            "parts": [{
                "text": text
            }]
        }],
        "generationConfig": {
            "responseModalities": ["AUDIO"],
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

    // Extract base64 audio data from response
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

    // Decode base64 to bytes
    let audio_bytes = general_purpose::STANDARD
        .decode(audio_base64)
        .map_err(|e| anyhow::anyhow!("Failed to decode base64 audio: {}", e))?;

    // Convert L16 PCM to WAV format
    let wav_data = pcm_to_wav(&audio_bytes, 24000, 1, 16)?;

    Ok(wav_data)
}

fn pcm_to_wav(pcm_data: &[u8], sample_rate: u32, channels: u16, bits_per_sample: u16) -> anyhow::Result<Vec<u8>> {
    let mut wav_data = Vec::new();

    // RIFF header
    wav_data.extend_from_slice(b"RIFF");
    let file_size = (36 + pcm_data.len()) as u32;
    wav_data.extend_from_slice(&(file_size - 8).to_le_bytes());
    wav_data.extend_from_slice(b"WAVE");

    // fmt chunk
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

    // data chunk
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
        .arg("-y") // Overwrite output file
        .arg(output_path)
        .status()?;

    if !status.success() {
        return Err(anyhow::anyhow!("Failed to convert PCM to MP3"));
    }

    Ok(())
}
