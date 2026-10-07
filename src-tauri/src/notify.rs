//! Telling the window that something just happened.
//!
//! The window learns most things by polling, four times a second, and that is
//! right for a roster. It is wrong for a sound: a beep for "somebody started
//! talking" that lands 400 ms late lands in the middle of their first word. So
//! the few moments a sound hangs on are pushed as events, from wherever they
//! happen, the instant they happen.
//!
//! A process-wide handle rather than one threaded through `net::start`: the
//! receive thread is created in six places, and none of them otherwise needs to
//! know a window exists. Before `init` runs, and in tests, emitting does
//! nothing.

use std::sync::OnceLock;

use tauri::{AppHandle, Emitter};

static APP: OnceLock<AppHandle> = OnceLock::new();

pub fn init(app: AppHandle) {
    let _ = APP.set(app);
}

/// Somebody's voice just started arriving, after at least a moment of silence.
pub fn voice_start(from: std::net::SocketAddr) {
    if let Some(app) = APP.get() {
        let _ = app.emit("voice-start", from.to_string());
    }
}

/// Do-not-disturb was switched, from the tray or the window. Sent either way,
/// so the window never has to work out which of the two it was.
pub fn dnd(on: bool) {
    if let Some(app) = APP.get() {
        let _ = app.emit("dnd", on);
    }
}

/// This machine's own talk key went down (`true`) or up (`false`).
pub fn talk_key(down: bool) {
    if let Some(app) = APP.get() {
        let _ = app.emit("talk-key", down);
    }
}
