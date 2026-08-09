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
use crate::discovery;
use crate::net;

pub struct Session {
    /// Held only to keep the audio pipeline alive; it stops when dropped.
    _audio: audio::Handle,
    net: Arc<net::Handle>,
    /// Advertising and browsing. Dropping it withdraws us from the network, so
    /// stopping the transport also makes us disappear from other rosters
    /// immediately rather than after a timeout.
    discovery: Option<Arc<discovery::Discovery>>,
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

    pub fn send_text(&self, text: &str) {
        self.net.send_text(text);
    }

    /// The log, with addresses replaced by names. `net` stores who sent what as
    /// an address because it has no idea who anybody is; putting a name on it
    /// is the session's job, since it is the only layer that holds both the
    /// socket and the roster.
    pub fn messages(&self) -> Vec<net::Message> {
        let names: Vec<(String, String)> = self
            .discovery
            .as_ref()
            .map(|d| {
                d.peers()
                    .into_iter()
                    .map(|p| (p.addr.to_string(), p.name))
                    .collect()
            })
            .unwrap_or_default();

        let mut msgs = self.net.messages();
        for m in &mut msgs {
            if m.mine {
                continue;
            }
            if let Some((_, name)) = names.iter().find(|(addr, _)| *addr == m.from) {
                m.from = name.clone();
            }
        }
        msgs
    }

    /// Empty when discovery could not start — which is a normal state on a
    /// network that filters mDNS, not a failure of the session.
    ///
    /// mDNS decides *who exists*; the socket decides *who is present*. Those
    /// are different questions and only one of them can be answered honestly by
    /// an announcement: a machine that has been switched off keeps looking
    /// discovered for a while, and a roster claiming somebody can hear you when
    /// they cannot is worse than no roster at all.
    pub fn peers(&self) -> Vec<discovery::Peer> {
        let mut peers = self
            .discovery
            .as_ref()
            .map(|d| d.peers())
            .unwrap_or_default();
        for p in &mut peers {
            p.live = self.net.heard_within(p.addr, net::HEARD_TIMEOUT);
            p.talking = self.net.talking(p.addr);
        }
        peers
    }
}

pub fn start(port: u16, peer: &str, transmit: Arc<AtomicBool>) -> Result<Session> {
    let pipeline = audio::start(transmit)?;
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

    // Discovery failing must not take the transport down with it. Plenty of
    // networks filter mDNS, and on those the app still works with a peer typed
    // by hand — which is why manual entries exist at all.
    let discovery = match discovery::start(port) {
        Ok(d) => Some(Arc::new(d)),
        Err(e) => {
            eprintln!("[mdns] discovery unavailable: {e:#}");
            None
        }
    };

    // Keep the send list in step with the roster. A second is plenty: peers
    // appear over mDNS in a couple of seconds and go quiet over seven, so this
    // is never the slow part, and doing it per frame would mean scanning the
    // roster fifty times a second to learn nothing new.
    let sync_stop = stop.clone();
    let sync_net = net.clone();
    let sync_discovery = discovery.clone();
    thread::Builder::new()
        .name("roster-sync".into())
        .spawn(move || {
            while !sync_stop.load(Ordering::Relaxed) {
                if let Some(d) = sync_discovery.as_ref() {
                    let peers = d.peers();
                    let known: Vec<_> = peers.iter().map(|p| p.addr).collect();
                    let live: Vec<_> = peers
                        .iter()
                        .filter(|p| sync_net.heard_within(p.addr, net::HEARD_TIMEOUT))
                        .map(|p| p.addr)
                        .collect();
                    sync_net.set_targets(known, live);
                }
                thread::sleep(Duration::from_secs(1));
            }
        })?;

    Ok(Session {
        _audio: pipeline.handle,
        net,
        discovery,
        stop,
    })
}
