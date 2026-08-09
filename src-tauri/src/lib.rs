mod audio;
mod net;

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

/// The running transport, or nothing. `net::Handle` stops on drop, so setting
/// this to `None` is the whole of `net_stop`.
struct NetState(Mutex<Option<net::Handle>>);

/// Commands return `Result<_, String>` rather than `anyhow::Result` because
/// anyhow's error type is not serialisable across the IPC boundary. The
/// conversion belongs here, at the edge, not inside `net`.
#[tauri::command]
fn net_start(state: State<NetState>, port: u16, peer: String) -> Result<(), String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    // Drop any existing transport before binding, or starting on the same port
    // fails with "address already in use" against ourselves.
    *guard = None;
    *guard = Some(net::start(port, &peer).map_err(|e| format!("{e:#}"))?);
    Ok(())
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
            greet, net_start, net_stop, net_stats
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

            match audio::start() {
                Ok(handle) => {
                    app.manage(handle);
                }
                Err(e) => eprintln!("[audio] failed to start: {e:#}"),
            }

            // The transport is started from the UI, not from here. Deliberately
            // no environment-variable fallback: two ways to configure one thing
            // guarantees an afternoon spent on "why is it using the other port".
            app.manage(NetState(Mutex::new(None)));

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
