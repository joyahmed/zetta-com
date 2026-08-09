mod audio;
mod config;
mod net;
mod session;

use std::sync::Mutex;

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, State, WindowEvent,
};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// The running session, or nothing. `session::Session` stops audio and the
/// socket on drop, so setting this to `None` is the whole of `net_stop`.
struct NetState(Mutex<Option<session::Session>>);

/// Commands return `Result<_, String>` rather than `anyhow::Result` because
/// anyhow's error type is not serialisable across the IPC boundary. The
/// conversion belongs here, at the edge, not inside `net`.
#[tauri::command]
fn net_start(
    app: tauri::AppHandle,
    state: State<NetState>,
    port: u16,
    peer: String,
) -> Result<(), String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    // Drop any existing transport before binding, or starting on the same port
    // fails with "address already in use" against ourselves.
    *guard = None;
    *guard = Some(session::start(port, &peer).map_err(|e| format!("{e:#}"))?);

    // Saved only after a successful bind, so a setting that cannot work is
    // never the one restored at next launch.
    if let Err(e) = config::save(&app, &config::Config { port, peer }) {
        eprintln!("[config] not saved: {e:#}");
    }
    Ok(())
}

/// What the app will auto-start with, or `None` if it has never been
/// configured. The UI reads this to fill its fields rather than keeping its own
/// copy, so there is one source of truth for what this machine does.
#[tauri::command]
fn config_get(app: tauri::AppHandle) -> Option<config::Config> {
    config::load(&app)
}

#[tauri::command]
fn net_stop(state: State<NetState>) -> Result<(), String> {
    *state.0.lock().map_err(|e| e.to_string())? = None;
    Ok(())
}

/// `None` when the transport is stopped. The UI polls this on a timer rather
/// than receiving an event per packet: at 50 packets a second per peer, one IPC
/// message each would melt the webview, and polling decouples the redraw rate
/// from the packet rate for free.
#[tauri::command]
fn net_stats(state: State<NetState>) -> Result<Option<net::Stats>, String> {
    Ok(state
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .map(|h| h.stats()))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet, net_start, net_stop, net_stats, config_get
        ])
        .setup(|app| {
            let show = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Zetta Com")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => reveal(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        reveal(tray.app_handle());
                    }
                })
                .build(app)?;

            // Auto-start from the saved settings. A PC whose job is to listen
            // must come up receiving without anyone clicking anything: it may
            // be in another room, it may have no microphone, and it may have
            // nobody sitting at it. Requiring a button press made a listener
            // depend on a person, which is the one thing v1 got right.
            //
            // Settings deliberately come from disk rather than the webview's
            // localStorage, because binding has to happen before any window has
            // loaded, and from a file rather than the environment, because two
            // ways to configure one thing guarantees an afternoon spent on
            // "why is it using the other port".
            let session = match config::load(app.handle()) {
                Some(cfg) => match session::start(cfg.port, &cfg.peer) {
                    Ok(s) => {
                        eprintln!("[net] auto-started on {} -> {}", cfg.port, cfg.peer);
                        Some(s)
                    }
                    Err(e) => {
                        // Not fatal: the window still opens, stopped, so the
                        // settings can be corrected by hand.
                        eprintln!("[net] auto-start failed: {e:#}");
                        None
                    }
                },
                None => None,
            };
            app.manage(NetState(Mutex::new(session)));

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Bring the main window back from the tray. Both the menu item and a
/// left-click need this, so it lives outside the closures.
fn reveal(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}
