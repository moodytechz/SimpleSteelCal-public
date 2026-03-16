use std::cell::RefCell;
use std::rc::Rc;

use steelcal_core::config::{
    config_path, default_config_json, effective_config, load_normalized_config,
};
use steelcal_core::history::{export_to_text, HistoryEntry, HistoryEntryType, SessionHistory};
use steelcal_core::{
    compute_coil, compute_costs, compute_each_total_psf, compute_scrap, CoilInputs, CostInputs,
    InputMode, Inputs, PriceMode,
};

use crate::ui_helpers::{
    build_filtered_history, build_preview_text, find_index, focus_clear_button,
    gauge_keys_for_table, parse_f64_or_zero, parse_finite_f64, parse_required_f64,
    populate_history_ui, recall_coil_entry, recall_scrap_entry, recall_sheet_entry,
    resolve_gauge_index, slint_model_to_vec, table_names_from, FilteredIndices,
};
use crate::{AppState, AppWindow};

pub fn open_help(app: &AppWindow) {
    app.set_show_about(false);
    app.set_show_config_editor(false);
    app.set_show_history(false);
    app.set_show_help(true);
}

pub fn open_about(app: &AppWindow) {
    app.set_show_help(false);
    app.set_show_config_editor(false);
    app.set_show_history(false);
    app.set_show_about(true);
}

pub fn open_config_editor(app: &AppWindow, original: &Rc<RefCell<String>>) {
    let content = match config_path() {
        Ok(path) if path.exists() => {
            std::fs::read_to_string(&path).unwrap_or_else(|_| default_config_json())
        }
        _ => default_config_json(),
    };
    let pretty = match serde_json::from_str::<serde_json::Value>(&content) {
        Ok(val) => serde_json::to_string_pretty(&val).unwrap_or_else(|_| content.clone()),
        Err(_) => content.clone(),
    };
    app.set_config_editor_text(pretty.clone().into());
    app.set_config_editor_status("".into());
    app.set_config_editor_status_is_error(false);
    *original.borrow_mut() = pretty;

    app.set_show_help(false);
    app.set_show_about(false);
    app.set_show_history(false);
    app.set_show_config_editor(true);
}

pub fn open_history(
    app: &AppWindow,
    history: &Rc<RefCell<SessionHistory>>,
    filtered: &Rc<RefCell<FilteredIndices>>,
) {
    app.set_history_filter_type_index(0);
    app.set_history_search_text("".into());
    app.set_history_status_text("".into());
    app.set_history_status_is_error(false);

    let hist = history.borrow();
    let (indices, labels) = build_filtered_history(&hist, 0, "");
    populate_history_ui(app, &labels);
    *filtered.borrow_mut() = indices;

    app.set_show_help(false);
    app.set_show_about(false);
    app.set_show_config_editor(false);
    app.set_show_history(true);
}

pub fn validate_config_editor(app: &AppWindow) {
    let text = app.get_config_editor_text().to_string();
    match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(_) => {
            app.set_config_editor_status("✓ Valid JSON".into());
            app.set_config_editor_status_is_error(false);
        }
        Err(e) => {
            app.set_config_editor_status(format!("✗ Invalid JSON: {e}").into());
            app.set_config_editor_status_is_error(true);
        }
    }
}

pub fn revert_config_editor(app: &AppWindow, original: &Rc<RefCell<String>>) {
    let orig = original.borrow().clone();
    app.set_config_editor_text(orig.into());
    app.set_config_editor_status("Reverted to last saved content".into());
    app.set_config_editor_status_is_error(false);
}

pub fn restore_config_defaults(app: &AppWindow) {
    let defaults = default_config_json();
    app.set_config_editor_text(defaults.into());
    app.set_config_editor_status("Defaults restored (not saved yet - click Save to apply)".into());
    app.set_config_editor_status_is_error(false);
}

pub fn open_config_location(app: &AppWindow) {
    match config_path() {
        Ok(path) => {
            let dir = path.parent().unwrap_or(&path);
            let _ = std::fs::create_dir_all(dir);
            #[cfg(windows)]
            {
                let _ = std::process::Command::new("explorer")
                    .arg(dir.as_os_str())
                    .spawn();
            }
            #[cfg(not(windows))]
            {
                let _ = std::process::Command::new("xdg-open")
                    .arg(dir.as_os_str())
                    .spawn();
            }
            app.set_config_editor_status("".into());
        }
        Err(e) => {
            app.set_config_editor_status(format!("✗ Cannot determine config path: {e}").into());
            app.set_config_editor_status_is_error(true);
        }
    }
}

pub fn save_config_editor(
    app: &AppWindow,
    state: &Rc<RefCell<AppState>>,
    original: &Rc<RefCell<String>>,
) {
    let text = app.get_config_editor_text().to_string();

    let parsed = match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(val) => val,
        Err(e) => {
            app.set_config_editor_status(format!("✗ Cannot save: invalid JSON — {e}").into());
            app.set_config_editor_status_is_error(true);
            return;
        }
    };

    let path = match config_path() {
        Ok(p) => p,
        Err(e) => {
            app.set_config_editor_status(format!("✗ Cannot determine config path: {e}").into());
            app.set_config_editor_status_is_error(true);
            return;
        }
    };

    let pretty = serde_json::to_string_pretty(&parsed).unwrap_or_else(|_| text.clone());

    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            app.set_config_editor_status(format!("✗ Cannot create config directory: {e}").into());
            app.set_config_editor_status_is_error(true);
            return;
        }
    }

    if let Err(e) = std::fs::write(&path, &pretty) {
        app.set_config_editor_status(format!("✗ Failed to save: {e}").into());
        app.set_config_editor_status_is_error(true);
        return;
    }

    *original.borrow_mut() = pretty.clone();
    app.set_config_editor_text(pretty.into());

    let st = state.borrow();
    let cleaned = load_normalized_config(&path, &st.tables).unwrap_or_default();
    let cfg = effective_config(&cleaned, &st.tables);
    drop(st);

    {
        let mut st_mut = state.borrow_mut();
        st_mut.density = cfg.density_lb_ft3;
    }

    app.set_default_table(cfg.default_table.clone().into());
    app.set_default_gauge(cfg.default_gauge.clone().into());
    app.set_coil_density_text(format!("{}", cfg.density_lb_ft3).into());

    let st = state.borrow();
    let table_names = table_names_from(&st.tables);
    let default_table_idx = find_index(&table_names, &cfg.default_table);
    let initial_table = table_names
        .get(default_table_idx as usize)
        .cloned()
        .unwrap_or_default();
    let initial_gauges = gauge_keys_for_table(&st.tables, &initial_table);
    let default_gauge_idx = find_index(&initial_gauges, &cfg.default_gauge);

    let table_model: Vec<slint::SharedString> = table_names.iter().map(|s| s.into()).collect();
    app.set_table_names(slint::ModelRc::from(table_model.as_slice()));
    app.set_selected_table_index(default_table_idx);

    let gauge_model: Vec<slint::SharedString> = initial_gauges.iter().map(|s| s.into()).collect();
    app.set_gauge_keys(slint::ModelRc::from(gauge_model.as_slice()));
    app.set_selected_gauge_index(default_gauge_idx);
    app.set_gauge_text(cfg.default_gauge.clone().into());

    app.set_config_editor_status("✓ Configuration saved successfully".into());
    app.set_config_editor_status_is_error(false);
}

fn build_sheet_history_inputs(
    width: f64,
    length: f64,
    qty: i32,
    calc_mode: i32,
    table_names: &[String],
    gauge_keys: &[String],
    selected_table_index: i32,
    gauge_text: &str,
    selected_gauge_index: i32,
    psf_text: &str,
    thickness_text: &str,
    price_mode_index: i32,
    price_value_text: &str,
    markup_text: &str,
    tax_text: &str,
    setup_fee_text: &str,
    minimum_order_text: &str,
) -> serde_json::Value {
    let mut inputs = serde_json::Map::new();
    inputs.insert("width".into(), serde_json::json!(width));
    inputs.insert("length".into(), serde_json::json!(length));
    inputs.insert("qty".into(), serde_json::json!(qty));

    match calc_mode {
        0 => {
            let table = table_names
                .get(selected_table_index as usize)
                .cloned()
                .unwrap_or_default();
            let gauge = resolve_gauge_index(gauge_keys, gauge_text, selected_gauge_index)
                .and_then(|index| gauge_keys.get(index).cloned())
                .unwrap_or_else(|| gauge_text.to_string());
            inputs.insert("table".into(), serde_json::json!(table));
            inputs.insert("gauge".into(), serde_json::json!(gauge));
            inputs.insert("input_mode".into(), serde_json::json!("gauge"));
        }
        1 => {
            inputs.insert("psf".into(), serde_json::json!(psf_text));
            inputs.insert("input_mode".into(), serde_json::json!("psf"));
        }
        2 => {
            inputs.insert("thickness".into(), serde_json::json!(thickness_text));
            inputs.insert("input_mode".into(), serde_json::json!("thickness"));
        }
        _ => {}
    }

    let price_mode = match price_mode_index {
        0 => "per_lb",
        1 => "per_ft2",
        2 => "per_sheet",
        _ => "unknown",
    };
    inputs.insert("price_mode".into(), serde_json::json!(price_mode));
    inputs.insert("price_value".into(), serde_json::json!(price_value_text));
    inputs.insert("markup".into(), serde_json::json!(markup_text));
    inputs.insert("tax".into(), serde_json::json!(tax_text));
    inputs.insert("setup_fee".into(), serde_json::json!(setup_fee_text));
    inputs.insert("minimum_order".into(), serde_json::json!(minimum_order_text));

    serde_json::Value::Object(inputs)
}

pub fn calculate_sheet(
    app: &AppWindow,
    state: &Rc<RefCell<AppState>>,
    history: &Rc<RefCell<SessionHistory>>,
) {
    app.set_result_each_lb("".into());
    app.set_result_total_lb("".into());
    app.set_result_psf("".into());
    app.set_result_area_each("".into());
    app.set_result_area_total("".into());
    app.set_error_message("".into());
    app.set_cost_each_before_tax("".into());
    app.set_cost_each_after_tax("".into());
    app.set_cost_total_before_tax("".into());
    app.set_cost_total_after_tax("".into());
    app.set_cost_minimum_applied("".into());
    app.set_pricing_error_message("".into());

    let set_error = |msg: &str| {
        app.set_error_message(msg.into());
    };
    let focus_after_calculate = || {
        focus_clear_button(app);
    };

    let width = match parse_finite_f64(app.get_width_text().as_ref(), "Width") {
        Ok(v) => v,
        Err(msg) => {
            set_error(&msg);
            focus_after_calculate();
            return;
        }
    };

    let length = match parse_finite_f64(app.get_length_text().as_ref(), "Length") {
        Ok(v) => v,
        Err(msg) => {
            set_error(&msg);
            focus_after_calculate();
            return;
        }
    };

    let qty: i32 = match app.get_qty_text().to_string().trim().parse() {
        Ok(v) => v,
        Err(_) => {
            set_error("Quantity must be a valid integer.");
            focus_after_calculate();
            return;
        }
    };

    let calc_mode = app.get_calc_mode();
    let mode = match calc_mode {
        0 => {
            let table_names = slint_model_to_vec(app.get_table_names());
            let gauge_keys = slint_model_to_vec(app.get_gauge_keys());
            let table = table_names
                .get(app.get_selected_table_index() as usize)
                .cloned()
                .unwrap_or_default();
            let key = resolve_gauge_index(
                &gauge_keys,
                app.get_gauge_text().as_str(),
                app.get_selected_gauge_index(),
            )
            .and_then(|index| gauge_keys.get(index).cloned())
            .unwrap_or_else(|| app.get_gauge_text().to_string());
            InputMode::Gauge { table, key }
        }
        1 => {
            let psf = match parse_finite_f64(app.get_psf_text().as_ref(), "PSF") {
                Ok(v) => v,
                Err(msg) => {
                    set_error(&msg);
                    focus_after_calculate();
                    return;
                }
            };
            InputMode::Psf(psf)
        }
        2 => {
            let thickness = match parse_finite_f64(app.get_thickness_text().as_ref(), "Thickness")
            {
                Ok(v) => v,
                Err(msg) => {
                    set_error(&msg);
                    focus_after_calculate();
                    return;
                }
            };
            InputMode::Thickness(thickness)
        }
        _ => {
            set_error("Unknown calculation mode.");
            focus_after_calculate();
            return;
        }
    };

    let st = state.borrow();
    let inputs = Inputs {
        width_in: width,
        length_in: length,
        qty,
        mode,
        density_lb_ft3: st.density,
    };

    match compute_each_total_psf(&inputs, &st.tables) {
        Ok(result) => {
            app.set_result_each_lb(format!("{:.3}", result.each_lb).into());
            app.set_result_total_lb(format!("{:.3}", result.total_lb).into());
            app.set_result_psf(format!("{:.4}", result.psf).into());
            app.set_result_area_each(format!("{:.4}", result.area_ft2_each).into());
            app.set_result_area_total(format!("{:.4}", result.area_ft2_total).into());

            drop(st);
            let mut st_mut = state.borrow_mut();
            st_mut.last_each_lb = result.each_lb;
            st_mut.last_area_each = result.area_ft2_each;
            st_mut.last_qty = qty;
            drop(st_mut);

            let history_inputs = build_sheet_history_inputs(
                width,
                length,
                qty,
                calc_mode,
                &slint_model_to_vec(app.get_table_names()),
                &slint_model_to_vec(app.get_gauge_keys()),
                app.get_selected_table_index(),
                app.get_gauge_text().as_str(),
                app.get_selected_gauge_index(),
                app.get_psf_text().as_str(),
                app.get_thickness_text().as_str(),
                app.get_price_mode_index(),
                app.get_price_value_text().as_str(),
                app.get_markup_text().as_str(),
                app.get_tax_text().as_str(),
                app.get_setup_fee_text().as_str(),
                app.get_minimum_order_text().as_str(),
            );
            let history_outputs = serde_json::json!({
                "mass_each": result.each_lb,
                "mass_total": result.total_lb,
                "psf": result.psf,
                "area_each": result.area_ft2_each,
                "area_total": result.area_ft2_total,
            });
            history.borrow_mut().add_entry(HistoryEntry::new(
                HistoryEntryType::Sheet,
                history_inputs,
                history_outputs,
            ));
        }
        Err(e) => {
            set_error(&e.user_message());
        }
    }

    focus_after_calculate();
}

pub fn calculate_pricing(app: &AppWindow, state: &Rc<RefCell<AppState>>) {
    app.set_cost_each_before_tax("".into());
    app.set_cost_each_after_tax("".into());
    app.set_cost_total_before_tax("".into());
    app.set_cost_total_after_tax("".into());
    app.set_cost_minimum_applied("".into());
    app.set_pricing_error_message("".into());

    let set_pricing_error = |msg: &str| {
        app.set_pricing_error_message(msg.into());
    };

    let price_mode = match app.get_price_mode_index() {
        0 => PriceMode::PerLb,
        1 => PriceMode::PerFt2,
        2 => PriceMode::PerSheet,
        _ => {
            set_pricing_error("Unknown price mode.");
            return;
        }
    };

    let price_value = match parse_f64_or_zero(app.get_price_value_text().as_str()) {
        Ok(v) => v,
        Err(msg) => {
            set_pricing_error(&msg);
            return;
        }
    };

    let markup = match parse_f64_or_zero(app.get_markup_text().as_str()) {
        Ok(v) => v,
        Err(msg) => {
            set_pricing_error(&msg);
            return;
        }
    };

    let tax = match parse_f64_or_zero(app.get_tax_text().as_str()) {
        Ok(v) => v,
        Err(msg) => {
            set_pricing_error(&msg);
            return;
        }
    };

    let setup_fee = match parse_f64_or_zero(app.get_setup_fee_text().as_str()) {
        Ok(v) => v,
        Err(msg) => {
            set_pricing_error(&msg);
            return;
        }
    };

    let minimum_order = match parse_f64_or_zero(app.get_minimum_order_text().as_str()) {
        Ok(v) => v,
        Err(msg) => {
            set_pricing_error(&msg);
            return;
        }
    };

    let cost_inputs = CostInputs {
        mode: price_mode,
        price_value,
        markup_pct: markup,
        tax_pct: tax,
        setup_fee,
        minimum_order,
    };

    let st = state.borrow();
    match compute_costs(
        &cost_inputs,
        st.last_qty,
        st.last_each_lb,
        st.last_area_each,
    ) {
        Ok(result) => {
            app.set_cost_each_before_tax(format!("${:.2}", result.each_before_tax).into());
            app.set_cost_each_after_tax(format!("${:.2}", result.each_after_tax).into());
            app.set_cost_total_before_tax(format!("${:.2}", result.total_before_tax).into());
            app.set_cost_total_after_tax(format!("${:.2}", result.total_after_tax).into());
            app.set_cost_minimum_applied(
                if result.minimum_applied { "Yes" } else { "No" }.into(),
            );
        }
        Err(e) => {
            set_pricing_error(&e.user_message());
        }
    }
}

pub fn calculate_coil(app: &AppWindow, history: &Rc<RefCell<SessionHistory>>) {
    app.set_coil_result_footage("".into());
    app.set_coil_result_piw("".into());
    app.set_coil_result_od("".into());
    app.set_coil_error_message("".into());

    let set_coil_error = |msg: &str| {
        app.set_coil_error_message(msg.into());
    };

    let coil_width = match parse_required_f64(app.get_coil_width_text().as_str(), "Coil Width") {
        Ok(v) => v,
        Err(msg) => {
            set_coil_error(&msg);
            return;
        }
    };

    let coil_thickness = match parse_required_f64(app.get_coil_thickness_text().as_str(), "Thickness")
    {
        Ok(v) => v,
        Err(msg) => {
            set_coil_error(&msg);
            return;
        }
    };

    let coil_id = match parse_required_f64(app.get_coil_id_text().as_str(), "Inner Diameter") {
        Ok(v) => v,
        Err(msg) => {
            set_coil_error(&msg);
            return;
        }
    };

    let coil_weight = match parse_required_f64(app.get_coil_weight_text().as_str(), "Weight") {
        Ok(v) => v,
        Err(msg) => {
            set_coil_error(&msg);
            return;
        }
    };

    let density = match parse_finite_f64(app.get_coil_density_text().as_ref(), "Density") {
        Ok(v) => v,
        Err(msg) => {
            set_coil_error(&msg);
            return;
        }
    };

    let inputs = CoilInputs {
        coil_width_in: coil_width,
        coil_thickness_in: coil_thickness,
        coil_id_in: coil_id,
        coil_weight_lb: coil_weight,
        density_lb_ft3: density,
    };

    match compute_coil(&inputs) {
        Ok(result) => {
            app.set_coil_result_footage(format!("{:.2}", result.coil_footage_ft).into());
            app.set_coil_result_piw(format!("{:.3}", result.coil_piw_lb_per_in).into());
            app.set_coil_result_od(
                result
                    .coil_od_in
                    .map_or("N/A".to_string(), |od| format!("{:.3}", od))
                    .into(),
            );

            let hist_inputs = serde_json::json!({
                "width": coil_width,
                "thickness": coil_thickness,
                "id": coil_id,
                "weight": coil_weight,
                "density": density,
            });
            let hist_outputs = serde_json::json!({
                "footage": result.coil_footage_ft,
                "piw": result.coil_piw_lb_per_in,
                "od": result.coil_od_in,
            });
            history.borrow_mut().add_entry(HistoryEntry::new(
                HistoryEntryType::Coil,
                hist_inputs,
                hist_outputs,
            ));
        }
        Err(e) => {
            set_coil_error(&e.user_message());
        }
    }
}

pub fn calculate_scrap(app: &AppWindow, history: &Rc<RefCell<SessionHistory>>) {
    app.set_scrap_result_scrap_lb("".into());
    app.set_scrap_result_total_cost("".into());
    app.set_scrap_result_price_per_lb("".into());
    app.set_scrap_result_scrap_charge_per_lb("".into());
    app.set_scrap_result_is_pickup("".into());
    app.set_scrap_error_message("".into());

    let set_scrap_error = |msg: &str| {
        app.set_scrap_error_message(msg.into());
    };

    let actual_weight =
        match parse_finite_f64(app.get_scrap_actual_weight_text().as_ref(), "Actual weight") {
            Ok(v) => v,
            Err(msg) => {
                set_scrap_error(&msg);
                return;
            }
        };

    let ending_weight =
        match parse_finite_f64(app.get_scrap_ending_weight_text().as_ref(), "Ending weight") {
            Ok(v) => v,
            Err(msg) => {
                set_scrap_error(&msg);
                return;
            }
        };

    let base_cost = match parse_f64_or_zero(app.get_scrap_base_cost_text().as_str()) {
        Ok(v) => v,
        Err(msg) => {
            set_scrap_error(&msg);
            return;
        }
    };

    let processing_cost = match parse_f64_or_zero(app.get_scrap_processing_cost_text().as_str()) {
        Ok(v) => v,
        Err(msg) => {
            set_scrap_error(&msg);
            return;
        }
    };

    match compute_scrap(actual_weight, ending_weight, base_cost, processing_cost) {
        Ok(result) => {
            app.set_scrap_result_scrap_lb(format!("{:.3}", result.scrap_lb).into());
            app.set_scrap_result_total_cost(format!("${:.2}", result.total_cost).into());
            app.set_scrap_result_price_per_lb(format!("${:.4}", result.price_per_lb).into());
            app.set_scrap_result_scrap_charge_per_lb(
                format!("${:.4}", result.scrap_charge_per_lb).into(),
            );
            app.set_scrap_result_is_pickup(
                if result.is_pickup { "Yes" } else { "No" }.into(),
            );

            let hist_inputs = serde_json::json!({
                "actual_weight": actual_weight,
                "ending_weight": ending_weight,
                "base_cost": base_cost,
                "processing_cost": processing_cost,
            });
            let hist_outputs = serde_json::json!({
                "scrap_lb": result.scrap_lb,
                "total_cost": result.total_cost,
                "price_per_lb": result.price_per_lb,
                "scrap_charge_per_lb": result.scrap_charge_per_lb,
                "is_pickup": result.is_pickup,
            });
            history.borrow_mut().add_entry(HistoryEntry::new(
                HistoryEntryType::Scrap,
                hist_inputs,
                hist_outputs,
            ));
        }
        Err(e) => {
            set_scrap_error(&e.user_message());
        }
    }
}

pub fn refresh_history_filter(
    app: &AppWindow,
    history: &Rc<RefCell<SessionHistory>>,
    filtered: &Rc<RefCell<FilteredIndices>>,
) {
    let filter_idx = app.get_history_filter_type_index();
    let search = app.get_history_search_text().to_string();

    let hist = history.borrow();
    let (indices, labels) = build_filtered_history(&hist, filter_idx, &search);
    populate_history_ui(app, &labels);
    *filtered.borrow_mut() = indices;
}

pub fn select_history_entry(
    app: &AppWindow,
    history: &Rc<RefCell<SessionHistory>>,
    filtered: &Rc<RefCell<FilteredIndices>>,
    idx: i32,
) {
    let fi = filtered.borrow();
    let hist = history.borrow();
    let entries = hist.get_entries();

    if let Some(&real_idx) = fi.get(idx as usize) {
        if let Some(entry) = entries.get(real_idx) {
            let preview = build_preview_text(entry);
            app.set_history_preview_text(preview.into());
            return;
        }
    }

    app.set_history_preview_text("".into());
}

pub fn recall_history_entry(
    app: &AppWindow,
    history: &Rc<RefCell<SessionHistory>>,
    filtered: &Rc<RefCell<FilteredIndices>>,
    state: &Rc<RefCell<AppState>>,
) {
    let selected = app.get_history_selected_index();
    if selected < 0 {
        app.set_history_status_text("No entry selected.".into());
        app.set_history_status_is_error(true);
        return;
    }

    let fi = filtered.borrow();
    let hist = history.borrow();
    let entries = hist.get_entries();

    let real_idx = match fi.get(selected as usize) {
        Some(&i) => i,
        None => {
            app.set_history_status_text("Invalid selection.".into());
            app.set_history_status_is_error(true);
            return;
        }
    };

    let entry = match entries.get(real_idx) {
        Some(e) => e,
        None => {
            app.set_history_status_text("Entry not found.".into());
            app.set_history_status_is_error(true);
            return;
        }
    };

    match entry.entry_type {
        HistoryEntryType::Sheet => {
            let st = state.borrow();
            recall_sheet_entry(app, &entry.inputs, Some(&st.tables));
            app.set_active_tab_index(0);
        }
        HistoryEntryType::Coil => {
            recall_coil_entry(app, &entry.inputs);
            app.set_active_tab_index(1);
        }
        HistoryEntryType::Scrap => {
            recall_scrap_entry(app, &entry.inputs);
            app.set_active_tab_index(0);
        }
        HistoryEntryType::Pricing => {
            let st = state.borrow();
            recall_sheet_entry(app, &entry.inputs, Some(&st.tables));
            app.set_active_tab_index(0);
        }
    }

    app.set_show_history(false);
}

pub fn export_history(
    app: &AppWindow,
    history: &Rc<RefCell<SessionHistory>>,
    filtered: &Rc<RefCell<FilteredIndices>>,
) {
    let fi = filtered.borrow();
    let hist = history.borrow();
    let entries = hist.get_entries();

    let filtered_entries: Vec<HistoryEntry> =
        fi.iter().filter_map(|&i| entries.get(i).cloned()).collect();

    let text = export_to_text(&filtered_entries);

    let export_path = if let Ok(exe) = std::env::current_exe() {
        exe.parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("steelcal_history_export.txt")
    } else {
        std::path::PathBuf::from("steelcal_history_export.txt")
    };

    match std::fs::write(&export_path, &text) {
        Ok(()) => {
            app.set_history_status_text(
                format!(
                    "✓ Exported {} entries to {}",
                    filtered_entries.len(),
                    export_path.display()
                )
                .into(),
            );
            app.set_history_status_is_error(false);
        }
        Err(e) => {
            app.set_history_status_text(format!("✗ Export failed: {e}").into());
            app.set_history_status_is_error(true);
        }
    }
}

pub fn clear_sheet(app: &AppWindow, state: &Rc<RefCell<AppState>>) {
    app.set_width_text("".into());
    app.set_length_text("".into());
    app.set_qty_text("1".into());
    app.set_calc_mode(0);
    app.set_psf_text("".into());
    app.set_thickness_text("".into());

    let st = state.borrow();
    let table_names = table_names_from(&st.tables);
    let default_table = app.get_default_table().to_string();
    let default_gauge = app.get_default_gauge().to_string();
    let table_idx = find_index(&table_names, &default_table);
    app.set_selected_table_index(table_idx);

    let initial_table = table_names
        .get(table_idx as usize)
        .cloned()
        .unwrap_or_default();
    let gauges = gauge_keys_for_table(&st.tables, &initial_table);
    let gauge_model: Vec<slint::SharedString> = gauges.iter().map(|s| s.into()).collect();
    app.set_gauge_keys(slint::ModelRc::from(gauge_model.as_slice()));
    let gauge_idx = find_index(&gauges, &default_gauge);
    app.set_selected_gauge_index(gauge_idx);
    app.set_gauge_text(default_gauge.into());

    app.set_result_each_lb("".into());
    app.set_result_total_lb("".into());
    app.set_result_psf("".into());
    app.set_result_area_each("".into());
    app.set_result_area_total("".into());
    app.set_error_message("".into());

    app.set_price_mode_index(0);
    app.set_price_value_text("".into());
    app.set_markup_text("".into());
    app.set_tax_text("".into());
    app.set_setup_fee_text("".into());
    app.set_minimum_order_text("".into());
    app.set_cost_each_before_tax("".into());
    app.set_cost_each_after_tax("".into());
    app.set_cost_total_before_tax("".into());
    app.set_cost_total_after_tax("".into());
    app.set_cost_minimum_applied("".into());
    app.set_pricing_error_message("".into());

    drop(st);
    let mut st_mut = state.borrow_mut();
    st_mut.last_each_lb = 0.0;
    st_mut.last_area_each = 0.0;
    st_mut.last_qty = 0;
}

pub fn clear_coil(app: &AppWindow, state: &Rc<RefCell<AppState>>) {
    app.set_coil_width_text("".into());
    app.set_coil_thickness_text("".into());
    app.set_coil_id_text("".into());
    app.set_coil_weight_text("".into());
    let st = state.borrow();
    app.set_coil_density_text(format!("{}", st.density).into());
    drop(st);

    app.set_coil_result_footage("".into());
    app.set_coil_result_piw("".into());
    app.set_coil_result_od("".into());
    app.set_coil_error_message("".into());
}

pub fn clear_scrap(app: &AppWindow) {
    app.set_scrap_actual_weight_text("".into());
    app.set_scrap_ending_weight_text("".into());
    app.set_scrap_base_cost_text("".into());
    app.set_scrap_processing_cost_text("".into());

    app.set_scrap_result_scrap_lb("".into());
    app.set_scrap_result_total_cost("".into());
    app.set_scrap_result_price_per_lb("".into());
    app.set_scrap_result_scrap_charge_per_lb("".into());
    app.set_scrap_result_is_pickup("".into());
    app.set_scrap_error_message("".into());
}

pub fn copy_to_actual(app: &AppWindow) {
    let mass_total = app.get_result_total_lb().to_string();
    app.set_scrap_actual_weight_text(mass_total.into());
}

pub fn copy_to_ending(app: &AppWindow) {
    let mass_total = app.get_result_total_lb().to_string();
    app.set_scrap_ending_weight_text(mass_total.into());
}

#[cfg(test)]
mod tests {
    use super::build_sheet_history_inputs;

    #[test]
    fn sheet_history_inputs_preserve_typed_gauge_and_include_pricing_fields() {
        let value = build_sheet_history_inputs(
            48.0,
            96.0,
            3,
            0,
            &["Galv".to_string()],
            &["16".to_string(), "18".to_string()],
            0,
            "18",
            0,
            "",
            "",
            2,
            "125.50",
            "12",
            "8.5",
            "15",
            "250",
        );

        assert_eq!(value["table"], "Galv");
        assert_eq!(value["gauge"], "18");
        assert_eq!(value["input_mode"], "gauge");
        assert_eq!(value["price_mode"], "per_sheet");
        assert_eq!(value["price_value"], "125.50");
        assert_eq!(value["markup"], "12");
        assert_eq!(value["tax"], "8.5");
        assert_eq!(value["setup_fee"], "15");
        assert_eq!(value["minimum_order"], "250");
    }
}
