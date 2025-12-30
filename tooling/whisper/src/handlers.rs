use anyhow::{anyhow, Result};
use axum::extract::Multipart;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;
use tempfile::NamedTempFile;
use tracing::{error, info};

#[derive(Serialize, Deserialize)]
pub struct TranscribeResponse {
    pub text: String,
    pub duration_ms: u128,
}

#[derive(Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// Handler for POST /transcribe
/// Accepts multipart/form-data with an audio file
/// Returns JSON with transcribed text
pub async fn handle_transcribe(
    mut multipart: Multipart,
) -> Result<Json<TranscribeResponse>, (StatusCode, Json<ErrorResponse>)> {
    let start = Instant::now();

    // Extract audio file from multipart
    let audio_data = match extract_audio_from_multipart(&mut multipart).await {
        Ok(data) => data,
        Err(e) => {
            error!("Failed to extract audio: {}", e);
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: format!("Failed to extract audio: {}", e),
                }),
            ));
        }
    };

    // Convert to WAV format
    let wav_path = match convert_to_wav(&audio_data).await {
        Ok(path) => path,
        Err(e) => {
            error!("Failed to convert to WAV: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to convert audio: {}", e),
                }),
            ));
        }
    };

    // Transcribe using whisper
    let transcription = match transcribe_audio(&wav_path).await {
        Ok(text) => text,
        Err(e) => {
            error!("Failed to transcribe: {}", e);
            // Cleanup temp file
            let _ = fs::remove_file(&wav_path);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Transcription failed: {}", e),
                }),
            ));
        }
    };

    // Cleanup temp file
    if let Err(e) = fs::remove_file(&wav_path) {
        error!("Failed to remove temp file: {}", e);
    }

    let duration_ms = start.elapsed().as_millis();

    info!("Transcription completed in {}ms", duration_ms);

    Ok(Json(TranscribeResponse {
        text: transcription,
        duration_ms,
    }))
}

/// Extract audio data from multipart form
async fn extract_audio_from_multipart(multipart: &mut Multipart) -> Result<Vec<u8>> {
    while let Some(field) = multipart.next_field().await? {
        let name = field.name().unwrap_or("").to_string();

        if name == "audio" || name == "file" {
            let data = field.bytes().await?;
            info!("Received audio file: {} bytes", data.len());
            return Ok(data.to_vec());
        }
    }

    Err(anyhow!("No audio file found in request"))
}

/// Convert audio to WAV format using ffmpeg
async fn convert_to_wav(audio_data: &[u8]) -> Result<PathBuf> {
    // Create temp file for input audio
    let mut input_file = NamedTempFile::new()?;
    std::io::Write::write_all(&mut input_file, audio_data)?;
    let input_path = input_file.path();

    // Create temp file for output WAV
    let output_file = NamedTempFile::new()?;
    let output_path = output_file.path().with_extension("wav");

    info!(
        "Converting audio: {} -> {}",
        input_path.display(),
        output_path.display()
    );

    let output = Command::new("ffmpeg")
        .arg("-i")
        .arg(input_path)
        .arg("-ar")
        .arg("16000")
        .arg("-y")
        .arg(&output_path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("FFmpeg conversion failed: {}", stderr));
    }

    Ok(output_path)
}

/// Transcribe audio using whisper-cli
async fn transcribe_audio(wav_path: &PathBuf) -> Result<String> {
    let model_path = std::env::var("WHISPER_MODEL_PATH")
        .unwrap_or_else(|_| "/app/whisper.cpp/models/ggml-base.bin".to_string());

    info!("Using whisper model: {}", model_path);

    let output = Command::new("whisper-cli")
        .arg("-m")
        .arg(model_path)
        .arg("-f")
        .arg(wav_path)
        .arg("-l")
        .arg("ru")
        .arg("--no-timestamps")
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Whisper CLI failed: {}", stderr));
    }

    let stdout = String::from_utf8(output.stdout)?;

    if stdout.trim().is_empty() {
        Ok("Empty transcription".to_string())
    } else {
        Ok(stdout.trim().to_string())
    }
}
