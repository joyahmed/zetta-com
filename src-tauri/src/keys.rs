//! Global shortcuts: what they do, registering them, and reporting the ones
//! that failed.
//!
//! Kept apart from the commands because these fire whether or not a window is
//! open — that is the whole point of a global key — so they answer to the app,
//! not to the frontend.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tauri::{Emitter, Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::discovery;
use crate::state::NetState;

#[derive(Clone)]
pub enum Action {
    /// Held, not toggled.
    Talk,
    /// Fires a canned message.
    Preset(String),
    /// Hold to talk to one machine: the roster entry at this position, counting
    /// from 1. Releasing stops transmitting and leaves the target where it is,
    /// so the next thing you type goes to the same person you just spoke to.
    TalkTo(usize),
    /// Aim at that machine and bring the window up ready to type.
    MessageTo(usize),
    /// Hold to talk to the whole room, whoever was selected.
    TalkAll,
    /// Aim at the whole room and bring the window up ready to type.
    MessageAll,
    /// Open the window with the add-a-PC field ready.
    AddPc,
    /// Show the list of every key and what it does.
    ShowShortcuts,
    /// Bring the window up from the tray. v1 stamped this on a Start Menu
    /// shortcut and let Explorer dispatch it; here it is a real global key, so
    /// it works whether or not anything is pinned.
    ShowWindow,
}

/// One row of the shortcut list the UI shows.
///
/// Carries whether it registered, because a global shortcut that lost a race to
/// another application does nothing and says nothing — and a list that only
/// shows intentions is worse than none, since it looks like a promise.
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutInfo {
    pub label: String,
    pub keys: String,
    pub registered: bool,
}

pub type ShortcutList = Arc<Mutex<Vec<ShortcutInfo>>>;

pub struct Shortcuts(pub ShortcutList);

/// Every key, what it does, and whether it actually took.
#[tauri::command]
pub fn shortcuts(state: State<Shortcuts>) -> Result<Vec<ShortcutInfo>, String> {
    let l = state.0.lock().map_err(|e| e.to_string())?;
    Ok(l.clone())
}

/// The roster entry at a one-based position, in the order the UI shows them.
///
/// Position rather than name, because the keys have to be registered at launch
/// and the roster does not exist yet — nobody has been discovered. It also
/// means the keys keep working when somebody's PC is renamed.
fn peer_at(app: &tauri::AppHandle, slot: usize) -> Option<discovery::Peer> {
    let state = app.state::<NetState>();
    let guard = state.0.lock().ok()?;
    let peers = guard.as_ref()?.peers();
    peers.into_iter().nth(slot.checked_sub(1)?)
}

/// The registered shortcuts and what each one means, shared with the handler.
///
/// A list rather than a map because `Shortcut` is compared, not hashed, and
/// there are never more than a handful.
pub type Bindings = Arc<Mutex<Vec<(Shortcut, Action)>>>;

/// Register a shortcut and remember what it does.
///
/// A global shortcut that loses a registration race to another application does
/// nothing and says nothing, which is a v1-class silent failure — so both the
/// parse and the registration report rather than being discarded.
pub fn bind(
    app: &tauri::AppHandle,
    bindings: &Bindings,
    listing: &ShortcutList,
    label: &str,
    spec: &str,
    action: Action,
) {
    if spec.trim().is_empty() {
        return;
    }
    let record = |registered: bool| {
        let mut l = match listing.lock() {
            Ok(l) => l,
            Err(e) => e.into_inner(),
        };
        l.push(ShortcutInfo {
            label: label.to_string(),
            keys: spec.replace("CommandOrControl", "Ctrl").replace("Digit", ""),
            registered,
        });
    };

    let shortcut: Shortcut = match spec.parse() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[keys] {spec:?} is not a shortcut: {e}");
            record(false);
            return;
        }
    };
    match app.global_shortcut().register(shortcut) {
        Ok(()) => {
            let mut list = match bindings.lock() {
                Ok(l) => l,
                Err(e) => e.into_inner(),
            };
            list.push((shortcut, action));
            record(true);
        }
        Err(e) => {
            eprintln!("[keys] {spec} could not be registered: {e}");
            record(false);
        }
    }
}


/// Bring the main window back from the tray. Both the tray menu and several
/// keys need it, so it lives beside them.
pub fn reveal(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// Point everything at one machine, or at everyone when `None`.
fn aim(app: &tauri::AppHandle, addr: Option<std::net::SocketAddr>) {
    if let Ok(g) = app.state::<NetState>().0.lock() {
        if let Some(s) = g.as_ref() {
            s.set_target(addr);
        }
    }
}

/// What a key press actually does.
///
/// Separated from registration so the behaviour can be read in one place
/// without the ceremony of installing it.
pub fn dispatch(
    app: &tauri::AppHandle,
    ptt: &Arc<AtomicBool>,
    action: Action,
    pressed: bool,
) {
    match action {
        // Held, not toggled. The key going up has to be as reliable as it
        // going down, or a missed release leaves the microphone open with
        // nothing on screen to say so.
        Action::Talk => ptt.store(pressed, Ordering::Relaxed),

        // On press only. Firing again on release would send everything twice.
        Action::Preset(text) => {
            if pressed {
                if let Ok(g) = app.state::<NetState>().0.lock() {
                    if let Some(s) = g.as_ref() {
                        s.send_text(&text);
                    }
                }
            }
        }

        // Aim, then talk, for as long as it is held. Aiming on press means the
        // very first frame already goes to the right person.
        Action::TalkTo(slot) => {
            if pressed {
                match peer_at(app, slot) {
                    Some(p) => {
                        aim(app, Some(p.addr));
                        eprintln!("[keys] talking to {}", p.name);
                    }
                    // Nobody there: say so rather than opening the microphone
                    // to whoever happened to be selected.
                    None => return eprintln!("[keys] no PC at position {slot}"),
                }
            }
            ptt.store(pressed, Ordering::Relaxed);
        }
        Action::MessageTo(slot) => {
            if pressed {
                match peer_at(app, slot) {
                    Some(p) => {
                        aim(app, Some(p.addr));
                        reveal(app);
                    }
                    None => eprintln!("[keys] no PC at position {slot}"),
                }
            }
        }

        Action::TalkAll => {
            if pressed {
                aim(app, None);
            }
            ptt.store(pressed, Ordering::Relaxed);
        }
        Action::MessageAll => {
            if pressed {
                aim(app, None);
                reveal(app);
            }
        }

        // These only ask the window to show something. The work is the
        // frontend's, so the key emits an event rather than reaching into it.
        Action::AddPc => {
            if pressed {
                reveal(app);
                let _ = app.emit("open-add-pc", ());
            }
        }
        Action::ShowShortcuts => {
            if pressed {
                reveal(app);
                let _ = app.emit("show-shortcuts", ());
            }
        }
        Action::ShowWindow => {
            if pressed {
                reveal(app);
            }
        }
    }
}

/// Register every key the app uses.
///
/// A position rather than a name for the per-PC keys, because these are
/// registered at launch when nobody has been discovered yet — and because a
/// position keeps working when a PC is renamed.
pub fn register_all(
    app: &tauri::AppHandle,
    bindings: &Bindings,
    listing: &ShortcutList,
    cfg: &crate::config::Config,
) {
    bind(
        app,
        bindings,
        listing,
        "Talk to whoever is selected",
        &cfg.talk_shortcut,
        Action::Talk,
    );
    for p in &cfg.presets {
        bind(
            app,
            bindings,
            listing,
            &format!("Send \u{201c}{}\u{201d}", p.label),
            &p.shortcut,
            Action::Preset(p.text.clone()),
        );
    }

    for (label, spec, action) in [
        (
            "Talk to everyone",
            "CommandOrControl+Alt+Digit0",
            Action::TalkAll,
        ),
        (
            "Message everyone",
            "CommandOrControl+Shift+Digit0",
            Action::MessageAll,
        ),
        ("Add a PC", "CommandOrControl+Alt+KeyA", Action::AddPc),
        (
            "Show shortcuts",
            "CommandOrControl+Alt+KeyK",
            Action::ShowShortcuts,
        ),
        (
            "Open the window",
            "CommandOrControl+Alt+KeyT",
            Action::ShowWindow,
        ),
    ] {
        bind(app, bindings, listing, label, spec, action);
    }

    // Nine of each, which is as many as a row of number keys gives and more
    // people than one person directs at once.
    for slot in 1..=9usize {
        bind(
            app,
            bindings,
            listing,
            &format!("Talk to PC {slot}"),
            &format!("CommandOrControl+Alt+Digit{slot}"),
            Action::TalkTo(slot),
        );
        bind(
            app,
            bindings,
            listing,
            &format!("Message PC {slot}"),
            &format!("CommandOrControl+Shift+Digit{slot}"),
            Action::MessageTo(slot),
        );
    }
}
