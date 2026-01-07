use crate::ai::common::voice_processing;
use crate::ai::common::voice_processing::{
    convert_pcm_to_mp3, elevenlabs_base_tts, gemini_base_tts, openai_base_tts,
};
use crate::models::common::system_messages::{AppsSystemMessages, TheViperRoomBotMessages};
use crate::state::llm_client_init_trait::OpenAIClientInit;
use crate::utils::common::{get_message, split_text_into_chunks};
use chrono::{Duration, Utc};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use tracing::{error, info, warn};

pub async fn podcast_voiceover_via_openai<T: OpenAIClientInit + Send + Sync>(
    text: String,
    user_tmp_dir: String,
    app_state: Arc<T>,
) -> anyhow::Result<PathBuf> {
    info!("Starting recording podcast via OpenAI TTS model...");

    let now = Utc::now();
    let utc_plus_3 = now + Duration::hours(3);
    let date_only = utc_plus_3.date_naive();
    let file_name = format!("The_Viper_Podcast_({})", date_only);

    const MAX_TTS_CHARS: usize = 4095;

    let char_count = text.chars().count();

    if char_count <= MAX_TTS_CHARS {
        let char_count = text.chars().count();
        info!(
            "Podcast text length is: {} characters. There is not need to split text into chunks.",
            char_count
        );

        let response = openai_base_tts(&text, app_state.clone(), 1.3).await?;
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

        let response = openai_base_tts(&text, app_state.clone(), 1.3).await?;
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
                    match voice_processing::generate_single_part_via_gemini(
                        &part_text, &tmp_dir, part_index,
                    )
                    .await
                    {
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

        let audio_data = elevenlabs_base_tts(&text, &api_key).await?;
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

        let audio_data = elevenlabs_base_tts(chunk, &api_key).await?;
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
        let audio_data = gemini_base_tts(&text, &api_key, &tts_instruction).await?;
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
        let audio_data = gemini_base_tts(chunk, &api_key, &tts_instruction).await?;
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
