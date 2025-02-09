use std::process::Command;
use std::path::{Path, PathBuf};
use std::fs;
use core::ai::common::voice_processing::speech_to_text;

fn main() {
    // if let Err(e) = convert_videos_to_wav() {
    //     eprintln!("Error: {}", e);
    // }
    if let Err(e) = transcribe_wav_files() {
        eprintln!("Error: {}", e);
    }
}

fn convert_videos_to_wav() -> Result<(), String> {
    ensure_directories()?;
    let input_videos_dir = "./tmp/input_videos";
    let output_audio_dir = "./tmp/output_audio";

    let mut has_files = false;
    for entry in fs::read_dir(input_videos_dir).map_err(|e| format!("Failed to read input directory: {}", e))? {
        let entry = entry.map_err(|e| format!("Failed to read file entry: {}", e))?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "mp4" || ext == "mkv" || ext == "avi" {
                    has_files = true;
                    let output_audio = Path::new(output_audio_dir).join(
                        path.file_stem().unwrap().to_string_lossy().to_string() + ".wav"
                    );
                    println!("Processing video: {} -> {}", path.display(), output_audio.display());
                    if let Err(e) = convert_video_to_audio(&path, &output_audio) {
                        eprintln!("Error processing {}: {}", path.display(), e);
                    }
                }
            }
        }
    }
    if !has_files {
        return Err("No video files found in input directory".to_string());
    }
    Ok(())
}

fn convert_video_to_audio(input_video: &Path, output_audio: &Path) -> Result<(), String> {
    let ffmpeg_command = Command::new("ffmpeg")
        .args(["-i", input_video.to_str().unwrap(), "-ac", "1", "-ar", "16000", "-f", "wav", output_audio.to_str().unwrap()])
        .output();

    match ffmpeg_command {
        Ok(output) => {
            if output.status.success() {
                Ok(())
            } else {
                Err(format!("FFmpeg error: {}", String::from_utf8_lossy(&output.stderr)))
            }
        }
        Err(e) => Err(format!("Failed to execute FFmpeg: {}", e)),
    }
}

fn ensure_directories() -> Result<(), String> {
    let input_path = Path::new("./tmp/input_videos");
    let output_path = Path::new("./tmp/output_audio");
    let output_texts_dir = Path::new("./tmp/output_text");

    if !input_path.exists() {
        fs::create_dir_all(input_path).map_err(|e| format!("Failed to create input directory: {}", e))?;
    }
    if !output_path.exists() {
        fs::create_dir_all(output_path).map_err(|e| format!("Failed to create output directory: {}", e))?;
    }
    if !output_texts_dir.exists() {
        fs::create_dir_all(output_texts_dir).map_err(|e| format!("Failed to create output directory: {}", e))?;
    }
    Ok(())
}

fn transcribe_wav_files() -> Result<(), String> {
    let input_audio_dir = Path::new("./tmp/output_audio");
    let output_texts_dir = Path::new("./tmp/output_text");

    if !output_texts_dir.exists() {
        fs::create_dir_all(output_texts_dir).map_err(|e| format!("Failed to create output directory: {}", e))?;
    }

    let mut has_files = false;
    for entry in fs::read_dir(input_audio_dir).map_err(|e| format!("Failed to read output audio directory: {}", e))? {
        let entry = entry.map_err(|e| format!("Failed to read file entry: {}", e))?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "wav" {
                    has_files = true;
                    let output_text = output_texts_dir.join(
                        path.file_stem().unwrap().to_string_lossy().to_string() + ".txt"
                    );
                    println!("Transcribing file: {} -> {}", path.display(), output_text.display());
                    if let Err(e) = tokio::runtime::Runtime::new().unwrap().block_on(transcribe_wav_to_txt(&path, &output_text)) {
                        eprintln!("Error transcribing {}: {}", path.display(), e);
                    }
                }
            }
        }
    }
    if !has_files {
        return Err("No WAV files found in output_audio directory".to_string());
    }
    Ok(())
}

async fn transcribe_wav_to_txt(input_wav: &Path, output_txt: &Path) -> Result<(), String> {
    if !input_wav.exists() {
        return Err(format!("WAV file not found: {}", input_wav.display()));
    }

    let chunk_dir = input_wav.with_file_name("chunks");
    let chunks = split_audio(input_wav, &chunk_dir)?;

    let mut transcription_result = String::new();

    for chunk in &chunks {
        println!("Transcribing chunk: {}", chunk.display());

        match speech_to_text(chunk).await {
            Ok(transcription) => {
                if !transcription.trim().is_empty() {
                    transcription_result.push_str(&transcription);
                    transcription_result.push('\n');
                } else {
                    println!("Empty transcription for chunk: {}", chunk.display());
                }
            }
            Err(e) => {
                eprintln!("Failed to transcribe {}: {}", chunk.display(), e);
            }
        }
    }

    if transcription_result.trim().is_empty() {
        return Err("Final transcription is empty.".to_string());
    }

    if let Err(e) = fs::write(output_txt, &transcription_result) {
        return Err(format!("Failed to write transcription to {}: {}", output_txt.display(), e));
    }

    println!("Successfully transcribed full audio: {} -> {}", input_wav.display(), output_txt.display());
    Ok(())
}

fn split_audio(input_wav: &Path, output_dir: &Path) -> Result<Vec<PathBuf>, String> {
    if !output_dir.exists() {
        fs::create_dir_all(output_dir).map_err(|e| format!("Failed to create chunk directory: {}", e))?;
    }

    let chunk_pattern = output_dir.join("chunk_%03d.wav");

    let status = Command::new("ffmpeg")
        .args([
            "-i", input_wav.to_str().unwrap(),
            "-f", "segment",
            "-segment_time", "30",
            "-c", "copy",
            chunk_pattern.to_str().unwrap(),
        ])
        .status()
        .map_err(|e| format!("Failed to execute FFmpeg: {}", e))?;

    if !status.success() {
        return Err("FFmpeg failed to split audio".to_string());
    }

    let mut chunks = Vec::new();
    for entry in fs::read_dir(output_dir).map_err(|e| format!("Failed to read chunk directory: {}", e))? {
        let path = entry.map_err(|e| format!("Failed to read chunk entry: {}", e))?.path();
        if path.extension().unwrap_or_default() == "wav" {
            chunks.push(path);
        }
    }
    chunks.sort();
    Ok(chunks)
}
