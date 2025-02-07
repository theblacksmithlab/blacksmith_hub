use core::models::common::app_name::AppName;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

pub fn load_super_admins(app_name: &AppName) -> Vec<u64> {
    let path = build_resource_file_path(app_name, "super_admins_list.json");

    let data = fs::read_to_string(&path).unwrap_or_else(|err| {
        eprintln!("Failed to read {}: {}", path.display(), err);
        "[]".to_string()
    });

    let json: Value = serde_json::from_str(&data).unwrap_or_else(|err| {
        eprintln!("Failed to parse JSON in {}: {}", path.display(), err);
        Value::Array(vec![])
    });

    json.as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|v| v.as_u64())
        .collect()
}

fn build_resource_file_path(app_name: &AppName, file_name: &str) -> PathBuf {
    PathBuf::from("common_res")
        .join(app_name.as_str())
        .join(file_name)
}
