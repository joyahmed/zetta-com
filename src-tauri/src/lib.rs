mod audio;
mod config;
mod discovery;
mod net;
mod session;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Shortcut, ShortcutState};

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

/// True only while the push-to-talk key is held.
///
/// It lives outside the session and outlives it, because the shortcut is
/// registered once at launch: the key must not stop working because somebody
/// pressed Stop and Start.
struct Ptt(Arc<AtomicBool>);

/// The push-to-talk key. F8 by default — a single key, so it can be *held*,
/// and one nothing else tends to claim. Reassignable in #13.
const PTT_KEY: Code = Code::F8;

/// Commands return `Result<_, String>` rather than `anyhow::Result` because
/// anyhow's error type is not serialisable across the IPC boundary. The
/// conversion belongs here, at the edge, not inside `net`.
#[tauri::command]
fn net_start(
    app: tauri::AppHandle,
    state: State<NetState>,
    ptt: State<Ptt>,
    port: u16,
    peer: String,
) -> Result<(), String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    // Drop any existing transport before binding, or starting on the same port
    // fails with "address already in use" against ourselves.
    *guard = None;
    *guard =
        Some(session::start(port, &peer, ptt.0.clone()).map_err(|e| format!("{e:#}"))?);

    // Saved only after a successful bind, so a setting that cannot work is
    // never the one restored at next launch.
    if let Err(e) = config::save(&app, &config::Config { port, peer }) {
        eprintln!("[config] not saved: {e:#}");
    }
    Ok(())
}

/// Everyone discovered on the LAN, live or recently gone. Empty when the
/// transport is stopped, and also on a network that filters mDNS — which is a
/// normal condition, not an error, and the reason peers can be added by hand.
#[tauri::command]
fn net_peers(state: State<NetState>) -> Result<Vec<discovery::Peer>, String> {
    Ok(state
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .map(|s| s.peers())
        .unwrap_or_default())
}

/// Send a line of text to everyone live.
///
/// A separate command from anything to do with voice, on purpose: in v1 one
/// keypress both pinged somebody and opened the mic to them, and when the ping
/// arrived and the voice did not there was no way to tell which half had
/// failed. Same socket underneath, separate action on top.
#[tauri::command]
fn send_text(state: State<NetState>, text: String) -> Result<(), String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(());
    }
    match state.0.lock().map_err(|e| e.to_string())?.as_ref() {
        Some(s) => {
            s.send_text(text);
            Ok(())
        }
        None => Err("Not running — press Start first.".into()),
    }
}

/// The message log, newest last. Polled with everything else.
#[tauri::command]
fn messages(state: State<NetState>) -> Result<Vec<net::Message>, String> {
    Ok(state
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .map(|s| s.messages())
        .unwrap_or_default())
}

/// Whether the push-to-talk key is down right now. Polled with everything else
/// rather than pushed as an event: the UI already asks four times a second, and
/// a second channel would be one more thing to keep in agreement with reality.
#[tauri::command]
fn ptt_held(ptt: State<Ptt>) -> bool {
    ptt.0.load(Ordering::Relaxed)
}

/// The name this machine advertises itself under, so the UI can show you which
/// entry in everyone else's roster is you.
#[tauri::command]
fn local_name() -> String {
    discovery::local_name()
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
    // Created before the builder because both the shortcut handler and the
    // session need it, and the handler is installed while the app is being
    // configured rather than after.
    let ptt = Arc::new(AtomicBool::new(false));
    let ptt_for_handler = ptt.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |_app, shortcut, event| {
                    if shortcut.matches(tauri_plugin_global_shortcut::Modifiers::empty(), PTT_KEY) {
                        // Held, not toggled. The key going up has to be as
                        // reliable as it going down, or a missed release
                        // leaves the microphone open with nothing on screen
                        // to say so.
                        ptt_for_handler.store(
                            matches!(event.state(), ShortcutState::Pressed),
                            Ordering::Relaxed,
                        );
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            greet, net_start, net_stop, net_stats, net_peers, local_name, config_get,
            ptt_held, send_text, messages
        ])
        .setup(move |app| {
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
            // Register the key itself. A global shortcut that loses a
            // registration race to another application does nothing and says
            // nothing, which is a v1-class silent failure, so the result is
            // reported rather than discarded.
            match app.global_shortcut().register(Shortcut::new(None, PTT_KEY)) {
                Ok(()) => eprintln!("[ptt] hold {PTT_KEY:?} to talk"),
                Err(e) => eprintln!("[ptt] could not register {PTT_KEY:?}: {e}"),
            }
            app.manage(Ptt(ptt.clone()));

            let session = match config::load(app.handle()) {
                Some(cfg) => match session::start(cfg.port, &cfg.peer, ptt.clone()) {
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
