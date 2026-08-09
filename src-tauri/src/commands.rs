//! Every command the window can call.
//!
//! Commands return `Result<_, String>` rather than `anyhow::Result` because
//! anyhow's error type is not serialisable across the IPC boundary. The
//! conversion belongs here, at the edge, not inside the modules doing the work.

use std::sync::atomic::Ordering;

use tauri::State;

use crate::state::{NetState, Ptt};
use crate::{config, discovery, net, session};

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// The running session, or nothing. `session::Session` stops audio and the
#[tauri::command]
pub fn net_start(
    app: tauri::AppHandle,
    state: State<NetState>,
    ptt: State<Ptt>,
    port: u16,
    peer: String,
) -> Result<(), String> {
    // A port on the command line wins, so a second instance started for testing
    // cannot be dragged back onto the first one's port by the shared config.
    let port = config::port_override().unwrap_or(port);
    let saved = config::load(&app).unwrap_or_default();
    let (manual, labels) = (saved.manual, saved.labels);
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    // Drop any existing transport before binding, or starting on the same port
    // fails with "address already in use" against ourselves.
    *guard = None;
    *guard = Some(
        session::start(port, &peer, &manual, labels, ptt.0.clone()).map_err(|e| format!("{e:#}"))?,
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
pub fn manual_peers(
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
            session::start(cfg.port, &cfg.peer, &cfg.manual, cfg.labels.clone(), ptt.0.clone())
                .map_err(|e| format!("{e:#}"))?,
        );
    }
    Ok(cfg.manual)
}

/// Everyone discovered on the LAN, live or recently gone. Empty when the
/// transport is stopped, and also on a network that filters mDNS — which is a
/// normal condition, not an error, and the reason peers can be added by hand.
#[tauri::command]
pub fn net_peers(state: State<NetState>) -> Result<Vec<discovery::Peer>, String> {
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
pub fn send_text(state: State<NetState>, text: String) -> Result<(), String> {
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

/// Give a machine your own name for it, or clear it with an empty string.
///
/// Restarts the session so it takes effect, same as adding a peer: renaming is
/// rare, and one code path that rebuilds beats two that have to agree.
#[tauri::command]
pub fn set_label(
    app: tauri::AppHandle,
    state: State<NetState>,
    ptt: State<Ptt>,
    addr: String,
    label: String,
) -> Result<(), String> {
    let mut cfg = config::load(&app).unwrap_or_default();
    let label = label.trim().to_string();
    if label.is_empty() {
        cfg.labels.remove(&addr);
    } else {
        cfg.labels.insert(addr, label);
    }
    config::save(&app, &cfg).map_err(|e| format!("{e:#}"))?;

    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    if guard.is_some() {
        *guard = None;
        *guard = Some(
            session::start(
                config::port_override().unwrap_or(cfg.port),
                &cfg.peer,
                &cfg.manual,
                cfg.labels.clone(),
                ptt.0.clone(),
            )
            .map_err(|e| format!("{e:#}"))?,
        );
    }
    Ok(())
}

/// Aim voice and text at one machine, or at everyone when `addr` is null.
///
/// This is the whole of per-person targeting: every send is already a unicast
/// to each recipient in turn, so addressing one person is a shorter list rather
/// than a different protocol.
#[tauri::command]
pub fn set_target(state: State<NetState>, addr: Option<String>) -> Result<(), String> {
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
pub fn messages(state: State<NetState>) -> Result<Vec<net::Message>, String> {
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
pub fn ptt_held(ptt: State<Ptt>) -> bool {
    ptt.0.load(Ordering::Relaxed)
}

/// The name this machine advertises itself under, so the UI can show you which
/// entry in everyone else's roster is you.
#[tauri::command]
pub fn local_name() -> String {
    discovery::local_name()
}

/// What the app will auto-start with, or `None` if it has never been
/// configured. The UI reads this to fill its fields rather than keeping its own
/// copy, so there is one source of truth for what this machine does.
#[tauri::command]
pub fn config_get(app: tauri::AppHandle) -> Option<config::Config> {
    config::load(&app)
}

#[tauri::command]
pub fn net_stop(state: State<NetState>) -> Result<(), String> {
    *state.0.lock().map_err(|e| e.to_string())? = None;
    Ok(())
}

/// `None` when the transport is stopped. The UI polls this on a timer rather
/// than receiving an event per packet: at 50 packets a second per peer, one IPC
/// message each would melt the webview, and polling decouples the redraw rate
/// from the packet rate for free.
#[tauri::command]
pub fn net_stats(state: State<NetState>) -> Result<Option<net::Stats>, String> {
    Ok(state
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .as_ref()
        .map(|h| h.stats()))
}

