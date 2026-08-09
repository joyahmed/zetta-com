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
    /// Set by the session from the socket, not by discovery: audio arriving is
    /// what "talking" means, and mDNS has nothing to say about it.
    pub talking: bool,
    /// Typed in manually rather than discovered. Worth showing: a manual entry
    /// that never goes live is a wrong address, where a discovered one that
    /// goes quiet is a switched-off PC, and those want different reactions.
    pub manual: bool,
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

/// This machine's name, for the roster. `COMPUTERNAME` is tried first so
/// Windows keeps showing exactly what v1's roster showed; `gethostname` comes
/// after it because macOS and Linux hand a GUI-launched app an environment with
/// neither variable set, and this fell all the way through to the literal
/// string "unknown" on both — which is then what every other machine displayed.
pub fn local_name() -> String {
    if let Ok(n) = std::env::var("COMPUTERNAME") {
        if !n.is_empty() {
            return n;
        }
    }
    let host = gethostname::gethostname().to_string_lossy().into_owned();
    // macOS answers "Joys-MacBook-Pro.local". Left alone that becomes the mDNS
    // hostname "Joys-MacBook-Pro.local.local." below, which resolves for nobody.
    let host = host.strip_suffix(".local").unwrap_or(&host).trim().to_string();
    if host.is_empty() {
        return "unknown".to_string();
    }
    host
}

/// A value unique to this process, used to recognise our own advertisement.
///
/// Not the machine name: two instances on one PC share that, and comparing
/// names made each of them treat the other as itself and ignore it — which is
/// invisible across two machines and makes discovery useless on one.
fn instance_id() -> String {
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    format!("{pid:x}{nanos:x}")
}

pub fn start(port: u16) -> Result<Discovery> {
    let daemon = ServiceDaemon::new().context("starting the mDNS daemon")?;
    let name = local_name();
    let id = instance_id();

    // The mDNS instance label has to be unique on the network, so it carries
    // the id. The *display* name travels in a TXT record instead, which keeps
    // the roster showing "JOY-PC" rather than "JOY-PC-1a2b3c".
    let mut props = HashMap::new();
    props.insert("id".to_string(), id.clone());
    props.insert("name".to_string(), name.clone());

    let label = format!("{name}-{id}");
    let info = ServiceInfo::new(
        SERVICE,
        &label,
        &format!("{name}.local."),
        (),
        port,
        props,
    )
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
    let me = id.clone();
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
                        // We advertise on the same network we browse, so we see
                        // ourselves. Talking to yourself is a real bug in an
                        // intercom, and this is where it is cheapest to
                        // prevent. Compared by instance id rather than by name,
                        // because two instances on one PC share a name and
                        // comparing names made each ignore the other.
                        let their_id = info
                            .get_property_val_str("id")
                            .unwrap_or_default()
                            .to_string();
                        if their_id == me {
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

                        // Port 0 is not an address anything can be sent to.
                        // mdns-sd will hand over a record whose SRV half has
                        // not arrived yet, and accepting one listed the Mac in
                        // the Windows roster as 192.168.0.188:0 — a peer that
                        // looks present and is unreachable, which is worse than
                        // not listing it at all. Skipping is safe rather than
                        // final: the daemon re-resolves and sends another
                        // ServiceResolved once the record is complete.
                        let port = info.get_port();
                        if port == 0 {
                            eprintln!(
                                "[mdns] {} resolved with port 0, ignoring",
                                info.get_fullname()
                            );
                            continue;
                        }

                        let key = info.get_fullname().to_string();
                        // The friendly name rides in TXT; the label is a
                        // fallback for anything advertising without it.
                        let mut name = info
                            .get_property_val_str("name")
                            .unwrap_or_default()
                            .to_string();
                        if name.is_empty() {
                            name = key.split('.').next().unwrap_or(&key).to_string();
                        }
                        let addr = SocketAddr::new(ip, port);

                        let mut map = match ev_peers.lock() {
                            Ok(m) => m,
                            Err(e) => e.into_inner(),
                        };
                        let fresh = map.insert(
                            key.clone(),
                            Entry {
                                peer: Peer {
                                    id: key.clone(),
                                    name: name.clone(),
                                    addr,
                                    live: true,
                                    talking: false,
                                    manual: false,
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
