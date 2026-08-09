mod audio;
mod config;
mod discovery;
mod net;
mod session;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

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

/// What a registered shortcut does.
#[derive(Clone)]
enum Action {
    /// Held, not toggled.
    Talk,
    /// Fires a canned message.
    Preset(String),
}

/// The registered shortcuts and what each one means, shared with the handler.
///
/// A list rather than a map because `Shortcut` is compared, not hashed, and
/// there are never more than a handful.
type Bindings = Arc<Mutex<Vec<(Shortcut, Action)>>>;

/// Register a shortcut and remember what it does.
///
/// A global shortcut that loses a registration race to another application does
/// nothing and says nothing, which is a v1-class silent failure — so both the
/// parse and the registration report rather than being discarded.
fn bind(app: &tauri::AppHandle, bindings: &Bindings, spec: &str, action: Action) {
    if spec.trim().is_empty() {
        return;
    }
    let shortcut: Shortcut = match spec.parse() {
        Ok(s) => s,
        Err(e) => return eprintln!("[keys] {spec:?} is not a shortcut: {e}"),
    };
    match app.global_shortcut().register(shortcut) {
        Ok(()) => {
            let mut list = match bindings.lock() {
                Ok(l) => l,
                Err(e) => e.into_inner(),
            };
            list.push((shortcut, action));
            eprintln!("[keys] {spec} registered");
        }
        Err(e) => eprintln!("[keys] {spec} could not be registered: {e}"),
    }
}

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
    // A port on the command line wins, so a second instance started for testing
    // cannot be dragged back onto the first one's port by the shared config.
    let port = config::port_override().unwrap_or(port);
    let manual = config::load(&app).map(|c| c.manual).unwrap_or_default();
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    // Drop any existing transport before binding, or starting on the same port
    // fails with "address already in use" against ourselves.
    *guard = None;
    *guard = Some(
        session::start(port, &peer, &manual, ptt.0.clone()).map_err(|e| format!("{e:#}"))?,
    );

    // Saved only after a successful bind, so a setting that cannot work is
    // never the one restored at next launch.
    // Merged into whatever is already there rather than replacing it, or
    // pressing Start would silently wipe the presets and the key bindings.
    // Not saved when the port came from the command line: an override is for
    // this run, and writing it back would move the other instance next launch.
    if config::port_override().is_none() {
        let mut cfg = config::load(&app).unwrap_or_default();
        cfg.port = port;
        cfg.peer = peer;
        cfg.manual = manual;
        if let Err(e) = config::save(&app, &cfg) {
            eprintln!("[config] not saved: {e:#}");
        }
    }
    Ok(())
}

/// Add or remove a hand-entered peer, then rebind so it takes effect.
///
/// Restarting rather than mutating the live list: the roster changes rarely,
/// and a rebind is one code path instead of two that have to agree.
#[tauri::command]
fn manual_peers(
    app: tauri::AppHandle,
    state: State<NetState>,
    ptt: State<Ptt>,
    add: Option<String>,
    remove: Option<String>,
) -> Result<Vec<String>, String> {
    let mut cfg = config::load(&app).unwrap_or_default();

    if let Some(entry) = add {
        let entry = entry.trim().to_string();
        if entry.is_empty() {
            return Err("Enter an address like 192.168.0.42:9001.".into());
        }
        // Checked before it is stored, so a typo is refused while you are
        // looking at it rather than logged quietly on the next restart.
        net::resolve_v4(&entry).map_err(|e| format!("{e:#}"))?;
        if !cfg.manual.contains(&entry) {
            cfg.manual.push(entry);
        }
    }
    if let Some(entry) = remove {
        cfg.manual.retain(|e| *e != entry);
    }

    config::save(&app, &cfg).map_err(|e| format!("{e:#}"))?;

    // Only rebind if it was already running; otherwise the list is simply
    // saved for whenever Start is pressed.
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    if guard.is_some() {
        *guard = None;
        *guard = Some(
            session::start(cfg.port, &cfg.peer, &cfg.manual, ptt.0.clone())
                .map_err(|e| format!("{e:#}"))?,
        );
    }
    Ok(cfg.manual)
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

/// Aim voice and text at one machine, or at everyone when `addr` is null.
///
/// This is the whole of per-person targeting: every send is already a unicast
/// to each recipient in turn, so addressing one person is a shorter list rather
/// than a different protocol.
#[tauri::command]
fn set_target(state: State<NetState>, addr: Option<String>) -> Result<(), String> {
    let parsed = match addr.as_deref().map(str::trim).filter(|a| !a.is_empty()) {
        Some(a) => Some(net::resolve_v4(a).map_err(|e| format!("{e:#}"))?),
        None => None,
    };
    if let Some(s) = state.0.lock().map_err(|e| e.to_string())?.as_ref() {
        s.set_target(parsed);
    }
    Ok(())
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
    let bindings: Bindings = Arc::new(Mutex::new(Vec::new()));

    let ptt_for_handler = ptt.clone();
    let bindings_for_handler = bindings.clone();

    let mut builder = tauri::Builder::default();

    // One instance, unless a port was given on the command line.
    //
    // Closing the window hides to the tray, so a running app is invisible — and
    // every later launch would take a second copy that cannot bind the port or
    // claim the hotkeys, and would look simply broken. v1 hit this too: two
    // copies of the listener were the cause of its reported echo, and a mutex
    // was the fix there as this is here.
    //
    // The override is the exception on purpose. Running two deliberately, for
    // testing, is the one time a second instance is wanted.
    if config::port_override().is_none() {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            eprintln!("[app] already running — showing the existing window");
            reveal(app);
        }));
    }

    builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    let action = {
                        let list = match bindings_for_handler.lock() {
                            Ok(l) => l,
                            Err(e) => e.into_inner(),
                        };
                        list.iter()
                            .find(|(s, _)| s == shortcut)
                            .map(|(_, a)| a.clone())
                    };
                    let Some(action) = action else { return };

                    match action {
                        // Held, not toggled. The key going up has to be as
                        // reliable as it going down, or a missed release
                        // leaves the microphone open with nothing on screen
                        // to say so.
                        Action::Talk => ptt_for_handler.store(
                            matches!(event.state(), ShortcutState::Pressed),
                            Ordering::Relaxed,
                        ),
                        // On press only. Firing again on release would send
                        // every canned message twice.
                        Action::Preset(text) => {
                            if matches!(event.state(), ShortcutState::Pressed) {
                                if let Some(s) =
                                    app.state::<NetState>().0.lock().ok().and_then(|g| {
                                        g.as_ref().map(|s| s.send_text(&text))
                                    })
                                {
                                    let _ = s;
                                }
                            }
                        }
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            greet, net_start, net_stop, net_stats, net_peers, local_name, config_get,
            ptt_held, send_text, messages, manual_peers, set_target
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
            let cfg = config::load(app.handle()).unwrap_or_default();
            let handle = app.handle().clone();
            bind(&handle, &bindings, &cfg.talk_shortcut, Action::Talk);
            for p in &cfg.presets {
                bind(
                    &handle,
                    &bindings,
                    &p.shortcut,
                    Action::Preset(p.text.clone()),
                );
            }
            app.manage(Ptt(ptt.clone()));

            let session = match config::load(app.handle()) {
                Some(cfg) => {
                    // Resolved once and reported, rather than logging the saved
                    // value while binding the overridden one — a log that
                    // misreports which port is in use is worse than no log.
                    let port = config::port_override().unwrap_or(cfg.port);
                    match session::start(port, &cfg.peer, &cfg.manual, ptt.clone()) {
                    Ok(s) => {
                        eprintln!("[net] auto-started on {port} -> {}", cfg.peer);
                        Some(s)
                    }
                    Err(e) => {
                        // Not fatal: the window still opens, stopped, so the
                        // settings can be corrected by hand.
                        eprintln!("[net] auto-start failed: {e:#}");
                        None
                    }
                    }
                }
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
