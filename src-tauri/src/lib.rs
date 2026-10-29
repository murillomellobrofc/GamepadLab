//! GamepadLab desktop shell: wires the input, calibration, profiles and
//! haptics crates to the web UI through Tauri commands and events.

mod calibrate;
mod commands;
mod emitter;
mod error;
mod haptics;
mod logging;
mod platform;
mod state;

use gamepadlab_input::InputService;
use state::AppState;
use std::sync::atomic::AtomicU32;
use std::sync::mpsc;
use std::sync::Arc;
use tauri::{Manager, RunEvent};

pub fn run() {
    logging::init(log::LevelFilter::Info);

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let handle = app.handle().clone();
            if let Ok(dir) = handle.path().app_log_dir() {
                logging::attach_file(&dir);
            }
            log::info!(
                "GamepadLab {} starting on {} {} ({})",
                env!("CARGO_PKG_VERSION"),
                std::env::consts::OS,
                platform::os_version(),
                std::env::consts::ARCH
            );
            let data_dir = commands::data_dir(&handle);
            let _ = std::fs::create_dir_all(&data_dir);

            let (tx, rx) = mpsc::channel();
            let input = InputService::start_default(tx);
            let sink = Arc::new(haptics::ServiceSink(Default::default()));
            let engine = haptics::start(handle.clone(), sink.clone());
            let state = Arc::new(AppState {
                input,
                haptics: engine,
                ui_rate: AtomicU32::new(60),
                data_dir,
            });
            let _ = sink.0.set(Arc::downgrade(&state));
            app.manage(state.clone());
            emitter::spawn_event_forwarder(handle.clone(), rx);
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
        ])
        .build(tauri::generate_context!())
        .expect("error while building GamepadLab");

    app.run(|_app, event| {
        if let RunEvent::Exit = event {
            log::info!("GamepadLab exiting");
            log::logger().flush();
        }
    });
}
