#![windows_subsystem = "windows"]

mod startup;
mod handlers;
mod init;
mod ui_helpers;
mod wiring;

slint::include_modules!();

use steelcal_core::gauges::GaugeTables;
use steelcal_core::history::SessionHistory;
use init::{apply_initial_ui, build_initial_models, load_initial_config, load_tables};

use std::cell::RefCell;
use std::rc::Rc;

use startup::show_startup_error;
use ui_helpers::FilteredIndices;
use wiring::{
    register_calculation_callbacks, register_config_callbacks, register_focus_callbacks,
    register_gauge_callbacks, register_history_callbacks, register_menu_callbacks,
};

// ---------------------------------------------------------------------------
// State shared with callbacks
// ---------------------------------------------------------------------------

pub(crate) struct AppState {
    pub(crate) tables: GaugeTables,
    pub(crate) density: f64,
    /// Cached results from the last sheet calculation, used by pricing.
    pub(crate) last_each_lb: f64,
    pub(crate) last_area_each: f64,
    pub(crate) last_qty: i32,
}

// ---------------------------------------------------------------------------
// Initialization
// ---------------------------------------------------------------------------

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let tables = load_tables();
    let cfg = load_initial_config(&tables);
    let models = build_initial_models(&tables, &cfg);
    let app = AppWindow::new()?;
    apply_initial_ui(&app, &cfg, &models);

    let state = Rc::new(RefCell::new(AppState {
        tables,
        density: cfg.density_lb_ft3,
        last_each_lb: 0.0,
        last_area_each: 0.0,
        last_qty: 0,
    }));

    let session_history: Rc<RefCell<SessionHistory>> = Rc::new(RefCell::new(SessionHistory::new()));
    let history_filtered_indices: Rc<RefCell<FilteredIndices>> = Rc::new(RefCell::new(Vec::new()));

    let config_editor_original = Rc::new(RefCell::new(String::new()));
    register_focus_callbacks(&app);
    register_gauge_callbacks(&app, &state);
    register_calculation_callbacks(&app, &state, &session_history);
    register_menu_callbacks(&app);
    register_config_callbacks(&app, &state, &config_editor_original);
    register_history_callbacks(&app, &state, &session_history, &history_filtered_indices);

    app.run()?;

    Ok(())
}

fn main() {
    if let Err(err) = run() {
        show_startup_error(&format!(
            "SteelCal failed to start.\n\n{err}\n\nPlease report this issue."
        ));
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::ui_helpers::{find_best_gauge_match, resolve_gauge_index};

    #[test]
    fn gauge_lookup_prefers_exact_match() {
        let keys = vec!["10".to_string(), "16".to_string(), "18".to_string()];
        assert_eq!(find_best_gauge_match(&keys, "16"), Some(1));
    }

    #[test]
    fn gauge_lookup_handles_fractional_numeric_equivalence() {
        let keys = vec!["7 GA".to_string(), "3/16".to_string(), "1/4".to_string()];
        assert_eq!(find_best_gauge_match(&keys, ".1875"), Some(1));
    }

    #[test]
    fn gauge_lookup_uses_prefix_matching_before_contains() {
        let keys = vec!["3/16".to_string(), "13/16".to_string(), "1/4".to_string()];
        assert_eq!(find_best_gauge_match(&keys, "3/"), Some(0));
    }

    #[test]
    fn resolve_gauge_index_prefers_typed_value_over_selected_index() {
        let keys = vec!["10".to_string(), "16".to_string(), "18".to_string()];
        assert_eq!(resolve_gauge_index(&keys, "18", 0), Some(2));
    }

    #[test]
    fn resolve_gauge_index_falls_back_to_selected_index() {
        let keys = vec!["10".to_string(), "16".to_string(), "18".to_string()];
        assert_eq!(resolve_gauge_index(&keys, "", 1), Some(1));
    }
}
