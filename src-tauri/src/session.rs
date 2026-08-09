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

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::RecvTimeoutError;
use std::sync::{Arc, Mutex};
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
    /// Resolved once at start. Changing the list restarts the session, which is
    /// simpler than mutating it live and happens rarely enough not to matter.
    manual: Vec<discovery::Peer>,
    /// Your name for a machine, by address. Overrides everything else, being
    /// the only name anybody chose deliberately.
    ///
    /// Behind a lock so renaming can be applied to a running session. It used
    /// to be plain, which meant a rename had to rebuild the whole session to
    /// take effect — tearing down the socket and rebinding it for what is only
    /// ever a display name.
    labels: Mutex<HashMap<String, String>>,
    /// The order the roster is shown in, by address. Behind a lock for the same
    /// reason labels are: reordering is a display change and must not cost a
    /// rebind of the socket.
    order: Mutex<Vec<String>>,
    stop: Arc<AtomicBool>,
    /// Joined on drop, before the net handle's last reference goes with them.
    threads: Vec<thread::JoinHandle<()>>,
}

impl Drop for Session {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        // These hold the other references to the net handle, so the handle —
        // and the socket inside it — cannot be dropped until they are done.
        // Setting the flag and returning left the port bound for a moment
        // after the session was gone, and anything that rebound in that moment
        // failed with "Only one usage of each socket address is normally
        // permitted".
        for t in self.threads.drain(..) {
            let _ = t.join();
        }
    }
}

impl Session {
    pub fn stats(&self) -> net::Stats {
        self.net.stats()
    }

    pub fn send_text(&self, text: &str) {
        self.net.send_text(text);
    }

    /// Aim voice and text at one machine, or at everyone when `None`.
    pub fn set_target(&self, addr: Option<SocketAddr>) {
        self.net.set_target(addr);
    }

    /// The log, with addresses replaced by names. `net` stores who sent what as
    /// an address because it has no idea who anybody is; putting a name on it
    /// is the session's job, since it is the only layer that holds both the
    /// socket and the roster.
    pub fn messages(&self) -> Vec<net::Message> {
        // The merged roster, not just what discovery found. A hand-added PC is
        // absent from discovery entirely, so looking there left its messages
        // labelled with an address — and an address is not who said something.
        let names: Vec<(String, String)> = self
            .peers()
            .into_iter()
            .map(|p| (p.addr.to_string(), p.name))
            .collect();

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

    /// Your names for machines, applied to a session that is already running.
    ///
    /// A rename is a display change and nothing more — it never reaches the
    /// socket — so it must not cost a rebind. Doing it that way was what made
    /// renaming fail with "address already in use".
    pub fn set_labels(&self, labels: HashMap<String, String>) {
        let mut guard = match self.labels.lock() {
            Ok(l) => l,
            Err(e) => e.into_inner(),
        };
        *guard = labels;
    }

    /// The order to show the roster in, applied to a running session.
    pub fn set_order(&self, order: Vec<String>) {
        let mut guard = match self.order.lock() {
            Ok(o) => o,
            Err(e) => e.into_inner(),
        };
        *guard = order;
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
        let labels = match self.labels.lock() {
            Ok(l) => l,
            Err(e) => e.into_inner(),
        };
        let mut peers = self
            .discovery
            .as_ref()
            .map(|d| d.peers())
            .unwrap_or_default();

        // Manual entries fill the gaps discovery leaves, and merge rather than
        // duplicate: an address you typed that mDNS also found is one person,
        // and showing them twice would make the roster lie about how many
        // people are there.
        for m in &self.manual {
            match peers.iter_mut().find(|p| p.addr == m.addr) {
                Some(existing) => existing.manual = true,
                None => peers.push(m.clone()),
            }
        }

        for p in &mut peers {
            p.live = self.net.heard_within(p.addr, net::HEARD_TIMEOUT);
            p.talking = self.net.talking(p.addr);
            // A name learned from a heartbeat beats the address a manual entry
            // starts out labelled with. Discovered peers already have a name
            // and keep it, since that one came with the advertisement.
            if p.manual {
                if let Some(name) = self.net.name_of(p.addr) {
                    p.name = name;
                }
            }
            // Your own name wins over both. A PC name is what the machine calls
            // itself; this is what you call the person sitting at it, and it is
            // the only one anybody chose on purpose.
            if let Some(label) = labels.get(&p.addr.to_string()) {
                if !label.trim().is_empty() {
                    p.name = label.clone();
                }
            }
        }
        // Chosen order first, then the rest by name. The slot numbers the
        // shortcuts use count down this list, so this is what decides which PC
        // is Ctrl+1 — and it must not shuffle every time somebody is renamed or
        // a new machine appears.
        let order = match self.order.lock() {
            Ok(o) => o,
            Err(e) => e.into_inner(),
        };
        peers.sort_by(|a, b| {
            let rank = |p: &discovery::Peer| {
                order
                    .iter()
                    .position(|x| *x == p.addr.to_string())
                    .unwrap_or(usize::MAX)
            };
            rank(a).cmp(&rank(b)).then_with(|| a.name.cmp(&b.name))
        });
        peers
    }
}

/// Turn typed addresses into peers.
///
/// Resolved to IPv4 literals here rather than kept as text, because a Windows
/// PC name resolves IPv6-link-local first and anything taking the first address
/// reaches nobody — the bug behind both of v1's "the text arrived and the voice
/// did not" failures. One that will not resolve is dropped with a line saying
/// so, rather than sitting in the roster looking reachable.
fn manual_peers(manual: &[String]) -> Vec<discovery::Peer> {
    manual
        .iter()
        .filter_map(|entry| match net::resolve_v4(entry) {
            Ok(addr) => Some(discovery::Peer {
                id: format!("manual:{entry}"),
                name: entry.clone(),
                addr,
                live: false,
                talking: false,
                manual: true,
            }),
            Err(e) => {
                eprintln!("[net] manual peer {entry}: {e:#}");
                None
            }
        })
        .collect()
}

pub fn start(
    port: u16,
    peer: &str,
    manual_entries: &[String],
    labels: HashMap<String, String>,
    order: Vec<String>,
    audio_prefs: audio::Prefs,
    transmit: Arc<AtomicBool>,
) -> Result<Session> {
    // The typed Address is just another manual peer. It used to be handled
    // separately: seeded into the send list at bind and then quietly dropped a
    // second later, when the roster sync replaced the list with discovery plus
    // the manual entries and forgot it existed. Everything went to the right
    // place for exactly one second.
    let mut entries: Vec<String> = manual_entries.to_vec();
    let typed = peer.trim();
    if !typed.is_empty() && !entries.iter().any(|e| e == typed) {
        entries.push(typed.to_string());
    }
    let manual = manual_peers(&entries);

    let pipeline = audio::start(transmit, audio_prefs)?;
    let net = Arc::new(net::start(
        port,
        peer,
        &discovery::local_name(),
        pipeline.frames_in.clone(),
    )?);

    let stop = Arc::new(AtomicBool::new(false));
    let pump_stop = stop.clone();
    let pump_net = net.clone();
    let frames_out = pipeline.frames_out;

    // The microphone sets the pace: one send per encoded frame, no timer to
    // drift against the capture rate.
    let pump_thread = thread::Builder::new()
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
    // manually — which is why manual entries exist at all.
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
    let sync_manual = manual.clone();
    let sync_thread = thread::Builder::new()
        .name("roster-sync".into())
        .spawn(move || {
            while !sync_stop.load(Ordering::Relaxed) {
                let mut known: Vec<_> = sync_discovery
                    .as_ref()
                    .map(|d| d.peers().into_iter().map(|p| p.addr).collect())
                    .unwrap_or_else(Vec::new);
                for m in &sync_manual {
                    if !known.contains(&m.addr) {
                        known.push(m.addr);
                    }
                }
                // A typed address is an instruction, not a guess: send there
                // whether or not it has ever answered. Discovered peers still
                // have to prove they are present, since an advertisement is
                // only evidence that a machine was once switched on.
                let live: Vec<_> = known
                    .iter()
                    .copied()
                    .filter(|a| {
                        sync_manual.iter().any(|m| m.addr == *a)
                            || sync_net.heard_within(*a, net::HEARD_TIMEOUT)
                    })
                    .collect();
                sync_net.set_targets(known, live);
                // A second, but checked ten times, so stopping waits a tenth
                // of a second rather than a whole one. This thread is joined on
                // drop now, and a sleeping thread is a rebind that has to wait
                // for it.
                for _ in 0..10 {
                    if sync_stop.load(Ordering::Relaxed) {
                        break;
                    }
                    thread::sleep(Duration::from_millis(100));
                }
            }
        })?;

    Ok(Session {
        _audio: pipeline.handle,
        net,
        discovery,
        manual,
        labels: Mutex::new(labels),
        order: Mutex::new(order),
        stop,
        threads: vec![pump_thread, sync_thread],
    })
}
