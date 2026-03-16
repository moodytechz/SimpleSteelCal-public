use std::path::{Path, PathBuf};

use steelcal_core::config::{config_path, effective_config, load_normalized_config};
use steelcal_core::gauges::{builtin_gauge_tables, load_override_tables, merge_tables, GaugeTables};
use steelcal_core::{APP_COPYRIGHT, APP_VERSION};

use crate::ui_helpers::{find_index, gauge_keys_for_table, table_names_from};
use crate::AppWindow;

pub struct InitialConfig {
    pub default_table: String,
    pub default_gauge: String,
    pub density_lb_ft3: f64,
}

pub struct InitialModels {
    pub table_names: Vec<String>,
    pub default_table_idx: i32,
    pub initial_gauges: Vec<String>,
    pub default_gauge_idx: i32,
}

pub fn resolve_override_path(current_exe: Option<&Path>) -> PathBuf {
    current_exe
        .and_then(|exe| exe.parent().map(|dir| dir.join("assets/gauge_tables.override.json")))
        .unwrap_or_else(|| Path::new("assets/gauge_tables.override.json").to_path_buf())
}

pub fn load_tables() -> GaugeTables {
    let mut tables = builtin_gauge_tables();
    let override_path = resolve_override_path(std::env::current_exe().ok().as_deref());

    if override_path.exists() {
        match load_override_tables(&override_path) {
            Ok(overrides) => {
                merge_tables(&mut tables, &overrides);
            }
            Err(e) => {
                log::warn!(
                    "Failed to load override gauge tables from {}: {}",
                    override_path.display(),
                    e
                );
            }
        }
    }

    tables
}

pub fn load_initial_config(tables: &GaugeTables) -> InitialConfig {
    let cleaned = match config_path() {
        Ok(path) => load_normalized_config(&path, tables).unwrap_or_default(),
        Err(_) => serde_json::Map::new(),
    };
    let cfg = effective_config(&cleaned, tables);

    InitialConfig {
        default_table: cfg.default_table,
        default_gauge: cfg.default_gauge,
        density_lb_ft3: cfg.density_lb_ft3,
    }
}

pub fn build_initial_models(tables: &GaugeTables, cfg: &InitialConfig) -> InitialModels {
    let table_names = table_names_from(tables);
    let default_table_idx = find_index(&table_names, &cfg.default_table);
    let initial_table = table_names
        .get(default_table_idx as usize)
        .cloned()
        .unwrap_or_default();
    let initial_gauges = gauge_keys_for_table(tables, &initial_table);
    let default_gauge_idx = find_index(&initial_gauges, &cfg.default_gauge);

    InitialModels {
        table_names,
        default_table_idx,
        initial_gauges,
        default_gauge_idx,
    }
}

pub fn apply_initial_ui(app: &AppWindow, cfg: &InitialConfig, models: &InitialModels) {
    app.set_default_table(cfg.default_table.clone().into());
    app.set_default_gauge(cfg.default_gauge.clone().into());
    app.set_app_version(APP_VERSION.into());
    app.set_app_copyright(APP_COPYRIGHT.into());

    let table_model: Vec<slint::SharedString> = models.table_names.iter().map(|s| s.into()).collect();
    app.set_table_names(slint::ModelRc::from(table_model.as_slice()));
    app.set_selected_table_index(models.default_table_idx);

    let gauge_model: Vec<slint::SharedString> =
        models.initial_gauges.iter().map(|s| s.into()).collect();
    app.set_gauge_keys(slint::ModelRc::from(gauge_model.as_slice()));
    app.set_selected_gauge_index(models.default_gauge_idx);
    app.set_gauge_text(
        models
            .initial_gauges
            .get(models.default_gauge_idx as usize)
            .cloned()
            .unwrap_or_default()
            .into(),
    );

    app.set_coil_density_text(format!("{}", cfg.density_lb_ft3).into());
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::resolve_override_path;

    #[test]
    fn resolve_override_path_prefers_exe_directory() {
        let exe = Some(PathBuf::from("/tmp/steelcal/SimpleSteelCalculator"));
        assert_eq!(
            resolve_override_path(exe.as_deref()),
            Path::new("/tmp/steelcal/assets/gauge_tables.override.json")
        );
    }

    #[test]
    fn resolve_override_path_falls_back_to_cwd_relative_assets() {
        assert_eq!(
            resolve_override_path(None),
            Path::new("assets/gauge_tables.override.json")
        );
    }
}
