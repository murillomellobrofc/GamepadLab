//! GamepadLab desktop shell: wires the input, calibration, profiles and
//! haptics crates to the web UI through Tauri commands and events.

mod calibrate;
mod cmd_calibration;
mod cmd_files;
mod cmd_firmware;
mod cmd_history;
mod commands;
mod emitter;
mod error;
mod haptics;
mod history;
mod logging;
mod overlay;
mod platform;
mod state;

use gamepadlab_input::InputService;
use state::AppState;
use std::sync::atomic::AtomicU32;
use std::sync::mpsc;
use std::sync::Arc;
use tauri::{Manager, RunEvent, Runtime};

/// Everything except platform plugins: state, background threads and
/// commands. Shared by the real app and the IPC tests (mock runtime).
pub fn configure<R: Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder
        .manage(overlay::OverlayState::default())
        .setup(|app| {
            let handle = app.handle().clone();
            let data_dir = commands::data_dir(&handle);
            let log_dir = if std::env::var_os("GAMEPADLAB_DATA_DIR").is_some() {
                Some(data_dir.join("logs"))
            } else {
                handle.path().app_log_dir().ok()
            };
            if let Some(dir) = log_dir {
                logging::attach_file(&dir);
            }
            log::info!(
                "GamepadLab {} starting on {} {} ({})",
                handle.package_info().version,
                std::env::consts::OS,
                platform::os_version(),
                std::env::consts::ARCH
            );
            let _ = std::fs::create_dir_all(&data_dir);

            let (tx, rx) = mpsc::channel();
            // GAMEPADLAB_NO_HARDWARE=1 keeps only the demo backend (CI, tests).
            let input = if std::env::var_os("GAMEPADLAB_NO_HARDWARE").is_some() {
                InputService::start(Default::default(), Vec::new(), tx)
            } else {
                InputService::start_default(tx)
            };
            let sink = Arc::new(haptics::ServiceSink(Default::default()));
            let engine = haptics::start(handle.clone(), sink.clone());
            let history = match history::History::open(&data_dir.join("history.sqlite")) {
                Ok(h) => Some(h),
                Err(e) => {
                    log::error!("history database unavailable: {e}");
                    None
                }
            };
            let state = Arc::new(AppState {
                input,
                haptics: engine,
                profiles: gamepadlab_profiles::ProfileStore::new(data_dir.join("profiles")),
                ui_rate: AtomicU32::new(60),
                data_dir,
                calibrations: Default::default(),
                captures: Default::default(),
                pending_capture: Default::default(),
                history,
            });
            let _ = sink.0.set(Arc::downgrade(&state));
            app.manage(state.clone());
            emitter::spawn_event_forwarder(handle.clone(), rx, state.clone());
            emitter::spawn_frame_emitter(handle, state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::list_controllers,
            commands::platform_warnings,
            commands::set_ui_rate,
            commands::reset_minmax,
            commands::set_demo,
            commands::demo_scenario,
            commands::demo_manual,
            commands::enable_sensors,
            commands::diagnostics,
            commands::set_log_level,
            commands::log_frontend,
            commands::open_privacy_settings,
            commands::rumble_play,
            commands::rumble_sequence,
            commands::rumble_motor_test,
            commands::rumble_stop,
            commands::set_trigger_effects,
            commands::set_lightbar,
            commands::set_player_leds,
            cmd_calibration::capture_start,
            cmd_calibration::capture_stop,
            cmd_calibration::capture_clear,
            cmd_calibration::analyze_stick,
            cmd_calibration::analyze_trigger_cmd,
            cmd_calibration::set_active_calibration,
            cmd_calibration::profiles_load,
            cmd_calibration::profiles_create,
            cmd_calibration::profiles_update,
            cmd_calibration::profiles_rename,
            cmd_calibration::profiles_duplicate,
            cmd_calibration::profiles_delete,
            cmd_calibration::profiles_set_active,
            cmd_calibration::profiles_reset,
            cmd_calibration::profiles_export,
            cmd_calibration::profiles_import,
            cmd_calibration::controller_profile_key,
            cmd_files::save_text_file,
            cmd_files::save_binary_file,
            cmd_files::read_text_file,
            cmd_firmware::firmware,
            cmd_firmware::firmware_switch_propose,
            cmd_history::history_add,
            cmd_history::history_list,
            cmd_history::history_delete,
            cmd_history::history_clear,
            overlay::overlay_open,
            overlay::overlay_update,
            overlay::overlay_close,
            overlay::overlay_state,
            overlay::overlay_is_open,
        ])
}

pub fn run() {
    logging::init(log::LevelFilter::Info);

    let app = configure(
        tauri::Builder::default()
            .plugin(tauri_plugin_dialog::init())
            .plugin(tauri_plugin_process::init())
            .plugin(tauri_plugin_updater::Builder::new().build()),
    )
    .build(tauri::generate_context!())
    .expect("error while building GamepadLab");

    app.run(|_app, event| {
        if let RunEvent::Exit = event {
            log::info!("GamepadLab exiting");
            log::logger().flush();
        }
    });
}
