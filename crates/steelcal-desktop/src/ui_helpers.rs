use std::cmp::Ordering;

use slint::{ComponentHandle, Model, ModelRc, SharedString};
use steelcal_core::gauges::{compare_keys, key_to_numeric, GaugeTables};
use steelcal_core::history::{format_timestamp, HistoryEntry, HistoryEntryType, SessionHistory};

use crate::AppWindow;

/// Indices into `SessionHistory::get_entries()` for the currently displayed
/// (filtered) list in the History dialog.
pub type FilteredIndices = Vec<usize>;

/// Collect sorted table names from the gauge tables map.
pub fn table_names_from(tables: &GaugeTables) -> Vec<String> {
    tables.keys().cloned().collect()
}

/// Collect gauge key labels for a given table name.
pub fn gauge_keys_for_table(tables: &GaugeTables, table_name: &str) -> Vec<String> {
    tables
        .get(table_name)
        .map(|t| t.entries.iter().map(|e| e.key.clone()).collect())
        .unwrap_or_default()
}

/// Find the index of a value in a slice (case-insensitive), or 0 if not found.
pub fn find_index(items: &[String], target: &str) -> i32 {
    let target_upper = target.to_uppercase();
    items
        .iter()
        .position(|s| s.to_uppercase() == target_upper)
        .map_or(0, |i| i as i32)
}

/// Convert a Slint string-model `[string]` into a Rust `Vec<String>`.
pub fn slint_model_to_vec(model: ModelRc<SharedString>) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..model.row_count() {
        out.push(model.row_data(i).unwrap_or_default().to_string());
    }
    out
}

fn normalize_gauge_search_token(input: &str) -> String {
    input
        .trim()
        .to_lowercase()
        .replace("inches", "")
        .replace("inch", "")
        .replace("in.", "")
        .replace(['"', ' '], "")
}

fn gauge_search_numeric(input: &str) -> Option<f64> {
    let normalized = normalize_gauge_search_token(input);
    if normalized.is_empty() {
        return None;
    }

    if let Ok(value) = normalized.parse::<f64>() {
        return Some(value);
    }

    let numeric = key_to_numeric(&normalized);
    (numeric.kind != 2).then_some(numeric.value)
}

fn smart_filter_gauge_indices(keys: &[String], query: &str) -> Vec<usize> {
    if query.trim().is_empty() {
        return (0..keys.len()).collect();
    }

    let query_normalized = normalize_gauge_search_token(query);
    let query_numeric = gauge_search_numeric(query);

    let mut direct_matches: Vec<(u8, usize)> = keys
        .iter()
        .enumerate()
        .filter_map(|(idx, key)| {
            let key_normalized = normalize_gauge_search_token(key);

            if key_normalized == query_normalized {
                return Some((0, idx));
            }

            if let (Some(query_value), Some(key_value)) = (query_numeric, gauge_search_numeric(key))
            {
                if (query_value - key_value).abs() < 1e-9 {
                    return Some((1, idx));
                }
            }

            if key_normalized.starts_with(&query_normalized) {
                return Some((2, idx));
            }

            if key_normalized.contains(&query_normalized) {
                return Some((3, idx));
            }

            None
        })
        .collect();

    if !direct_matches.is_empty() {
        direct_matches.sort_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then_with(|| compare_keys(&keys[left.1], &keys[right.1]))
        });
        return direct_matches.into_iter().map(|(_, idx)| idx).collect();
    }

    if let Some(query_value) = query_numeric {
        let mut nearby_matches: Vec<(f64, usize)> = keys
            .iter()
            .enumerate()
            .filter_map(|(idx, key)| {
                let key_value = gauge_search_numeric(key)?;
                Some(((query_value - key_value).abs(), idx))
            })
            .collect();

        nearby_matches.sort_by(|left, right| {
            left.0
                .partial_cmp(&right.0)
                .unwrap_or(Ordering::Equal)
                .then_with(|| compare_keys(&keys[left.1], &keys[right.1]))
        });

        return nearby_matches
            .into_iter()
            .take(15)
            .map(|(_, idx)| idx)
            .collect();
    }

    Vec::new()
}

pub fn current_table_name(app: &AppWindow) -> String {
    let table_names = slint_model_to_vec(app.get_table_names());
    table_names
        .get(app.get_selected_table_index() as usize)
        .cloned()
        .unwrap_or_default()
}

pub fn find_best_gauge_match(keys: &[String], query: &str) -> Option<usize> {
    smart_filter_gauge_indices(keys, query).into_iter().next()
}

fn dispatch_focus_navigation(app: &AppWindow, key: slint::platform::Key, steps: usize) {
    let text: SharedString = key.into();
    let window = app.window();
    for _ in 0..steps {
        window.dispatch_event(slint::platform::WindowEvent::KeyPressed { text: text.clone() });
        window.dispatch_event(slint::platform::WindowEvent::KeyReleased { text: text.clone() });
    }
}

pub fn focus_clear_button(app: &AppWindow) {
    dispatch_focus_navigation(app, slint::platform::Key::Tab, 1);
}

pub fn focus_gauge_field(app: &AppWindow) {
    dispatch_focus_navigation(app, slint::platform::Key::Backtab, 5);
}

pub fn resolve_gauge_index(
    keys: &[String],
    gauge_text: &str,
    selected_index: i32,
) -> Option<usize> {
    let trimmed = gauge_text.trim();
    if !trimmed.is_empty() {
        if let Some(index) = find_best_gauge_match(keys, trimmed) {
            return Some(index);
        }
    }

    (selected_index >= 0)
        .then_some(selected_index as usize)
        .filter(|index| *index < keys.len())
}

/// Parse a string to f64, returning 0.0 for empty strings.
pub fn parse_f64_or_zero(s: &str) -> Result<f64, String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Ok(0.0);
    }
    let v = trimmed
        .parse::<f64>()
        .map_err(|_| format!("Invalid number: '{trimmed}'"))?;
    if !v.is_finite() {
        return Err(format!("Invalid number: '{trimmed}' (must be finite)"));
    }
    Ok(v)
}

/// Parse a required f64 field.
pub fn parse_required_f64(s: &str, field_name: &str) -> Result<f64, String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err(format!("{field_name} is required."));
    }
    let v = trimmed
        .parse::<f64>()
        .map_err(|_| format!("{field_name} must be a valid number."))?;
    if !v.is_finite() {
        return Err(format!(
            "{field_name} must be a finite number (not NaN or Infinity)."
        ));
    }
    Ok(v)
}

/// Parse an inline f64 value with a finite check.
pub fn parse_finite_f64(s: &str, field_name: &str) -> Result<f64, String> {
    let trimmed = s.trim();
    let v = trimmed
        .parse::<f64>()
        .map_err(|_| format!("{field_name} must be a valid number."))?;
    if !v.is_finite() {
        return Err(format!(
            "{field_name} must be a finite number (not NaN or Infinity)."
        ));
    }
    Ok(v)
}

/// Build the filtered entry list from session history.
pub fn build_filtered_history(
    history: &SessionHistory,
    filter_type_index: i32,
    search_text: &str,
) -> (FilteredIndices, Vec<String>) {
    let entries = history.get_entries();
    let search_lower = search_text.trim().to_lowercase();

    let mut indices = Vec::new();
    let mut labels = Vec::new();

    for (i, entry) in entries.iter().enumerate() {
        let type_matches = match filter_type_index {
            1 => entry.entry_type == HistoryEntryType::Sheet,
            2 => entry.entry_type == HistoryEntryType::Coil,
            3 => entry.entry_type == HistoryEntryType::Scrap,
            _ => true,
        };

        if !type_matches {
            continue;
        }

        let (date, time) = format_timestamp(entry.timestamp);
        let label = format!("{date} {time} - {typ} Calculation", typ = entry.entry_type);

        if !search_lower.is_empty() && !label.to_lowercase().contains(&search_lower) {
            let inputs_str = entry.inputs.to_string().to_lowercase();
            let outputs_str = entry.outputs.to_string().to_lowercase();
            if !inputs_str.contains(&search_lower) && !outputs_str.contains(&search_lower) {
                continue;
            }
        }

        indices.push(i);
        labels.push(label);
    }

    (indices, labels)
}

/// Push the filtered history list into the Slint UI.
pub fn populate_history_ui(app: &AppWindow, labels: &[String]) {
    let model: Vec<SharedString> = labels.iter().map(|s| s.into()).collect();
    app.set_history_entry_labels(ModelRc::from(model.as_slice()));
    app.set_history_entry_count(labels.len() as i32);
    app.set_history_selected_index(-1);
    app.set_history_preview_text("".into());
}

/// Build a detailed preview string for a history entry.
pub fn build_preview_text(entry: &HistoryEntry) -> String {
    let (date, time) = format_timestamp(entry.timestamp);
    let mut preview = format!(
        "Type: {typ}\nTimestamp: {date} {time}\n\n",
        typ = entry.entry_type,
    );

    preview.push_str("--- Inputs ---\n");
    if let serde_json::Value::Object(map) = &entry.inputs {
        for (k, v) in map {
            let val_str = match v {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            preview.push_str(&format!("  {k}: {val_str}\n"));
        }
    } else {
        preview.push_str(&format!("  {}\n", entry.inputs));
    }

    preview.push_str("\n--- Outputs ---\n");
    if let serde_json::Value::Object(map) = &entry.outputs {
        for (k, v) in map {
            let val_str = match v {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            preview.push_str(&format!("  {k}: {val_str}\n"));
        }
    } else {
        preview.push_str(&format!("  {}\n", entry.outputs));
    }

    preview
}

/// Extract a string from a JSON value, returning empty string for missing keys.
fn json_str(value: &serde_json::Value, key: &str) -> String {
    match value.get(key) {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Number(n)) => n.to_string(),
        Some(serde_json::Value::Bool(b)) => b.to_string(),
        Some(serde_json::Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    }
}

/// Recall a Sheet history entry: restore all sheet panel fields.
pub fn recall_sheet_entry(
    app: &AppWindow,
    inputs: &serde_json::Value,
    tables: Option<&GaugeTables>,
) {
    app.set_width_text(json_str(inputs, "width").into());
    app.set_length_text(json_str(inputs, "length").into());
    app.set_qty_text(json_str(inputs, "qty").into());

    let input_mode = json_str(inputs, "input_mode");
    match input_mode.as_str() {
        "gauge" => {
            app.set_calc_mode(0);
            let table_name = json_str(inputs, "table");
            let gauge_name = json_str(inputs, "gauge");
            let table_names = slint_model_to_vec(app.get_table_names());
            let table_idx = find_index(&table_names, &table_name);
            app.set_selected_table_index(table_idx);

            if let Some(gt) = tables {
                let resolved_table = table_names
                    .get(table_idx as usize)
                    .cloned()
                    .unwrap_or_default();
                let keys = gauge_keys_for_table(gt, &resolved_table);
                let model: Vec<SharedString> = keys.iter().map(|s| s.into()).collect();
                app.set_gauge_keys(ModelRc::from(model.as_slice()));

                let gauge_idx = find_index(&keys, &gauge_name);
                app.set_selected_gauge_index(gauge_idx);
                app.set_gauge_text(gauge_name.into());
            } else {
                let gauge_keys = slint_model_to_vec(app.get_gauge_keys());
                let gauge_idx = find_index(&gauge_keys, &gauge_name);
                app.set_selected_gauge_index(gauge_idx);
                app.set_gauge_text(gauge_name.into());
            }
        }
        "psf" => {
            app.set_calc_mode(1);
            app.set_psf_text(json_str(inputs, "psf").into());
        }
        "thickness" => {
            app.set_calc_mode(2);
            app.set_thickness_text(json_str(inputs, "thickness").into());
        }
        _ => {
            app.set_calc_mode(0);
        }
    }

    let price_mode_str = json_str(inputs, "price_mode");
    let price_mode_idx = match price_mode_str.as_str() {
        "per_lb" => 0,
        "per_ft2" => 1,
        "per_sheet" => 2,
        _ => 0,
    };
    app.set_price_mode_index(price_mode_idx);
    app.set_price_value_text(json_str(inputs, "price_value").into());
    app.set_markup_text(json_str(inputs, "markup").into());
    app.set_tax_text(json_str(inputs, "tax").into());
    app.set_setup_fee_text(json_str(inputs, "setup_fee").into());
    app.set_minimum_order_text(json_str(inputs, "minimum_order").into());
}

pub fn recall_coil_entry(app: &AppWindow, inputs: &serde_json::Value) {
    app.set_coil_width_text(json_str(inputs, "width").into());
    app.set_coil_thickness_text(json_str(inputs, "thickness").into());
    app.set_coil_id_text(json_str(inputs, "id").into());
    app.set_coil_weight_text(json_str(inputs, "weight").into());
    let density = json_str(inputs, "density");
    if !density.is_empty() {
        app.set_coil_density_text(density.into());
    }
}

pub fn recall_scrap_entry(app: &AppWindow, inputs: &serde_json::Value) {
    app.set_scrap_actual_weight_text(json_str(inputs, "actual_weight").into());
    app.set_scrap_ending_weight_text(json_str(inputs, "ending_weight").into());
    app.set_scrap_base_cost_text(json_str(inputs, "base_cost").into());
    app.set_scrap_processing_cost_text(json_str(inputs, "processing_cost").into());
}
