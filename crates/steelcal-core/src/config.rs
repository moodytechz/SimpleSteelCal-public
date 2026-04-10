use std::fs;
use std::path::{Path, PathBuf};

use directories::BaseDirs;
use log;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::errors::SteelCalError;
use crate::gauges::{
    canonical_gauge_key, normalize_table_name, GaugeTables, DEFAULT_GAUGE_KEY, DEFAULT_TABLE_NAME,
};
use crate::{
    APP_DATA_DIRNAME, CONFIG_FILENAME, DENSITY_LB_PER_FT3_DEFAULT, HISTORY_FILENAME,
    UI_FONT_SIZE_DEFAULT, UI_HEADING_DELTA_DEFAULT, UI_TK_SCALING_DEFAULT,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectiveConfig {
    pub density_lb_ft3: f64,
    pub default_table: String,
    pub default_gauge: String,
    pub ui_font_size: i64,
    pub ui_heading_delta: i64,
    pub ui_scaling: f64,
}

pub const CONFIG_SCHEMA_VERSION: u32 = 1;

pub fn user_data_dir() -> Result<PathBuf, SteelCalError> {
    if cfg!(target_os = "windows") {
        // On Windows, use the standard AppData/Roaming directory via `directories`.
        if let Some(base_dirs) = BaseDirs::new() {
            return Ok(base_dirs.data_dir().join(APP_DATA_DIRNAME));
        }
    } else {
        // On Unix/macOS, use a dotfile in the home directory.
        if let Some(base_dirs) = BaseDirs::new() {
            return Ok(base_dirs.home_dir().join(format!(".{APP_DATA_DIRNAME}")));
        }
    }

    Err(SteelCalError::config(
        "Failed to determine user data directory.",
    ))
}

pub fn config_path() -> Result<PathBuf, SteelCalError> {
    Ok(user_data_dir()?.join(CONFIG_FILENAME))
}

pub fn history_export_path() -> Result<PathBuf, SteelCalError> {
    Ok(user_data_dir()?.join(HISTORY_FILENAME))
}

pub fn load_json_object(path: &Path) -> Result<Map<String, Value>, SteelCalError> {
    let contents = fs::read_to_string(path)?;
    let raw = serde_json::from_str::<Value>(&contents)?;
    let Value::Object(map) = raw else {
        return Err(SteelCalError::config("Config must be a JSON object."));
    };
    Ok(map)
}

pub fn write_json_object(path: &Path, payload: &Map<String, Value>) -> Result<(), SteelCalError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let contents = serde_json::to_string_pretty(payload)?;
    fs::write(path, contents)?;
    Ok(())
}

/// Set of config keys that `normalize_config` recognises.
const KNOWN_CONFIG_KEYS: &[&str] = &[
    "config_version",
    "density_lb_ft3",
    "ui_font_size",
    "ui_heading_delta",
    "ui_scaling",
    "default_table",
    "default_gauge",
];

#[must_use]
pub fn normalize_config(raw: &Map<String, Value>, tables: &GaugeTables) -> Map<String, Value> {
    let mut cleaned = Map::new();

    cleaned.insert("config_version".to_string(), json!(CONFIG_SCHEMA_VERSION));

    // Warn about unknown keys
    for key in raw.keys() {
        if !KNOWN_CONFIG_KEYS.contains(&key.as_str()) {
            log::warn!("Config: unknown key '{key}' ignored");
        }
    }

    // density_lb_ft3 — must be a positive number
    if let Some(val) = raw.get("density_lb_ft3") {
        if let Some(value) = val.as_f64() {
            if value > 0.0 {
                cleaned.insert("density_lb_ft3".to_string(), json!(value));
            } else {
                log::warn!("Config: key 'density_lb_ft3' has invalid value ({value}): must be > 0");
            }
        } else {
            log::warn!("Config: key 'density_lb_ft3' has invalid type: expected number, got {val}");
        }
    }

    // ui_font_size — must be a positive integer
    if let Some(val) = raw.get("ui_font_size") {
        if let Some(value) = val.as_i64() {
            if value > 0 {
                cleaned.insert("ui_font_size".to_string(), json!(value));
            } else {
                log::warn!("Config: key 'ui_font_size' has invalid value ({value}): must be > 0");
            }
        } else {
            log::warn!("Config: key 'ui_font_size' has invalid type: expected integer, got {val}");
        }
    }

    // ui_heading_delta — must be a non-negative integer
    if let Some(val) = raw.get("ui_heading_delta") {
        if let Some(value) = val.as_i64() {
            if value >= 0 {
                cleaned.insert("ui_heading_delta".to_string(), json!(value));
            } else {
                log::warn!(
                    "Config: key 'ui_heading_delta' has invalid value ({value}): must be >= 0"
                );
            }
        } else {
            log::warn!(
                "Config: key 'ui_heading_delta' has invalid type: expected integer, got {val}"
            );
        }
    }

    // ui_scaling — must be a non-negative number
    if let Some(val) = raw.get("ui_scaling") {
        if let Some(value) = val.as_f64() {
            if value >= 0.0 {
                cleaned.insert("ui_scaling".to_string(), json!(value));
            } else {
                log::warn!("Config: key 'ui_scaling' has invalid value ({value}): must be >= 0");
            }
        } else {
            log::warn!("Config: key 'ui_scaling' has invalid type: expected number, got {val}");
        }
    }

    let mut default_table = DEFAULT_TABLE_NAME.to_string();
    if let Some(val) = raw.get("default_table") {
        if let Some(value) = val.as_str() {
            let normalized = normalize_table_name(value);
            if tables.contains_key(&normalized) {
                cleaned.insert(
                    "default_table".to_string(),
                    Value::String(normalized.clone()),
                );
                default_table = normalized;
            }
        } else {
            log::warn!("Config: key 'default_table' has invalid type: expected string, got {val}");
        }
    }

    if let Some(val) = raw.get("default_gauge") {
        if let Some(value) = val.as_str() {
            if !value.trim().is_empty() {
                if let Some(normalized) = canonical_gauge_key(tables, &default_table, value.trim())
                {
                    cleaned.insert("default_gauge".to_string(), Value::String(normalized));
                }
            }
        } else {
            log::warn!("Config: key 'default_gauge' has invalid type: expected string, got {val}");
        }
    }

    cleaned
}

#[must_use]
pub fn default_table(cleaned: &Map<String, Value>, tables: &GaugeTables) -> String {
    cleaned
        .get("default_table")
        .and_then(Value::as_str)
        .map(normalize_table_name)
        .filter(|value| tables.contains_key(value))
        .unwrap_or_else(|| DEFAULT_TABLE_NAME.to_string())
}

#[must_use]
pub fn default_gauge(
    cleaned: &Map<String, Value>,
    tables: &GaugeTables,
    table_name: Option<&str>,
) -> String {
    let table = normalize_table_name(table_name.unwrap_or(&default_table(cleaned, tables)));

    if let Some(value) = cleaned.get("default_gauge").and_then(Value::as_str) {
        if let Some(canonical) = canonical_gauge_key(tables, &table, value) {
            return canonical;
        }
    }

    tables
        .get(&table)
        .and_then(|entries| entries.first_key())
        .map(str::to_string)
        .unwrap_or_else(|| DEFAULT_GAUGE_KEY.to_string())
}

#[must_use]
pub fn effective_config(cleaned: &Map<String, Value>, tables: &GaugeTables) -> EffectiveConfig {
    let table = default_table(cleaned, tables);

    EffectiveConfig {
        density_lb_ft3: cleaned
            .get("density_lb_ft3")
            .and_then(Value::as_f64)
            .unwrap_or(DENSITY_LB_PER_FT3_DEFAULT),
        default_table: table.clone(),
        default_gauge: default_gauge(cleaned, tables, Some(&table)),
        ui_font_size: cleaned
            .get("ui_font_size")
            .and_then(Value::as_i64)
            .unwrap_or(UI_FONT_SIZE_DEFAULT),
        ui_heading_delta: cleaned
            .get("ui_heading_delta")
            .and_then(Value::as_i64)
            .unwrap_or(UI_HEADING_DELTA_DEFAULT),
        ui_scaling: cleaned
            .get("ui_scaling")
            .and_then(Value::as_f64)
            .unwrap_or(UI_TK_SCALING_DEFAULT),
    }
}

/// Returns the built-in default configuration as a pretty-printed JSON string.
///
/// This provides the canonical default values for all configuration keys,
/// useful for display in the config editor's "Restore Defaults" action.
#[must_use]
pub fn default_config_json() -> String {
    let defaults = json!({
        "config_version": CONFIG_SCHEMA_VERSION,
        "density_lb_ft3": DENSITY_LB_PER_FT3_DEFAULT,
        "default_table": DEFAULT_TABLE_NAME,
        "default_gauge": DEFAULT_GAUGE_KEY,
        "ui_font_size": UI_FONT_SIZE_DEFAULT,
        "ui_heading_delta": UI_HEADING_DELTA_DEFAULT,
        "ui_scaling": UI_TK_SCALING_DEFAULT
    });
    serde_json::to_string_pretty(&defaults).unwrap_or_else(|_| "{}".to_string())
}

fn config_version(raw: &Map<String, Value>) -> Option<u32> {
    raw.get("config_version")
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
}

fn migrate_config(raw: &Map<String, Value>, tables: &GaugeTables) -> Map<String, Value> {
    normalize_config(raw, tables)
}

pub fn load_normalized_config(
    path: &Path,
    tables: &GaugeTables,
) -> Result<Map<String, Value>, SteelCalError> {
    if !path.exists() {
        return Ok(Map::new());
    }

    match load_json_object(path) {
        Ok(raw) => match config_version(&raw) {
            Some(version) if version > CONFIG_SCHEMA_VERSION => {
                log::warn!(
                    "Config file '{}' uses newer schema version {} (supported: {}); preserving file and using defaults",
                    path.display(),
                    version,
                    CONFIG_SCHEMA_VERSION
                );
                Ok(Map::new())
            }
            Some(version) if version == CONFIG_SCHEMA_VERSION => {
                let cleaned = normalize_config(&raw, tables);
                if raw != cleaned {
                    if let Err(err) = write_json_object(path, &cleaned) {
                        log::warn!(
                            "Config file '{}' was normalized in memory but could not be rewritten ({}); continuing with normalized values",
                            path.display(),
                            err
                        );
                    }
                }
                Ok(cleaned)
            }
            _ => {
                let cleaned = migrate_config(&raw, tables);
                if let Err(err) = write_json_object(path, &cleaned) {
                    log::warn!(
                        "Config file '{}' was migrated in memory but could not be rewritten ({}); continuing with migrated values",
                        path.display(),
                        err
                    );
                }
                Ok(cleaned)
            }
        },
        Err(err) => {
            log::warn!(
                "Config file '{}' contains invalid data ({}); using defaults",
                path.display(),
                err
            );
            Ok(Map::new())
        }
    }
}

#[must_use]
pub fn sidecar_config_candidates(
    app_dir: &Path,
    bundle_dir: Option<&Path>,
    active_config_path: &Path,
) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    for root in [Some(app_dir), bundle_dir].into_iter().flatten() {
        let candidate = root.join(CONFIG_FILENAME);
        if candidate != active_config_path && !candidates.contains(&candidate) {
            candidates.push(candidate);
        }
    }

    candidates
}

pub fn seed_user_config_from_sidecars(
    active_config_path: &Path,
    sidecars: &[PathBuf],
    tables: &GaugeTables,
) -> Result<bool, SteelCalError> {
    if active_config_path.exists() {
        return Ok(false);
    }

    for candidate in sidecars {
        if !candidate.exists() {
            continue;
        }

        if let Ok(raw) = load_json_object(candidate) {
            let cleaned = normalize_config(&raw, tables);
            write_json_object(active_config_path, &cleaned)?;
            return Ok(true);
        }
    }

    Ok(false)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gauges::builtin_gauge_tables;
    use serde_json::json;
    use std::io::Write;
    use std::sync::Mutex;

    // ------ Simple test logger that captures log messages ------
    //
    // Rust's `log::set_logger` is global and one-shot; tests run in parallel.
    // We accept that messages from concurrent tests can interleave, so each
    // assertion searches the accumulated buffer rather than assuming isolation.

    static LOG_MESSAGES: Mutex<Vec<String>> = Mutex::new(Vec::new());

    struct TestLogger;

    impl log::Log for TestLogger {
        fn enabled(&self, _metadata: &log::Metadata<'_>) -> bool {
            true
        }

        fn log(&self, record: &log::Record<'_>) {
            if let Ok(mut msgs) = LOG_MESSAGES.lock() {
                msgs.push(format!("[{}] {}", record.level(), record.args()));
            }
        }

        fn flush(&self) {}
    }

    static INIT: std::sync::Once = std::sync::Once::new();

    fn init_test_logger() {
        INIT.call_once(|| {
            let _ =
                log::set_logger(&TestLogger).map(|()| log::set_max_level(log::LevelFilter::Warn));
        });
    }

    /// Returns a snapshot of all logged messages (does not drain).
    fn snapshot_log_messages() -> Vec<String> {
        LOG_MESSAGES.lock().map(|m| m.clone()).unwrap_or_default()
    }

    // ------ Tests ------

    #[test]
    fn normalize_config_warns_on_unknown_key() {
        init_test_logger();

        let tables = builtin_gauge_tables();
        let raw: Map<String, Value> =
            serde_json::from_value(json!({ "some_random_key": 42 })).unwrap();

        let cleaned = normalize_config(&raw, &tables);

        // The unknown key must NOT appear in the cleaned output.
        assert!(
            !cleaned.contains_key("some_random_key"),
            "Unknown key should be dropped"
        );

        // A warning should have been logged naming the key.
        let messages = snapshot_log_messages();
        assert!(
            messages
                .iter()
                .any(|m| m.contains("some_random_key") && m.contains("unknown key")),
            "Expected warning about unknown key 'some_random_key', got: {messages:?}"
        );
    }

    #[test]
    fn normalize_config_warns_on_invalid_type_for_known_key() {
        init_test_logger();

        let tables = builtin_gauge_tables();

        // ui_font_size must be an integer, but we pass a boolean.
        let raw: Map<String, Value> =
            serde_json::from_value(json!({ "ui_font_size": true })).unwrap();

        let cleaned = normalize_config(&raw, &tables);

        // The invalid-type key must NOT appear in cleaned output.
        assert!(
            !cleaned.contains_key("ui_font_size"),
            "Invalid-type key should be dropped"
        );

        // A warning should have been logged naming the key and reason.
        let messages = snapshot_log_messages();
        assert!(
            messages
                .iter()
                .any(|m| m.contains("ui_font_size") && m.contains("invalid type")),
            "Expected warning about invalid type for 'ui_font_size', got: {messages:?}"
        );
    }

    #[test]
    fn normalize_config_warns_on_invalid_type_density() {
        init_test_logger();

        let tables = builtin_gauge_tables();

        // density_lb_ft3 must be a number, but we pass a string.
        let raw: Map<String, Value> =
            serde_json::from_value(json!({ "density_lb_ft3": "not a number" })).unwrap();

        let cleaned = normalize_config(&raw, &tables);
        assert!(!cleaned.contains_key("density_lb_ft3"));

        let messages = snapshot_log_messages();
        assert!(
            messages
                .iter()
                .any(|m| m.contains("density_lb_ft3") && m.contains("invalid type")),
            "Expected warning about invalid type for 'density_lb_ft3', got: {messages:?}"
        );
    }

    #[test]
    fn normalize_config_warns_on_invalid_type_default_table() {
        init_test_logger();

        let tables = builtin_gauge_tables();

        // default_table must be a string, but we pass an integer.
        let raw: Map<String, Value> =
            serde_json::from_value(json!({ "default_table": 123 })).unwrap();

        let cleaned = normalize_config(&raw, &tables);
        assert!(!cleaned.contains_key("default_table"));

        let messages = snapshot_log_messages();
        assert!(
            messages
                .iter()
                .any(|m| m.contains("default_table") && m.contains("invalid type")),
            "Expected warning about invalid type for 'default_table', got: {messages:?}"
        );
    }

    #[test]
    fn corrupted_config_file_returns_defaults() {
        init_test_logger();

        let tables = builtin_gauge_tables();

        // Create a temp file with invalid JSON content.
        let dir = std::env::temp_dir().join("steelcal_test_corrupted");
        let _ = fs::create_dir_all(&dir);
        let config_path = dir.join("corrupt_config.json");
        {
            let mut f = fs::File::create(&config_path).unwrap();
            f.write_all(b"{invalid json garbage!!!").unwrap();
        }

        let result = load_normalized_config(&config_path, &tables);

        // Must succeed (not propagate error) with empty/default map.
        assert!(result.is_ok(), "Corrupted config should not error");
        let cleaned = result.unwrap();
        assert!(cleaned.is_empty(), "Corrupted config should yield defaults");

        // A warning should have been logged about invalid data.
        let messages = snapshot_log_messages();
        assert!(
            messages
                .iter()
                .any(|m| m.contains("invalid data") || m.contains("using defaults")),
            "Expected warning about corrupted config, got: {messages:?}"
        );

        // Clean up temp file.
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupted_config_binary_garbage_returns_defaults() {
        init_test_logger();

        let tables = builtin_gauge_tables();

        let dir = std::env::temp_dir().join("steelcal_test_binary");
        let _ = fs::create_dir_all(&dir);
        let config_path = dir.join("binary_config.json");
        {
            let mut f = fs::File::create(&config_path).unwrap();
            // Pure binary garbage.
            f.write_all(&[0u8, 1, 2, 255, 254, 128, 0, 0, 42]).unwrap();
        }

        let result = load_normalized_config(&config_path, &tables);
        assert!(result.is_ok(), "Binary garbage config should not error");
        let cleaned = result.unwrap();
        assert!(cleaned.is_empty(), "Binary garbage should yield defaults");

        let messages = snapshot_log_messages();
        assert!(
            messages.iter().any(|m| m.contains("using defaults")),
            "Expected warning about defaults, got: {messages:?}"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn valid_config_still_works_after_changes() {
        let tables = builtin_gauge_tables();
        let raw: Map<String, Value> = serde_json::from_value(json!({
            "density_lb_ft3": 490.0,
            "ui_font_size": 14,
            "ui_heading_delta": 3,
            "ui_scaling": 1.25,
            "default_table": "HR/HRPO/CR",
            "default_gauge": "16"
        }))
        .unwrap();

        let cleaned = normalize_config(&raw, &tables);

        assert_eq!(
            cleaned.get("density_lb_ft3").and_then(Value::as_f64),
            Some(490.0)
        );
        assert_eq!(
            cleaned.get("ui_font_size").and_then(Value::as_i64),
            Some(14)
        );
        assert_eq!(
            cleaned.get("ui_heading_delta").and_then(Value::as_i64),
            Some(3)
        );
        assert_eq!(
            cleaned.get("ui_scaling").and_then(Value::as_f64),
            Some(1.25)
        );
        assert_eq!(
            cleaned.get("default_table").and_then(Value::as_str),
            Some("HR/HRPO/CR")
        );
        assert_eq!(
            cleaned.get("default_gauge").and_then(Value::as_str),
            Some("16")
        );
    }

    #[test]
    fn default_config_json_is_valid_and_contains_all_keys() {
        let json_str = default_config_json();
        let parsed: Value =
            serde_json::from_str(&json_str).expect("default_config_json() must produce valid JSON");
        let obj = parsed.as_object().expect("Must be a JSON object");

        assert!(obj.contains_key("density_lb_ft3"), "Missing density_lb_ft3");
        assert!(obj.contains_key("default_table"), "Missing default_table");
        assert!(obj.contains_key("default_gauge"), "Missing default_gauge");
        assert!(obj.contains_key("ui_font_size"), "Missing ui_font_size");
        assert!(
            obj.contains_key("ui_heading_delta"),
            "Missing ui_heading_delta"
        );
        assert!(obj.contains_key("ui_scaling"), "Missing ui_scaling");

        // Check the actual default values
        assert_eq!(obj["density_lb_ft3"].as_f64(), Some(490.0));
        assert_eq!(obj["default_table"].as_str(), Some("HR/HRPO/CR"));
        assert_eq!(obj["default_gauge"].as_str(), Some("16"));
    }

    #[test]
    fn user_data_dir_uses_cfg_target_os() {
        // On Windows (where this test runs), user_data_dir should use
        // BaseDirs::data_dir(), which maps to APPDATA/Roaming.
        // Critically, it should NOT probe for the APPDATA env var directly.
        let result = user_data_dir();
        assert!(result.is_ok(), "user_data_dir() should succeed");
        let path = result.unwrap();

        if cfg!(target_os = "windows") {
            // On Windows, the path should be under AppData/Roaming (data_dir).
            let path_str = path.to_string_lossy().to_lowercase();
            assert!(
                path_str.contains("appdata") || path_str.contains("roaming"),
                "Expected AppData/Roaming path on Windows, got: {path:?}"
            );
            assert!(
                path_str.contains("simplesteelcalculator"),
                "Expected SimpleSteelCalculator in path, got: {path:?}"
            );
        } else {
            // On Unix/macOS, should be a dotfile directory.
            let path_str = path.to_string_lossy();
            assert!(
                path_str.contains(".SimpleSteelCalculator"),
                "Expected .SimpleSteelCalculator in path on Unix, got: {path:?}"
            );
        }
    }
}
