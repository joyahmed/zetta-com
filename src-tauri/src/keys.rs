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
    /// Go on or off air without touching the window. The frontend owns start
    /// and stop — it holds the port and reports the errors — so this asks
    /// rather than does, the same way ShowShortcuts does.
    ToggleTransport,
    // There is deliberately no key for adding a PC. It was Ctrl+Alt+A, and it
    // is a thing you do once per machine, from a button that is already on
    // screen — a global chord reserved system-wide for that is a cost with no
    // return, and every key removed makes the ones that remain easier to hold
    // in your head.
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
    /// Set for the keys a user may change, and the name their choice is saved
    /// under. `None` for the per-PC ranges: those are nine keys generated from
    /// one pattern, and letting each be rebound separately would turn a rule
    /// you can hold in your head into eighteen facts you cannot.
    pub id: Option<String>,
}

/// The keys that can be rebound, in the order they are shown, with what they do
/// and what they are unless somebody says otherwise.
pub const EDITABLE: [(&str, &str, &str); 6] = [
    ("talk", "Talk to whoever is selected", "F8"),
    ("talk-all", "Talk to everyone", "CommandOrControl+Digit0"),
    (
        "message-all",
        "Message everyone",
        "CommandOrControl+Shift+Digit0",
    ),
    ("start-stop", "Start or stop", "F9"),
    // Ctrl+Alt+T rather than F10. F10 registers, but Windows treats it as the
    // menu-bar key and other applications swallow it, so a global key on it is
    // one that sometimes silently does nothing — the exact failure this list
    // exists to expose. This is also the key people already reach for to wake
    // the app.
    ("open-window", "Open the window", "CommandOrControl+Alt+KeyT"),
    ("show-shortcuts", "Show shortcuts", "F1"),
];

/// What a rebindable key is set to: the saved choice, or its default.
pub fn spec_for(cfg: &crate::config::Config, id: &str) -> String {
    if let Some(s) = cfg.shortcuts.get(id) {
        if !s.trim().is_empty() {
            return s.clone();
        }
    }
    // `talk_shortcut` predates the map and is still where an older config keeps
    // it, so it is honoured rather than silently replaced by the default.
    if id == "talk" && !cfg.talk_shortcut.trim().is_empty() {
        return cfg.talk_shortcut.clone();
    }
    EDITABLE
        .iter()
        .find(|(k, _, _)| *k == id)
        .map(|(_, _, d)| d.to_string())
        .unwrap_or_default()
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

/// The binding table, so a key can be changed without restarting.
pub struct BindingsState(pub Bindings);

/// Throw every registration away and build them again from the config.
///
/// Wholesale rather than unregistering one key and adding another: the listing
/// the UI shows is rebuilt from the same pass, so doing it piecemeal would mean
/// keeping two structures in step and the list quietly drifting from what is
/// actually registered. There are about thirty keys and this happens when
/// somebody edits one.
pub fn rebind(
    app: &tauri::AppHandle,
    bindings: &Bindings,
    listing: &ShortcutList,
    cfg: &crate::config::Config,
) {
    if let Err(e) = app.global_shortcut().unregister_all() {
        eprintln!("[keys] could not release the old keys: {e}");
    }
    match bindings.lock() {
        Ok(mut b) => b.clear(),
        Err(e) => e.into_inner().clear(),
    }
    match listing.lock() {
        Ok(mut l) => l.clear(),
        Err(e) => e.into_inner().clear(),
    }
    register_all(app, bindings, listing, cfg);
}

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
    id: Option<&str>,
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
            id: id.map(str::to_string),
            label: label.to_string(),
            // Tauri's spec names a letter "KeyA" and a number "Digit1"; both
            // are noise on a key cap. Stripping only "Digit" left the letter
            // keys reading "Ctrl+Alt+KeyK" in the list, which looks like a
            // typo rather than a shortcut.
            keys: spec
                .replace("CommandOrControl", "Ctrl")
                .replace("Digit", "")
                .replace("Key", ""),
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
        Action::Talk => {
            // Said out loud because this is the first link in the chain: if the
            // key never reaches here, no amount of looking at the socket will
            // explain the silence.
            eprintln!("[keys] talk key {}", if pressed { "down" } else { "up" });
            ptt.store(pressed, Ordering::Relaxed)
        }

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
        // No reveal(). Going off air mid-conversation should not drag the
        // window in front of whatever is being worked on — that is the reason
        // to have the key at all.
        Action::ToggleTransport => {
            if pressed {
                let _ = app.emit("toggle-transport", ());
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
    // The rebindable ones, in EDITABLE's order, each taking the saved choice or
    // falling back to its default.
    //
    // Talking is Ctrl+number by default and messaging Ctrl+Shift+number: one
    // modifier for the thing done constantly, the same number either way. Those
    // are *global*, so the app holds Ctrl+0…9 for every other application on
    // the machine while it runs. The app's own actions are single function keys
    // instead — they are pressed occasionally, which is when a chord is hardest,
    // and F1 for the key list because every other program has taught that.
    for (id, label, _) in EDITABLE {
        let action = match id {
            "talk" => Action::Talk,
            "talk-all" => Action::TalkAll,
            "message-all" => Action::MessageAll,
            "start-stop" => Action::ToggleTransport,
            "open-window" => Action::ShowWindow,
            "show-shortcuts" => Action::ShowShortcuts,
            _ => continue,
        };
        bind(
            app,
            bindings,
            listing,
            label,
            &spec_for(cfg, id),
            action,
            Some(id),
        );
    }

    for p in &cfg.presets {
        bind(
            app,
            bindings,
            listing,
            &format!("Send \u{201c}{}\u{201d}", p.label),
            &p.shortcut,
            Action::Preset(p.text.clone()),
            None,
        );
    }

    // Nine of each, which is as many as a row of number keys gives and more
    // people than one person directs at once.
    for slot in 1..=9usize {
        bind(
            app,
            bindings,
            listing,
            &format!("Talk to PC {slot}"),
            &format!("CommandOrControl+Digit{slot}"),
            Action::TalkTo(slot),
            None,
        );
        bind(
            app,
            bindings,
            listing,
            &format!("Message PC {slot}"),
            &format!("CommandOrControl+Shift+Digit{slot}"),
            Action::MessageTo(slot),
            None,
        );
    }
}
