//! Finding other instances on the LAN, and being findable.
//!
//! This is what replaces typing an address, and it is also what lets the app
//! report that somebody is *not* there — the thing v1 could never do. A roster
//! that greys out four seconds after a PC is switched off removes most of the
//! "is it even working" debugging that cost days in the old version.
//!
//! Slice 3a: advertise and browse, and say what appears and vanishes. Nothing
//! depends on it yet.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use serde::Serialize;

/// Our service type. The `_udp` matters: it must match the transport actually
/// used, or tools that inspect the network describe us wrongly.
const SERVICE: &str = "_zettacom._udp.local.";

/// How long a peer may go unheard before it is considered gone. Long enough to
/// survive a missed announcement, short enough that switching a PC off is
/// visible while you are still looking at the screen.
const PEER_TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Peer {
    /// Stable identity: the mDNS instance name, which is the machine's name.
    pub id: String,
    /// What to show a human.
    pub name: String,
    pub addr: SocketAddr,
    /// False once nothing has been heard from it for PEER_TIMEOUT. Kept in the
    /// roster rather than removed, because "was here, now gone" is more useful
    /// than a name silently disappearing.
    pub live: bool,
}

struct Entry {
    peer: Peer,
    seen: Instant,
}

pub struct Discovery {
    peers: Arc<Mutex<HashMap<String, Entry>>>,
    stop: Arc<AtomicBool>,
    /// Held so the daemon lives as long as we do; dropping it withdraws our
    /// advertisement and stops the browse.
    daemon: ServiceDaemon,
    fullname: String,
}

impl Drop for Discovery {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        // Withdraw the advertisement explicitly rather than letting it time
        // out, so other machines grey us out immediately instead of waiting.
        let _ = self.daemon.unregister(&self.fullname);
        let _ = self.daemon.shutdown();
    }
}

impl Discovery {
    /// The roster, newest state first-hand: liveness is computed at read time
    /// from when each peer was last heard, so it cannot go stale between polls.
    pub fn peers(&self) -> Vec<Peer> {
        let now = Instant::now();
        let map = match self.peers.lock() {
            Ok(m) => m,
            Err(e) => e.into_inner(),
        };
        let mut out: Vec<Peer> = map
            .values()
            .map(|e| Peer {
                live: now.duration_since(e.seen) < PEER_TIMEOUT,
                ..e.peer.clone()
            })
            .collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }
}

/// This machine's name, for the roster. `COMPUTERNAME` on Windows is what
/// people recognise, and it is what v1's roster used.
pub fn local_name() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}

pub fn start(port: u16) -> Result<Discovery> {
    let daemon = ServiceDaemon::new().context("starting the mDNS daemon")?;
    let name = local_name();

    // Advertise. The instance name has to be unique on the network; the
    // machine name is exactly that, and is also what a human recognises.
    let info = ServiceInfo::new(SERVICE, &name, &format!("{name}.local."), (), port, None)
        .context("building the service advertisement")?
        // Ask the daemon to fill in this host's addresses rather than guessing
        // at them: a machine with a VPN adapter or a WSL bridge has several,
        // and picking the wrong one advertises an address nobody can reach.
        .enable_addr_auto();

    let fullname = info.get_fullname().to_string();
    daemon
        .register(info)
        .context("registering the service advertisement")?;

    let browse = daemon
        .browse(SERVICE)
        .context("starting the mDNS browse")?;

    let peers: Arc<Mutex<HashMap<String, Entry>>> = Arc::new(Mutex::new(HashMap::new()));
    let stop = Arc::new(AtomicBool::new(false));

    let ev_peers = peers.clone();
    let ev_stop = stop.clone();
    let me = fullname.clone();
    thread::Builder::new()
        .name("mdns".into())
        .spawn(move || {
            while !ev_stop.load(Ordering::Relaxed) {
                // Timed rather than blocking, so the thread notices the stop
                // flag instead of parking until the next announcement.
                let event = match browse.recv_timeout(Duration::from_millis(500)) {
                    Ok(e) => e,
                    Err(_) => continue,
                };

                match event {
                    ServiceEvent::ServiceResolved(info) => {
                        // We advertise on the same network we browse, so we
                        // see ourselves. Talking to yourself is a real bug in
                        // an intercom, and this is where it is cheapest to
                        // prevent.
                        if info.get_fullname() == me {
                            continue;
                        }
                        let Some(ip) = info
                            .get_addresses()
                            .iter()
                            .find(|a| a.is_ipv4())
                            .map(|a| a.to_ip_addr())
                        else {
                            // IPv4 only, deliberately. The socket binds
                            // 0.0.0.0, and a peer advertised only over IPv6
                            // would be unreachable — the same trap that made
                            // every v1 listener play perfect silence.
                            eprintln!(
                                "[mdns] {} has no IPv4 address, ignoring",
                                info.get_fullname()
                            );
                            continue;
                        };

                        let id = info.get_fullname().to_string();
                        let name = info
                            .get_fullname()
                            .split('.')
                            .next()
                            .unwrap_or(&id)
                            .to_string();
                        let addr = SocketAddr::new(ip, info.get_port());

                        let mut map = match ev_peers.lock() {
                            Ok(m) => m,
                            Err(e) => e.into_inner(),
                        };
                        let fresh = map.insert(
                            id.clone(),
                            Entry {
                                peer: Peer {
                                    id: id.clone(),
                                    name: name.clone(),
                                    addr,
                                    live: true,
                                },
                                seen: Instant::now(),
                            },
                        );
                        if fresh.is_none() {
                            eprintln!("[mdns] + {name} at {addr}");
                        }
                    }
                    ServiceEvent::ServiceRemoved(_, fullname) => {
                        let mut map = match ev_peers.lock() {
                            Ok(m) => m,
                            Err(e) => e.into_inner(),
                        };
                        if let Some(e) = map.remove(&fullname) {
                            eprintln!("[mdns] - {} gone", e.peer.name);
                        }
                    }
                    _ => {}
                }
            }
        })?;

    eprintln!("[mdns] advertising {name} on {SERVICE} port {port}");

    Ok(Discovery {
        peers,
        stop,
        daemon,
        fullname,
    })
}
