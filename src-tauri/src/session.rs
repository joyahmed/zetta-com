//! Owns `audio` and `net` and wires them to each other.
//!
//! This layer exists so neither of them has to import the other. Without it
//! `audio` grows networking and `net` grows codecs, and by the time push-to-talk
//! and per-peer decoders arrive neither module can be tested on its own.
//!
//! ```text
//! capture → ring A → encoder ──► frames_out ──► pump ──► net tx
//! playback ← ring B ← decoder ◄── frames_in ◄────────── net rx
//! ```

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::RecvTimeoutError;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::Result;

use crate::audio;
use crate::net;

pub struct Session {
    /// Held only to keep the audio pipeline alive; it stops when dropped.
    _audio: audio::Handle,
    net: Arc<net::Handle>,
    stop: Arc<AtomicBool>,
}

impl Drop for Session {
    fn drop(&mut self) {
        // The pump thread holds the other reference to the net handle, so the
        // socket closes once it sees this flag and lets go — a moment after
        // this returns rather than during it.
        self.stop.store(true, Ordering::Relaxed);
    }
}

impl Session {
    pub fn stats(&self) -> net::Stats {
        self.net.stats()
    }
}

pub fn start(port: u16, peer: &str) -> Result<Session> {
    let pipeline = audio::start()?;
    let net = Arc::new(net::start(port, peer, pipeline.frames_in.clone())?);

    let stop = Arc::new(AtomicBool::new(false));
    let pump_stop = stop.clone();
    let pump_net = net.clone();
    let frames_out = pipeline.frames_out;

    // The microphone sets the pace: one send per encoded frame, no timer to
    // drift against the capture rate.
    thread::Builder::new()
        .name("session-pump".into())
        .spawn(move || {
            while !pump_stop.load(Ordering::Relaxed) {
                match frames_out.recv_timeout(Duration::from_millis(200)) {
                    Ok(frame) => pump_net.send_audio(&frame.data, frame.samples),
                    // Timeouts are how this loop notices the stop flag.
                    Err(RecvTimeoutError::Timeout) => continue,
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            }
        })?;

    Ok(Session {
        _audio: pipeline.handle,
        net,
        stop,
    })
}
