use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use tracing::{error, info, warn};
use unicode_normalization::UnicodeNormalization;
use walkdir::WalkDir;

/// The function validates the data of incoming json files by comparing the values for the keys 
/// of  reference json structure and the json structures of incoming json files.
/// 
/// * normalize_structure_file() fn normalize data in the reference json structure
/// 
/// * apply_normalization flag is used to enable and disable the need for data normalization 
/// before validation
///
pub async fn validate_input_data() -> Result<()> {
    let file_path = "./common_res/w3a/w3a_learning_structure.txt";
    let file_content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read file: {}", file_path))?;

    let json_data: Value =
        serde_json::from_str(&file_content).with_context(|| "Failed to parse JSON structure")?;

    let mut valid_module_titles = HashSet::new();
    let mut valid_block_titles = HashSet::new();
    let mut valid_lesson_titles = HashSet::new();

    if let Value::Object(modules) = json_data {
        for (module, blocks) in modules {
            valid_module_titles.insert(module.nfc().collect::<String>());

            if let Value::Object(blocks_map) = blocks {
                for (block_title, lessons) in blocks_map {
                    valid_block_titles.insert(block_title.nfc().collect::<String>());

                    if let Value::Array(lessons_list) = lessons {
                        for lesson in lessons_list {
                            if let Value::String(lesson_title) = lesson {
                                valid_lesson_titles.insert(lesson_title.nfc().collect::<String>());
                            }
                        }
                    }
                }
            }
        }
    }

    // info!(
    //     "Valid Module Title are: {:?}, module titles amount: {}",
    //     valid_module_titles,
    //     valid_module_titles.len()
    // );
    // info!(
    //     "Valid Block Titles are: {:?}, block titles amount: {}",
    //     valid_block_titles,
    //     valid_block_titles.len()
    // );
    // info!(
    //     "Valid Lesson Titles are: {:?}, lesson titles amount: {}",
    //     valid_lesson_titles,
    //     valid_lesson_titles.len()
    // );

    // match normalize_structure_file() {
    //     Ok(_) => info!("W3A learning structure file normalized successfully!"),
    //     Err(e) => error!(
    //         "Error during normalization W3A learning structure file: {}",
    //         e
    //     ),
    // }

    match normalize_and_validate_input_jsons(
        &valid_module_titles,
        &valid_block_titles,
        &valid_lesson_titles,
        true
    )
    .await
    {
        Ok(_) => info!("Input JSON files validated successfully!"),
        Err(e) => error!("Error during validation input W3A files: {}", e),
    }

    Ok(())
}

pub async fn normalize_and_validate_input_jsons(
    module_titles: &HashSet<String>,
    block_titles: &HashSet<String>,
    lesson_titles: &HashSet<String>,
    apply_normalization: bool,
) -> Result<(), String> {
    let input_dir = Path::new("./tools/tmp/input_jsons");
    if !input_dir.exists() {
        fs::create_dir_all(input_dir)
            .map_err(|e| format!("Error making dir {:?}: {}", input_dir, e))?;
        warn!("Input data dir created: {:?}", input_dir);
    }

    let mut files_processed = 0;
    let mut files_modified = 0;
    let mut processed_lessons = HashSet::new();

    for entry in WalkDir::new(input_dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        if !path.is_file() || path.extension().unwrap_or_default() != "json" {
            continue;
        }

        files_processed += 1;
        let file_content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let mut json: Value = serde_json::from_str(&file_content).map_err(|e| e.to_string())?;

        let module = json["module"].as_str().unwrap_or("").to_string();
        let block_title = json["block_title"].as_str().unwrap_or("").to_string();
        let lesson_title = json["lesson_title"].as_str().unwrap_or("").to_string();

        let (normalized_module, normalized_block_title, normalized_lesson_title) =
            if apply_normalization {
                normalize_titles(&module, &block_title, &lesson_title)
            } else {
                (module.clone(), block_title.clone(), lesson_title.clone())
            };

        let module_changed = module != normalized_module;
        let block_changed = block_title != normalized_block_title;
        let lesson_changed = lesson_title != normalized_lesson_title;

        let needs_modification = module_changed || block_changed || lesson_changed;

        if apply_normalization && needs_modification {
            files_modified += 1;
            info!("Modifying file: {:?}", path);

            if module_changed {
                info!("Module changed:");
                info!("  Old module: '{}'", module);
                info!("  New module: '{}'", normalized_module);
            }

            if block_changed {
                info!("Block title changed:");
                info!("  Old block_title: '{}'", block_title);
                info!("  New block_title: '{}'", normalized_block_title);
            }

            if lesson_changed {
                info!("Lesson title changed:");
                info!("  Old lesson_title: '{}'", lesson_title);
                info!("  New lesson_title: '{}'", normalized_lesson_title);
            }

            json["module"] = json!(normalized_module);
            json["block_title"] = json!(normalized_block_title);
            json["lesson_title"] = json!(normalized_lesson_title);

            let formatted_json = serde_json::to_string_pretty(&json).map_err(|e| e.to_string())?;
            fs::write(path, formatted_json).map_err(|e| e.to_string())?;
        }

        let check_module = if apply_normalization { &normalized_module } else { &module };
        let check_block = if apply_normalization { &normalized_block_title } else { &block_title };
        let check_lesson = if apply_normalization { &normalized_lesson_title } else { &lesson_title };
        
        if !module_titles.contains(check_module) {
            return Err(format!(
                "Error: module | '{}' | in file {:?} doesn't match any option in reference file",
                check_module, path
            ));
        }
        if !block_titles.contains(check_block) {
            return Err(format!(
                "Error: block_title | '{}' | in file {:?} doesn't match any option in reference file",
                check_block, path
            ));
        }
        if !lesson_titles.contains(check_lesson) {
            return Err(format!(
                "Error: lesson_title | '{}' | in file {:?} doesn't match any option in reference file",
                check_lesson, path
            ));
        }

        processed_lessons.insert(check_lesson.clone());
    }

    let missing_lessons: Vec<_> = lesson_titles.difference(&processed_lessons).collect();
    if !missing_lessons.is_empty() {
        error!(
            "Missing lessons! The following lessons were not found in input JSON files: {:?}",
            missing_lessons
        );
        return Err(format!(
            "Missing lessons: {:?}",
            missing_lessons
        ));
    }
    
    info!("Completed!");
    info!("Total files processed: {}", files_processed);
    info!("Modified files: {}", files_modified);
    info!("All JSON files are correct and normalized!");
    Ok(())
}

fn normalize_titles(module: &str, block_title: &str, lesson_title: &str) -> (String, String, String) {
    let normalized_module = module.nfc().collect::<String>();
    let normalized_block_title = block_title.nfc().collect::<String>();
    let normalized_lesson_title = lesson_title.nfc().collect::<String>();

    (normalized_module, normalized_block_title, normalized_lesson_title)
}

fn normalize_structure_file() -> Result<(), String> {
    let file_path = "./common_res/w3a/w3a_learning_structure.txt";

    let content =
        fs::read_to_string(file_path).map_err(|e| format!("Failed to read file: {}", e))?;

    let mut json: Value =
        serde_json::from_str(&content).map_err(|e| format!("JSON parsing error: {}", e))?;

    normalize_json_value(&mut json);

    let normalized_content = serde_json::to_string_pretty(&json)
        .map_err(|e| format!("JSON serialization error: {}", e))?;

    fs::write(file_path, normalized_content).map_err(|e| format!("Error writing file: {}", e))?;
    
    Ok(())
}

fn normalize_json_value(value: &mut Value) {
    match value {
        Value::String(s) => {
            *s = s.nfc().collect::<String>();
        }
        Value::Array(arr) => {
            for item in arr {
                normalize_json_value(item);
            }
        }
        Value::Object(map) => {
            for (_, v) in map {
                normalize_json_value(v);
            }
        }
        _ => {}
    }
}
