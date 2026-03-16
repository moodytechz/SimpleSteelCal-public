use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use steelcal_core::config::{
    default_table, effective_config, load_normalized_config, CONFIG_SCHEMA_VERSION,
};
use steelcal_core::gauges::{builtin_gauge_tables, DEFAULT_TABLE_NAME};
use steelcal_core::DENSITY_LB_PER_FT3_DEFAULT;

fn temp_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("steelcal-{name}-{nanos}.json"))
}

#[test]
fn unversioned_config_is_upgraded_and_rewritten() {
    let path = temp_path("unversioned");
    let original = json!({
        "density_lb_ft3": 500.0,
        "default_table": "steel",
        "default_gauge": "16"
    });
    fs::write(&path, serde_json::to_string_pretty(&original).unwrap()).unwrap();

    let tables = builtin_gauge_tables();
    let cleaned = load_normalized_config(&path, &tables).unwrap();

    assert_eq!(
        cleaned.get("config_version").and_then(Value::as_i64),
        Some(CONFIG_SCHEMA_VERSION as i64)
    );

    let rewritten: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(rewritten["config_version"], CONFIG_SCHEMA_VERSION);
    assert_eq!(rewritten["density_lb_ft3"], 500.0);

    let _ = fs::remove_file(path);
}

#[test]
fn current_version_config_loads_without_rewrite() {
    let path = temp_path("current");
    let original = json!({
        "config_version": CONFIG_SCHEMA_VERSION,
        "density_lb_ft3": 501.0,
        "default_table": DEFAULT_TABLE_NAME,
        "default_gauge": "16"
    });
    let original_text = serde_json::to_string_pretty(&original).unwrap();
    fs::write(&path, &original_text).unwrap();

    let tables = builtin_gauge_tables();
    let cleaned = load_normalized_config(&path, &tables).unwrap();

    assert_eq!(
        cleaned.get("config_version").and_then(Value::as_i64),
        Some(CONFIG_SCHEMA_VERSION as i64)
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), original_text);

    let _ = fs::remove_file(path);
}

#[test]
fn newer_version_config_is_preserved_and_runtime_uses_defaults() {
    let path = temp_path("future");
    let original = json!({
        "config_version": CONFIG_SCHEMA_VERSION + 1,
        "density_lb_ft3": 999.0,
        "default_table": DEFAULT_TABLE_NAME,
        "default_gauge": "16",
        "future_key": "keep-me"
    });
    let original_text = serde_json::to_string_pretty(&original).unwrap();
    fs::write(&path, &original_text).unwrap();

    let tables = builtin_gauge_tables();
    let cleaned = load_normalized_config(&path, &tables).unwrap();

    assert!(cleaned.is_empty());
    assert_eq!(fs::read_to_string(&path).unwrap(), original_text);

    let effective = effective_config(&cleaned, &tables);
    assert_eq!(effective.density_lb_ft3, DENSITY_LB_PER_FT3_DEFAULT);
    assert_eq!(default_table(&cleaned, &tables), DEFAULT_TABLE_NAME);

    let _ = fs::remove_file(path);
}
