use crate::ai::common::common::raw_llm_processing_json;
use crate::models::common::ai::LlmModel;
use crate::models::common::app_name::AppName;
use crate::models::common::system_roles::W3ARoleType;
use crate::state::llm_client_init_trait::OpenAIClientInit;
use crate::state::qdrant_client_init_trait::QdrantClientInit;
use crate::utils::common::{build_resource_file_path, get_system_role_or_fallback};
use anyhow::anyhow;
use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{error, info, warn};

pub async fn get_llm_recommendation<T: OpenAIClientInit + QdrantClientInit + Send + Sync>(
    user_raw_request: &str,
    clarified_request: &str,
    app_state: Arc<T>,
    current_cache: &str,
    app_name: &AppName,
    lesson_learned: &Vec<String>,
) -> Result<String> {
    let full_structure = load_academy_structure(app_name).await?;
    let _structure_for_llm = prepare_structure_for_llm(&full_structure)?;
    let modules = get_modules_from_structure(&full_structure);

    let module = get_module_recommendation(
        user_raw_request,
        clarified_request,
        app_state.clone(),
        current_cache,
        app_name,
        &modules,
    )
    .await?;

    info!("Recommended module: {}", module);

    let blocks = get_blocks_from_module(&full_structure, &module);

    let block = get_block_recommendation(
        user_raw_request,
        clarified_request,
        app_state.clone(),
        current_cache,
        app_name,
        &module,
        &blocks,
        &full_structure,
    )
    .await?;

    info!("Recommended block: {}", block);

    let lessons = get_lessons_from_block(&full_structure, &module, &block);

    let lesson = get_lesson_recommendation(
        user_raw_request,
        clarified_request,
        app_state.clone(),
        current_cache,
        app_name,
        &lessons,
        lesson_learned,
    )
    .await?;

    info!("Recommended lesson: {}", lesson);

    Ok(lesson)
}

pub async fn load_academy_structure(app_name: &AppName) -> Result<Value> {
    let file_path = build_resource_file_path(app_name, "learning_structure_with_urls.json");

    let file_content = match tokio::fs::read_to_string(&file_path).await {
        Ok(content) => content,
        Err(err) => {
            error!("Failed to read academy structure file: {}", err);
            return Err(anyhow!("Failed to read academy structure file"));
        }
    };

    let original_structure: Value = serde_json::from_str(&file_content)?;

    let normalized_structure = normalize_structure_case(&original_structure);

    Ok(normalized_structure)
}

fn normalize_structure_case(structure: &Value) -> Value {
    match structure {
        Value::Object(map) => {
            let mut new_map = serde_json::Map::new();
            for (key, value) in map {
                let lowercase_key = key.to_lowercase();
                let normalized_value = normalize_structure_case(value);
                new_map.insert(lowercase_key, normalized_value);
            }
            Value::Object(new_map)
        }
        Value::Array(arr) => {
            let mut new_arr = Vec::new();
            for item in arr {
                new_arr.push(normalize_structure_case(item));
            }
            Value::Array(new_arr)
        }
        Value::String(s) => Value::String(s.to_lowercase()),
        _ => structure.clone(),
    }
}

pub fn prepare_structure_for_llm(full_structure: &Value) -> Result<String> {
    let mut llm_structure = json!({});

    if let Some(full_obj) = full_structure.as_object() {
        for (module, module_value) in full_obj {
            let mut llm_level = json!({});

            if let Some(level_obj) = module_value.as_object() {
                for (block, block_value) in level_obj {
                    let mut lesson_titles = Vec::new();

                    if let Some(lessons_obj) = block_value.as_object() {
                        for (lesson_title, _) in lessons_obj {
                            lesson_titles.push(lesson_title.clone());
                        }
                    }

                    llm_level[block] = json!(lesson_titles);
                }
            }

            llm_structure[module] = llm_level;
        }
    }

    Ok(serde_json::to_string_pretty(&llm_structure)?)
}

pub fn get_modules_from_structure(structure: &Value) -> Vec<String> {
    let mut modules = Vec::new();

    if let Some(obj) = structure.as_object() {
        for (level, _) in obj {
            modules.push(level.clone());
        }
    }

    modules
}

pub fn get_blocks_from_module(structure: &Value, module: &str) -> Vec<String> {
    let mut blocks = Vec::new();

    if let Some(level_obj) = structure.get(module).and_then(|l| l.as_object()) {
        for (block, _) in level_obj {
            blocks.push(block.clone());
        }
    }

    blocks
}

pub fn get_lessons_from_block(structure: &Value, module: &str, block: &str) -> Vec<String> {
    let mut lessons = Vec::new();

    if let Some(block_value) = structure.get(module).and_then(|m| m.get(block)) {
        if let Some(lessons_obj) = block_value.as_object() {
            for (lesson_title, _) in lessons_obj {
                lessons.push(lesson_title.clone());
            }
        }
    }

    lessons
}

pub async fn get_module_recommendation<T: OpenAIClientInit + QdrantClientInit + Send + Sync>(
    user_raw_request: &str,
    clarified_request: &str,
    app_state: Arc<T>,
    current_cache: &str,
    app_name: &AppName,
    modules: &Vec<String>,
) -> Result<String> {
    let system_role =
        get_system_role_or_fallback(&app_name, W3ARoleType::ModuleRecommendation, None);

    let llm_message = format!(
        "Текущий запрос пользователя: {}\nУточнение запроса: {}\nИстория чата: {}\nОсновные модули программы обучения Web3 Academy: {:?}",
        user_raw_request, clarified_request, current_cache, modules
    );

    let result =
        raw_llm_processing_json(&system_role, &llm_message, app_state, LlmModel::Light).await?;

    let parsed_json: Value = serde_json::from_str(&result)?;

    let default_module = modules.first().unwrap_or(&String::new()).clone();

    let module_from_llm = parsed_json
        .get("Рекомендованный модуль")
        .and_then(|v| v.as_str())
        .unwrap_or(&default_module);

    let module_lowercase = module_from_llm.to_lowercase();

    let module = if modules.iter().any(|l| l.to_lowercase() == module_lowercase) {
        module_lowercase
    } else {
        warn!(
            "Invalid module recommendation: '{}', defaulting to BASIC module",
            module_from_llm
        );
        modules
            .first()
            .unwrap_or(&"basic".to_string())
            .to_lowercase()
    };

    Ok(module)
}

pub async fn get_block_recommendation<T: OpenAIClientInit + QdrantClientInit + Send + Sync>(
    user_raw_request: &str,
    clarified_request: &str,
    app_state: Arc<T>,
    current_cache: &str,
    app_name: &AppName,
    module: &str,
    blocks: &Vec<String>,
    full_structure: &Value,
) -> Result<String> {
    let system_role =
        get_system_role_or_fallback(&app_name, W3ARoleType::BlockRecommendation, None);

    let mut blocks_with_lessons = String::new();
    for block in blocks {
        blocks_with_lessons.push_str(&format!("Блок уроков: {}\nУроки в составе блока:\n", block));

        let lessons = get_lessons_from_block(full_structure, module, block);
        for lesson in &lessons {
            blocks_with_lessons.push_str(&format!("- {}\n", lesson));
        }
        blocks_with_lessons.push_str("\n");
    }

    let llm_message = format!(
        "Текущий запрос пользователя: {}\nУточнение запроса: {}\nИстория чата: {}\n\nРелевантные блоки уроков с их содержимым:\n{}",
        user_raw_request, clarified_request, current_cache,
        blocks_with_lessons
    );

    let result =
        raw_llm_processing_json(&system_role, &llm_message, app_state, LlmModel::ComplexMini).await?;

    let parsed_json: Value = serde_json::from_str(&result)?;

    let default_block = blocks.first().unwrap_or(&String::new()).clone();

    let block_from_llm = parsed_json
        .get("Рекомендованный блок")
        .and_then(|v| v.as_str())
        .unwrap_or(&default_block);

    let block_lowercase = block_from_llm.to_lowercase();

    let block = if blocks.iter().any(|c| c.to_lowercase() == block_lowercase) {
        blocks
            .iter()
            .find(|&c| c.to_lowercase() == block_lowercase)
            .unwrap_or(blocks.first().unwrap_or(&"".to_string()))
            .clone()
    } else {
        warn!(
            "Invalid block recommendation: '{}', defaulting to first block",
            block_from_llm
        );
        blocks.first().unwrap_or(&"".to_string()).clone()
    };

    Ok(block)
}

pub async fn get_lesson_recommendation<T: OpenAIClientInit + QdrantClientInit + Send + Sync>(
    user_raw_request: &str,
    clarified_request: &str,
    app_state: Arc<T>,
    current_cache: &str,
    app_name: &AppName,
    lessons: &Vec<String>,
    lesson_learned: &Vec<String>,
) -> Result<String> {
    let system_role =
        get_system_role_or_fallback(&app_name, W3ARoleType::LessonRecommendation, None);

    let llm_message = format!(
        "Текущий запрос пользователя: {}\nУточнение запроса: {}\nИстория чата: {}\n\nРелевантные уроки: {:?}\nРанее изученные уроки: {:?}",
        user_raw_request, clarified_request, current_cache,
        lessons, lesson_learned
    );

    let result =
        raw_llm_processing_json(&system_role, &llm_message, app_state, LlmModel::ComplexMini).await?;

    let parsed_json: Value = serde_json::from_str(&result)?;

    let default_lesson = lessons.first().unwrap_or(&String::new()).clone();

    let lesson_from_llm = parsed_json
        .get("Рекомендованный урок")
        .and_then(|v| v.as_str())
        .unwrap_or(&default_lesson);

    let lesson_lowercase = lesson_from_llm.to_lowercase();

    let lesson = if lessons.iter().any(|l| l.to_lowercase() == lesson_lowercase) {
        lessons
            .iter()
            .find(|&l| l.to_lowercase() == lesson_lowercase)
            .unwrap_or(&default_lesson)
            .clone()
    } else {
        warn!(
            "Invalid lesson recommendation: '{}', defaulting to first lesson",
            lesson_from_llm
        );
        default_lesson
    };

    Ok(lesson)
}
