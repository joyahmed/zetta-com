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

use std::sync::atomic::Ordering;

use tauri::{AppHandle, Emitter, Manager};

static APP: OnceLock<AppHandle> = OnceLock::new();

pub fn init(app: AppHandle) {
    let _ = APP.set(app);
}

/// Somebody's voice just started arriving, after at least a moment of silence.
///
/// Not while this machine is talking on speakers: the window plays the blip,
/// and with the push-to-talk key down the microphone is open to the room — so
/// the blip went out to whoever you were talking to, over the top of you.
/// On headphones it cannot reach the microphone and still plays.
pub fn voice_start(from: std::net::SocketAddr) {
    if let Some(app) = APP.get() {
        let held = app
            .try_state::<crate::state::Ptt>()
            .is_some_and(|p| p.0.load(Ordering::Relaxed));
        let headphones = crate::audio::HEADPHONES.load(Ordering::Relaxed);
        if !blip_allowed(held, headphones) {
            return;
        }
        let _ = app.emit("voice-start", from.to_string());
    }
}

/// Whether a sound may play now without being sent to the room. Apart from
/// `voice_start` so the rule can be tested without an app.
fn blip_allowed(talking: bool, headphones: bool) -> bool {
    !talking || headphones
}

/// A transport was started, or replaced in place by a restart (a passphrase,
/// a microphone, a PC added by hand). No payload: the window uses it to reset
/// anything it worked out from the old one — who it had already seen, for the
/// presence tones — and every restart looks like the whole room appearing.
pub fn session_restart() {
    if let Some(app) = APP.get() {
        let _ = app.emit("session-restart", ());
    }
}

/// Who you are talking to changed — from the window, from a key, or because
/// the group you had picked was renamed or deleted. Carries the whole
/// `state::Target`, so the window draws the selection from this and never
/// keeps its own idea of it.
pub fn target(t: &crate::state::Target) {
    if let Some(app) = APP.get() {
        let _ = app.emit("target", t);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_blip_while_talking_on_speakers() {
        assert!(!blip_allowed(true, false));
        assert!(blip_allowed(true, true));
        assert!(blip_allowed(false, false));
    }
}
