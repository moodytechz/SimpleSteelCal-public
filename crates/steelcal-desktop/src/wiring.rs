use std::cell::RefCell;
use std::rc::Rc;

use slint::ComponentHandle;
use steelcal_core::history::SessionHistory;

use crate::handlers::{
    calculate_coil, calculate_pricing, calculate_scrap, calculate_sheet, clear_coil, clear_scrap,
    clear_sheet, copy_to_actual, copy_to_ending, export_history, open_about, open_config_editor,
    open_config_location, open_help, open_history, recall_history_entry, refresh_history_filter,
    restore_config_defaults, revert_config_editor, save_config_editor, select_history_entry,
    validate_config_editor,
};
use crate::ui_helpers::{
    current_table_name, find_best_gauge_match, focus_gauge_field, gauge_keys_for_table,
    slint_model_to_vec, FilteredIndices,
};
use crate::{AppState, AppWindow};

pub fn register_focus_callbacks(app: &AppWindow) {
    let app_weak = app.as_weak();
    app.on_advance_focus(move || {
        let Some(app) = app_weak.upgrade() else {
            return;
        };

        let tab: slint::SharedString = slint::platform::Key::Tab.into();
        let window = app.window();
        window.dispatch_event(slint::platform::WindowEvent::KeyPressed { text: tab.clone() });
        window.dispatch_event(slint::platform::WindowEvent::KeyReleased { text: tab });
    });
}

pub fn register_gauge_callbacks(app: &AppWindow, state: &Rc<RefCell<AppState>>) {
    {
        let app_weak = app.as_weak();
        let state = Rc::clone(state);
        app.on_table_changed(move |idx| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            let st = state.borrow();
            let names = slint_model_to_vec(app.get_table_names());
            let table_name = names.get(idx as usize).cloned().unwrap_or_default();
            let keys = gauge_keys_for_table(&st.tables, &table_name);
            let model: Vec<slint::SharedString> = keys.iter().map(|s| s.into()).collect();
            app.set_gauge_keys(slint::ModelRc::from(model.as_slice()));
            app.set_selected_gauge_index(0);
            app.set_gauge_text(keys.first().cloned().unwrap_or_default().into());
        });
    }

    {
        let app_weak = app.as_weak();
        let state = Rc::clone(state);
        app.on_gauge_edited(move |text| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };

            let st = state.borrow();
            let table_name = current_table_name(&app);
            let keys = gauge_keys_for_table(&st.tables, &table_name);
            if let Some(index) = find_best_gauge_match(&keys, text.as_str()) {
                app.set_selected_gauge_index(index as i32);
            } else {
                app.set_selected_gauge_index(-1);
            }
        });
    }

    {
        let app_weak = app.as_weak();
        let state = Rc::clone(state);
        app.on_gauge_accept(move |text| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };

            let st = state.borrow();
            let table_name = current_table_name(&app);
            let keys = gauge_keys_for_table(&st.tables, &table_name);
            if let Some(index) = find_best_gauge_match(&keys, text.as_str()) {
                app.set_selected_gauge_index(index as i32);
                if let Some(key) = keys.get(index) {
                    app.set_gauge_text(key.clone().into());
                }
            }
        });
    }
}

pub fn register_calculation_callbacks(
    app: &AppWindow,
    state: &Rc<RefCell<AppState>>,
    history: &Rc<RefCell<SessionHistory>>,
) {
    {
        let app_weak = app.as_weak();
        let state = Rc::clone(state);
        let history = Rc::clone(history);
        app.on_calculate(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            calculate_sheet(&app, &state, &history);
        });
    }

    {
        let app_weak = app.as_weak();
        let state = Rc::clone(state);
        app.on_calculate_pricing(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            calculate_pricing(&app, &state);
        });
    }

    {
        let app_weak = app.as_weak();
        let history = Rc::clone(history);
        app.on_calculate_coil(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            calculate_coil(&app, &history);
        });
    }

    {
        let app_weak = app.as_weak();
        let history = Rc::clone(history);
        app.on_calculate_scrap(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            calculate_scrap(&app, &history);
        });
    }

    {
        let app_weak = app.as_weak();
        let state = Rc::clone(state);
        app.on_clear_sheet(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            clear_sheet(&app, &state);
            focus_gauge_field(&app);
        });
    }

    {
        let app_weak = app.as_weak();
        let state = Rc::clone(state);
        app.on_clear_coil(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            clear_coil(&app, &state);
        });
    }

    {
        let app_weak = app.as_weak();
        app.on_clear_scrap(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            clear_scrap(&app);
        });
    }

    {
        let app_weak = app.as_weak();
        app.on_copy_to_actual(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            copy_to_actual(&app);
        });
    }

    {
        let app_weak = app.as_weak();
        app.on_copy_to_ending(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            copy_to_ending(&app);
        });
    }
}

pub fn register_menu_callbacks(app: &AppWindow) {
    app.on_menu_exit(move || {
        let _ = slint::quit_event_loop();
    });

    {
        let app_weak = app.as_weak();
        app.on_menu_open_help(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            open_help(&app);
        });
    }

    {
        let app_weak = app.as_weak();
        app.on_menu_open_about(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            open_about(&app);
        });
    }
}

pub fn register_config_callbacks(
    app: &AppWindow,
    state: &Rc<RefCell<AppState>>,
    original: &Rc<RefCell<String>>,
) {
    {
        let app_weak = app.as_weak();
        let original = Rc::clone(original);
        app.on_menu_open_config_editor(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            open_config_editor(&app, &original);
        });
    }

    {
        let app_weak = app.as_weak();
        app.on_config_editor_validate(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            validate_config_editor(&app);
        });
    }

    {
        let app_weak = app.as_weak();
        let state = Rc::clone(state);
        let original = Rc::clone(original);
        app.on_config_editor_save(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            save_config_editor(&app, &state, &original);
        });
    }

    {
        let app_weak = app.as_weak();
        let original = Rc::clone(original);
        app.on_config_editor_revert(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            revert_config_editor(&app, &original);
        });
    }

    {
        let app_weak = app.as_weak();
        app.on_config_editor_restore_defaults(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            restore_config_defaults(&app);
        });
    }

    {
        let app_weak = app.as_weak();
        app.on_config_editor_open_location(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            open_config_location(&app);
        });
    }
}

pub fn register_history_callbacks(
    app: &AppWindow,
    state: &Rc<RefCell<AppState>>,
    history: &Rc<RefCell<SessionHistory>>,
    filtered: &Rc<RefCell<FilteredIndices>>,
) {
    {
        let app_weak = app.as_weak();
        let history = Rc::clone(history);
        let filtered = Rc::clone(filtered);
        app.on_menu_open_history(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            open_history(&app, &history, &filtered);
        });
    }

    {
        let app_weak = app.as_weak();
        let history = Rc::clone(history);
        let filtered = Rc::clone(filtered);
        app.on_history_filter_changed(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            refresh_history_filter(&app, &history, &filtered);
        });
    }

    {
        let app_weak = app.as_weak();
        let history = Rc::clone(history);
        let filtered = Rc::clone(filtered);
        app.on_history_entry_selected(move |idx| {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            select_history_entry(&app, &history, &filtered, idx);
        });
    }

    {
        let app_weak = app.as_weak();
        let history = Rc::clone(history);
        let filtered = Rc::clone(filtered);
        let state = Rc::clone(state);
        app.on_history_recall(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            recall_history_entry(&app, &history, &filtered, &state);
        });
    }

    {
        let app_weak = app.as_weak();
        let history = Rc::clone(history);
        let filtered = Rc::clone(filtered);
        app.on_history_export(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            export_history(&app, &history, &filtered);
        });
    }
}
