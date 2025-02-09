use std::fs;
use std::path::Path;
use std::process::Command;

pub fn convert_videos_to_wav() -> Result<(), String> {
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

pub fn convert_video_to_audio(input_video: &Path, output_audio: &Path) -> Result<(), String> {
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

pub fn ensure_directories() -> Result<(), String> {
    let input_path = Path::new("./tmp/input_videos");
    let output_path = Path::new("./tmp/output_audio");

    if !input_path.exists() {
        fs::create_dir_all(input_path).map_err(|e| format!("Failed to create input directory: {}", e))?;
    }
    if !output_path.exists() {
        fs::create_dir_all(output_path).map_err(|e| format!("Failed to create output directory: {}", e))?;
    }
    
    Ok(())
}