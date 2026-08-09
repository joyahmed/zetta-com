//! UDP transport.
//!
//! Started and stopped from the UI rather than from the environment, because
//! discovery (step 3) and push-to-talk (step 4) both need to change peers while
//! the app is running, and startup-only configuration would be written twice.
//!
//! Still no audio on the wire: the payload is a placeholder. What this proves
//! is the socket, the ports, the firewall, and that a structured header
//! survives the trip with its sequence intact.

use std::collections::{HashMap, VecDeque};
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::sync::mpsc::SyncSender;
use std::sync::Mutex;
use std::time::Instant;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use serde::Serialize;

/// Bumped when the wire format changes. One byte that stops a future build's
/// packets from being decoded as noise by this one.
pub const VER: u8 = 1;
pub const KIND_AUDIO: u8 = 0;
pub const KIND_TEXT: u8 = 1;
pub const KIND_HEARTBEAT: u8 = 2;

/// How many messages to keep. A log nobody can scroll forever is a log that
/// cannot grow without bound while the app sits in the tray for a week.
const MESSAGE_LIMIT: usize = 200;
pub const HEADER_LEN: usize = 8;

/// How often to say we are still here. Frequent enough that going quiet is
/// noticed while you are still looking at the screen, rare enough to be free.
const HEARTBEAT_EVERY: Duration = Duration::from_secs(2);
/// Silence longer than this and a peer is treated as gone. Three missed
/// heartbeats, so one dropped datagram never greys anybody out.
pub const HEARD_TIMEOUT: Duration = Duration::from_secs(7);
/// Audio arrives every 20 ms while somebody holds their key, so silence this
/// long means they let go. Short enough that the indicator tracks speech,
/// long enough that a couple of dropped packets do not make it flicker.
const TALKING_TIMEOUT: Duration = Duration::from_millis(400);

/// ```text
///  0        1        2                 4                              8
///  ┌────────┬────────┬─────────────────┬──────────────────────────────┐
///  │  ver   │  kind  │       seq       │          timestamp           │
///  └────────┴────────┴─────────────────┴──────────────────────────────┘
/// ```
/// Big-endian, which is network byte order by convention and keeps a hexdump
/// readable left to right. The choice matters less than never mixing it.
#[derive(Debug, Clone, Copy)]
pub struct Header {
    pub ver: u8,
    pub kind: u8,
    pub seq: u16,
    pub ts: u32,
}

impl Header {
    pub fn write(&self, buf: &mut [u8]) {
        buf[0] = self.ver;
        buf[1] = self.kind;
        buf[2..4].copy_from_slice(&self.seq.to_be_bytes());
        buf[4..8].copy_from_slice(&self.ts.to_be_bytes());
    }

    /// `None` for anything too short or from another version. This is the only
    /// place bytes from outside the process are interpreted, and anyone on the
    /// LAN can send a one-byte datagram, so the length check comes before any
    /// indexing rather than after it.
    pub fn parse(buf: &[u8]) -> Option<Header> {
        if buf.len() < HEADER_LEN {
            return None;
        }
        let h = Header {
            ver: buf[0],
            kind: buf[1],
            seq: u16::from_be_bytes([buf[2], buf[3]]),
            ts: u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]),
        };
        if h.ver != VER {
            return None;
        }
        Some(h)
    }
}

#[derive(Default)]
struct Counters {
    tx: AtomicU64,
    rx: AtomicU64,
    /// Datagrams rejected as too short or wrong version.
    bad: AtomicU64,
    /// Packets the sequence numbers say never arrived.
    lost: AtomicU64,
    last_seq: AtomicU64,
}

/// What the UI polls for. Serialised straight across the IPC boundary.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub port: u16,
    pub peer: String,
    pub tx: u64,
    pub rx: u64,
    pub bad: u64,
    pub lost: u64,
    pub last_seq: u16,
}

/// One line of the log. `from` is an address here; the session turns it into a
/// name, because `net` has no idea who anybody is.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: u64,
    pub from: String,
    pub text: String,
    /// Sent by us rather than received. The log shows both, so that pressing a
    /// key and nothing arriving is distinguishable from never having pressed it.
    pub mine: bool,
    /// Unix milliseconds, stamped on arrival.
    pub at: u64,
}

#[derive(Default)]
struct Targets {
    known: Vec<SocketAddr>,
    live: Vec<SocketAddr>,
}

/// Dropping this stops both threads, same contract as `audio::Handle`.
pub struct Handle {
    stop: Arc<AtomicBool>,
    counters: Arc<Counters>,
    port: u16,
    peer: String,
    socket: UdpSocket,
    /// Sequence and timestamp live here rather than in the sender thread
    /// because sends are now driven by the encoder, not by a timer.
    seq: AtomicU64,
    ts: AtomicU64,
    /// Who to send to. `known` is everyone discovered plus the manual address;
    /// `live` is the subset heard from recently.
    ///
    /// Heartbeats go to `known` and audio to `live`, and the difference is what
    /// stops a deadlock: if both went to `live`, two instances that had never
    /// heard each other would each wait for the other to speak first and
    /// neither ever would.
    targets: Arc<Mutex<Targets>>,
    /// One recipient, or everyone live when `None`.
    ///
    /// Sending to one person needs no field in the header and no change to the
    /// wire format, because every send is already a unicast to each recipient
    /// in turn — addressing one is simply a shorter list. What the receiver
    /// cannot tell from this alone is whether it was addressed personally, and
    /// that is worth adding when it matters.
    target: Arc<Mutex<Option<SocketAddr>>>,
    /// When each address was last heard from — the evidence behind presence.
    ///
    /// This is what makes "live" an observation rather than an assumption.
    /// mDNS only ever says a machine *announced itself*, which stays true for a
    /// while after it is switched off, and a roster that claims someone can
    /// hear you when they cannot is worse than no roster at all.
    heard: Arc<Mutex<HashMap<SocketAddr, Instant>>>,
    /// Machine names learned from heartbeats, by address.
    ///
    /// Discovery supplies names for peers it finds, but a hand-entered address
    /// has nobody to ask — it would sit in the roster as "192.168.0.42:9001",
    /// which is not a thing you can tell somebody to talk to. The heartbeat
    /// already goes to everyone; carrying the name in it costs a few bytes
    /// every two seconds and means every peer has a name however it was found.
    names: Arc<Mutex<HashMap<SocketAddr, String>>>,
    /// When audio — as opposed to a heartbeat — last arrived from each address.
    ///
    /// This is how the UI knows who is speaking, and it needs no flag in the
    /// header to do it: with push-to-talk, audio is only sent while somebody is
    /// holding their key, so receiving audio *is* the fact that they are
    /// talking. A field would have said the same thing less reliably, since it
    /// could disagree with whether packets were actually arriving.
    last_audio: Arc<Mutex<HashMap<SocketAddr, Instant>>>,
    messages: Arc<Mutex<VecDeque<Message>>>,
    next_message_id: Arc<AtomicU64>,
    /// Counts calls to `send_audio` so the recipient report can be throttled to
    /// roughly one line a second at fifty frames a second.
    sent_report: AtomicU64,
    /// Joined on drop. Setting the stop flag and returning was not enough: the
    /// receiver sits in `recv_from` for up to its 200 ms read timeout, and it
    /// holds a clone of the socket the whole time — so the port stayed bound
    /// after the handle was gone, and anything that rebound immediately failed
    /// with "Only one usage of each socket address is normally permitted".
    threads: Vec<thread::JoinHandle<()>>,
}

impl Drop for Handle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        // Up to one read timeout, once, on stop or restart. Worth it: the
        // alternative is a rebind that races the socket it is replacing.
        for t in self.threads.drain(..) {
            let _ = t.join();
        }
    }
}

impl Handle {
    /// Replace the send list. Called from the session as the roster changes.
    pub fn set_targets(&self, known: Vec<SocketAddr>, live: Vec<SocketAddr>) {
        let mut t = match self.targets.lock() {
            Ok(t) => t,
            Err(e) => e.into_inner(),
        };
        *t = Targets { known, live };
    }

    /// Direct everything at one machine, or at everyone when `None`.
    pub fn set_target(&self, addr: Option<SocketAddr>) {
        let mut t = match self.target.lock() {
            Ok(t) => t,
            Err(e) => e.into_inner(),
        };
        *t = addr;
    }

    /// Who a send goes to right now: the chosen one if it is still live, or
    /// everyone.
    ///
    /// The liveness check matters — picking somebody and then watching them
    /// switch their PC off should not silently send into the void, so a target
    /// that has gone away falls back rather than swallowing the message.
    fn recipients(&self) -> Vec<SocketAddr> {
        let live = {
            let t = match self.targets.lock() {
                Ok(t) => t,
                Err(e) => e.into_inner(),
            };
            t.live.clone()
        };
        let chosen = {
            let t = match self.target.lock() {
                Ok(t) => t,
                Err(e) => e.into_inner(),
            };
            *t
        };
        match chosen {
            Some(addr) if live.contains(&addr) => vec![addr],
            Some(_) => Vec::new(),
            None => live,
        }
    }

    /// Send one encoded audio frame to everyone live. `samples` is the sender's
    /// frame size, so the timestamp advances by what this machine actually
    /// captured rather than by a constant the receiver would have to guess at.
    ///
    /// One encode, N sends. Broadcast would be one send, and is the trap: some
    /// WiFi access points rate-limit or drop broadcast frames, so one person
    /// silently hears nothing while everyone else is fine. Fan-out costs a few
    /// hundred bytes per frame per peer on a LAN, and it fails visibly.
    pub fn send_audio(&self, data: &[u8], samples: u32) {
        if data.len() > MAX_PAYLOAD {
            return;
        }
        let targets = self.recipients();

        // Once a second while the key is held, say who this is going to.
        // "The microphone is working" and "somebody is receiving it" are
        // different claims, and with no line here an empty recipient list looks
        // exactly like silence: audio frames are produced, counted, and thrown
        // away without a word. A selected peer that is not live sends to
        // nobody, which is the case worth naming out loud.
        let n = self.sent_report.fetch_add(1, Ordering::Relaxed);
        if n % 50 == 0 {
            if targets.is_empty() {
                let t = match self.targets.lock() {
                    Ok(t) => t,
                    Err(e) => e.into_inner(),
                };
                eprintln!(
                    "[net] talking, but sending to nobody — {} known, {} live, target {:?}",
                    t.known.len(),
                    t.live.len(),
                    self.target.lock().ok().and_then(|g| *g)
                );
            } else {
                eprintln!("[net] talking to {} of them: {:?}", targets.len(), targets);
            }
        }

        if targets.is_empty() {
            return;
        }

        // The sequence advances once per frame, not once per recipient:
        // everyone is receiving the same stream, and a per-recipient sequence
        // would make every listener see gaps the size of the roster.
        let mut buf = [0u8; HEADER_LEN + MAX_PAYLOAD];
        let h = Header {
            ver: VER,
            kind: KIND_AUDIO,
            seq: self.seq.fetch_add(1, Ordering::Relaxed) as u16,
            ts: self.ts.fetch_add(samples as u64, Ordering::Relaxed) as u32,
        };
        h.write(&mut buf);
        buf[HEADER_LEN..HEADER_LEN + data.len()].copy_from_slice(data);
        let packet = &buf[..HEADER_LEN + data.len()];

        for addr in targets {
            match self.socket.send_to(packet, addr) {
                Ok(_) => {
                    self.counters.tx.fetch_add(1, Ordering::Relaxed);
                }
                Err(e) => eprintln!("[net] tx to {addr} failed: {e}"),
            }
        }
    }

    /// Whether this address has been heard from recently enough to count as
    /// present. Computed at read time from the last-heard stamp, so it cannot
    /// be stale between polls the way a swept flag can.
    pub fn heard_within(&self, addr: SocketAddr, within: Duration) -> bool {
        stamped_within(&self.heard, addr, within)
    }

    /// The name a machine calls itself, if it has told us.
    pub fn name_of(&self, addr: SocketAddr) -> Option<String> {
        let map = match self.names.lock() {
            Ok(m) => m,
            Err(e) => e.into_inner(),
        };
        map.get(&addr).cloned()
    }

    /// Whether audio arrived from this address recently enough to call them
    /// currently speaking.
    pub fn talking(&self, addr: SocketAddr) -> bool {
        stamped_within(&self.last_audio, addr, TALKING_TIMEOUT)
    }

    /// Send a line of text to everyone live.
    ///
    /// Its own path and its own call, deliberately. In v1 one keypress both
    /// pinged somebody and opened the mic to them, and when the ping arrived
    /// and the voice did not there was no way to tell which half had failed —
    /// they had resolved the same name differently. Same socket, same header,
    /// separate action.
    pub fn send_text(&self, text: &str) {
        let bytes = text.as_bytes();
        if bytes.is_empty() || bytes.len() > MAX_PAYLOAD {
            return;
        }
        let targets = self.recipients();

        let mut buf = [0u8; HEADER_LEN + MAX_PAYLOAD];
        Header {
            ver: VER,
            kind: KIND_TEXT,
            // Text does not ride the audio sequence: it is not a stream, and
            // advancing that counter would punch holes in the reorder window
            // at the far end.
            seq: 0,
            ts: 0,
        }
        .write(&mut buf);
        buf[HEADER_LEN..HEADER_LEN + bytes.len()].copy_from_slice(bytes);
        let packet = &buf[..HEADER_LEN + bytes.len()];

        // Logged because "nothing happened" needs to distinguish between no
        // recipients and a send that failed. Sending to nobody is the far more
        // common case and the one with no other symptom.
        if targets.is_empty() {
            eprintln!("[net] text not sent: nobody to send to");
        } else {
            eprintln!("[net] text -> {} peer(s): {:?}", targets.len(), targets);
        }

        for addr in targets {
            if let Err(e) = self.socket.send_to(packet, addr) {
                eprintln!("[net] text to {addr} failed: {e}");
            }
        }

        // Logged whether or not anybody was listening. A message that reached
        // nobody still happened, and hiding it would make an empty roster look
        // like a broken keyboard.
        self.push_message(String::new(), text.to_string(), true);
    }

    fn push_message(&self, from: String, text: String, mine: bool) {
        let msg = Message {
            id: self.next_message_id.fetch_add(1, Ordering::Relaxed),
            from,
            text,
            mine,
            at: unix_millis(),
        };
        let mut log = match self.messages.lock() {
            Ok(m) => m,
            Err(e) => e.into_inner(),
        };
        if log.len() >= MESSAGE_LIMIT {
            log.pop_front();
        }
        log.push_back(msg);
    }

    pub fn messages(&self) -> Vec<Message> {
        let log = match self.messages.lock() {
            Ok(m) => m,
            Err(e) => e.into_inner(),
        };
        log.iter().cloned().collect()
    }

    pub fn stats(&self) -> Stats {
        Stats {
            port: self.port,
            peer: self.peer.clone(),
            tx: self.counters.tx.load(Ordering::Relaxed),
            rx: self.counters.rx.load(Ordering::Relaxed),
            bad: self.counters.bad.load(Ordering::Relaxed),
            lost: self.counters.lost.load(Ordering::Relaxed),
            last_seq: self.counters.last_seq.load(Ordering::Relaxed) as u16,
        }
    }
}

/// Wall-clock, for display only. `Instant` is used everywhere timing matters,
/// because it cannot jump backwards when the clock is corrected; this is the one
/// place a human has to read the value.
fn unix_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Shared by presence and talking: both ask "was this address stamped recently".
/// Computed at read time so it cannot be stale between polls the way a swept
/// flag can.
fn stamped_within(
    map: &Arc<Mutex<HashMap<SocketAddr, Instant>>>,
    addr: SocketAddr,
    within: Duration,
) -> bool {
    let map = match map.lock() {
        Ok(m) => m,
        Err(e) => e.into_inner(),
    };
    map.get(&addr).is_some_and(|t| t.elapsed() < within)
}

/// Resolve to an IPv4 address specifically.
///
/// A Windows PC name resolves IPv6-link-local first, and anything that takes
/// the first address reaches nobody. Both of v1's "text arrived but voice did
/// not" failures were this, on the sending side.
pub fn resolve_v4(peer: &str) -> Result<SocketAddr> {
    peer.to_socket_addrs()
        .with_context(|| format!("resolving {peer}"))?
        .find(SocketAddr::is_ipv4)
        .ok_or_else(|| anyhow!("{peer} resolves to no IPv4 address"))
}

/// A 20 ms Opus frame is 80-120 bytes; this is generous slack and keeps the
/// send buffer on the stack.
const MAX_PAYLOAD: usize = 1400;

/// How many packets to hold before releasing, so one that overtakes another has
/// time to be put back in order. Three frames is 60 ms — audible as latency,
/// and the price of not clicking on every reorder.
const JITTER_TARGET: usize = 3;
/// Slots in the reorder window. Must exceed JITTER_TARGET with room to spare,
/// and a power of two so `% SLOTS` is cheap and wraps with the sequence number.
const SLOTS: usize = 32;

/// Reorder window.
///
/// UDP delivers late, out of order, and not at all, and decoding in arrival
/// order turns every one of those into a click. This holds packets by sequence
/// number and releases them in order, giving a straggler a few frames' grace
/// before declaring it lost.
///
/// Indexed by `seq % SLOTS` rather than kept in a map, because a map keyed on
/// u16 sorts wrongly across the wrap at 65535 — which arrives after about 22
/// minutes of talking, in a real conversation and never in a test.
struct Jitter {
    slots: [Option<Vec<u8>>; SLOTS],
    /// The sequence number we are waiting to release. `None` until the first
    /// packet arrives and sets the origin.
    next: Option<u16>,
    depth: usize,
}

impl Jitter {
    fn new() -> Self {
        Self {
            slots: [const { None }; SLOTS],
            next: None,
            depth: 0,
        }
    }

    fn reset(&mut self, seq: u16) {
        self.slots = [const { None }; SLOTS];
        self.depth = 0;
        self.next = Some(seq);
    }

    /// Take a packet in, and hand back everything now releasable in order.
    /// `None` in the output means a frame is genuinely missing and the decoder
    /// should conceal it rather than skip it.
    fn push(&mut self, seq: u16, payload: Vec<u8>, out: &mut Vec<Option<Vec<u8>>>) {
        let next = match self.next {
            Some(n) => n,
            None => {
                self.reset(seq);
                seq
            }
        };

        let ahead = seq.wrapping_sub(next);
        if ahead >= 0x8000 {
            // Older than what we have already released — it lost its race and
            // playing it now would put it in the wrong place.
            return;
        }
        if ahead as usize >= SLOTS {
            // Too far ahead to be reordering: the talker restarted, or the
            // network went away and came back. Starting over beats emitting
            // half a window of concealment.
            self.reset(seq);
        }

        let idx = (seq as usize) % SLOTS;
        if self.slots[idx].is_none() {
            self.depth += 1;
        }
        self.slots[idx] = Some(payload);

        // Release in order. A present frame always goes. A missing one is only
        // given up on once enough packets are banked behind it to prove it is
        // late rather than merely out of order.
        loop {
            let next = self.next.unwrap_or(seq);
            let idx = (next as usize) % SLOTS;
            if let Some(p) = self.slots[idx].take() {
                self.depth -= 1;
                out.push(Some(p));
            } else if self.depth >= JITTER_TARGET {
                out.push(None);
            } else {
                break;
            }
            self.next = Some(next.wrapping_add(1));
        }
    }
}

/// `audio_in` carries `None` for a frame the sequence numbers prove is missing,
/// so the decoder can conceal the gap rather than skip it.
pub fn start(
    port: u16,
    peer: &str,
    local_name: &str,
    audio_in: SyncSender<(SocketAddr, Option<Vec<u8>>)>,
) -> Result<Handle> {
    // An address is optional now. Discovery supplies peers on a normal network;
    // this is the escape hatch for one that filters mDNS, or for a PC on
    // another subnet that will never be discovered at all.
    let dest = if peer.trim().is_empty() {
        None
    } else {
        Some(resolve_v4(peer)?)
    };

    // 0.0.0.0 written out on purpose. An empty host binds IPv6-only on Windows
    // and then silently discards every IPv4 datagram — the bug that made every
    // v1 listener play perfect silence.
    let socket =
        UdpSocket::bind(("0.0.0.0", port)).with_context(|| format!("binding 0.0.0.0:{port}"))?;

    // Without this the receiver blocks in recv_from forever and never notices
    // the stop flag, so every restart from the UI would leak a thread.
    socket
        .set_read_timeout(Some(Duration::from_millis(200)))
        .context("setting read timeout")?;

    match dest {
        Some(d) => eprintln!("[net] bound 0.0.0.0:{port}, manual peer {peer} -> {d}"),
        None => eprintln!("[net] bound 0.0.0.0:{port}, peers from discovery"),
    }

    let stop = Arc::new(AtomicBool::new(false));
    let counters = Arc::new(Counters::default());

    // There is no sender thread any more. Sends are driven by the encoder
    // through Handle::send_audio, so the microphone sets the packet rate
    // rather than a timer that would have to be kept in step with it.
    let tx_sock = socket.try_clone().context("cloning socket for sender")?;

    let heard: Arc<Mutex<HashMap<SocketAddr, Instant>>> = Arc::new(Mutex::new(HashMap::new()));
    let last_audio: Arc<Mutex<HashMap<SocketAddr, Instant>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let names: Arc<Mutex<HashMap<SocketAddr, String>>> = Arc::new(Mutex::new(HashMap::new()));
    let messages: Arc<Mutex<VecDeque<Message>>> = Arc::new(Mutex::new(VecDeque::new()));
    let message_id = Arc::new(AtomicU64::new(1));

    // Say we are still here, whether or not anyone is talking. Without this a
    // silent peer is indistinguishable from a switched-off one, which is the
    // gap v1 never closed: it could report that somebody was there, never that
    // they were not.
    // Seeded with the manual address when there is one, so a network that
    // filters mDNS still has somebody to talk to from the first heartbeat.
    let targets = Arc::new(Mutex::new(Targets {
        known: dest.into_iter().collect(),
        live: dest.into_iter().collect(),
    }));

    let hb_sock = socket.try_clone().context("cloning socket for heartbeat")?;
    let hb_stop = stop.clone();
    let hb_targets = targets.clone();
    let local_name = local_name.to_string();
    let hb_thread = thread::Builder::new()
        .name("net-heartbeat".into())
        .spawn(move || {
            // Its own sequence space, fixed at zero: heartbeats must not
            // advance the audio sequence, or the receiver's reorder window
            // would see gaps that never existed.
            //
            // The payload is this machine's name, so a peer that was typed in
            // manually still ends up with something a human can read.
            let name = local_name.as_bytes();
            let n = name.len().min(MAX_PAYLOAD);
            let mut buf = [0u8; HEADER_LEN + MAX_PAYLOAD];
            Header {
                ver: VER,
                kind: KIND_HEARTBEAT,
                seq: 0,
                ts: 0,
            }
            .write(&mut buf);
            buf[HEADER_LEN..HEADER_LEN + n].copy_from_slice(&name[..n]);
            let buf = &buf[..HEADER_LEN + n];

            while !hb_stop.load(Ordering::Relaxed) {
                // To everyone known, not only everyone live. Two instances
                // that have never heard each other would otherwise each wait
                // for the other to speak first.
                let known = {
                    let t = match hb_targets.lock() {
                        Ok(t) => t,
                        Err(e) => e.into_inner(),
                    };
                    t.known.clone()
                };
                for addr in known {
                    if let Err(e) = hb_sock.send_to(buf, addr) {
                        eprintln!("[net] heartbeat to {addr} failed: {e}");
                    }
                }
                thread::sleep(HEARTBEAT_EVERY);
            }
        })?;

    let rx_stop = stop.clone();
    let rx_counters = counters.clone();
    let rx_heard = heard.clone();
    let rx_last_audio = last_audio.clone();
    let rx_names = names.clone();
    let rx_messages = messages.clone();
    // Shared with the Handle rather than a second counter, so sent and received
    // lines interleave in the order they actually happened.
    let rx_message_id = message_id.clone();
    let rx_thread = thread::Builder::new().name("net-rx".into()).spawn(move || {
        let mut buf = [0u8; 2048];
        // One reorder window per source. Sequence numbers are per-sender, so a
        // shared window would treat two people talking as one stream full of
        // gaps and throw most of both away.
        let mut windows: HashMap<SocketAddr, (Jitter, Option<u16>)> = HashMap::new();
        let mut ready: Vec<Option<Vec<u8>>> = Vec::with_capacity(SLOTS);

        while !rx_stop.load(Ordering::Relaxed) {
            let (len, from) = match socket.recv_from(&mut buf) {
                Ok(v) => v,
                // Timeouts are how this loop breathes; they are not errors.
                Err(_) => continue,
            };

            let Some(h) = Header::parse(&buf[..len]) else {
                rx_counters.bad.fetch_add(1, Ordering::Relaxed);
                continue;
            };

            // Anything well-formed counts as a sign of life, not just
            // heartbeats — somebody mid-sentence is obviously present, and
            // waiting for their next heartbeat to say so would be silly.
            {
                let mut map = match rx_heard.lock() {
                    Ok(m) => m,
                    Err(e) => e.into_inner(),
                };
                map.insert(from, Instant::now());
            }

            // A heartbeat carries the sender's machine name, which is how a
            // hand-entered address ends up with something readable beside it.
            if h.kind == KIND_HEARTBEAT && len > HEADER_LEN {
                let name = String::from_utf8_lossy(&buf[HEADER_LEN..len]).to_string();
                if !name.is_empty() {
                    let mut map = match rx_names.lock() {
                        Ok(m) => m,
                        Err(e) => e.into_inner(),
                    };
                    map.insert(from, name);
                }
            }

            rx_counters.rx.fetch_add(1, Ordering::Relaxed);

            if h.kind == KIND_TEXT && len > HEADER_LEN {
                // Lossy: a message with a bad byte in it is still worth
                // showing, and refusing to display anything is a worse
                // failure than a replacement character.
                let text = String::from_utf8_lossy(&buf[HEADER_LEN..len]).to_string();
                eprintln!("[net] text <- {from} ({} bytes)", len - HEADER_LEN);
                let msg = Message {
                    id: rx_message_id.fetch_add(1, Ordering::Relaxed),
                    from: from.to_string(),
                    text,
                    mine: false,
                    at: unix_millis(),
                };
                let mut log = match rx_messages.lock() {
                    Ok(m) => m,
                    Err(e) => e.into_inner(),
                };
                if log.len() >= MESSAGE_LIMIT {
                    log.pop_front();
                }
                log.push_back(msg);
                continue;
            }

            if h.kind == KIND_AUDIO && len > HEADER_LEN {
                {
                    let mut map = match rx_last_audio.lock() {
                        Ok(m) => m,
                        Err(e) => e.into_inner(),
                    };
                    map.insert(from, Instant::now());
                }

                let (jitter, expected) = windows
                    .entry(from)
                    .or_insert_with(|| (Jitter::new(), None));

                // Loss is counted on audio alone, and per sender. Heartbeats
                // carry their own fixed sequence, and folding several senders
                // into one counter would report gaps that never existed.
                if let Some(want) = *expected {
                    // seq is u16 and wraps every ~22 minutes of talking at 50
                    // packets a second, so this is wrapping_sub and not `>`. A
                    // plain comparison stalls permanently at the wrap.
                    let ahead = h.seq.wrapping_sub(want);
                    if ahead > 0 && ahead < 0x8000 {
                        rx_counters.lost.fetch_add(ahead as u64, Ordering::Relaxed);
                    }
                }
                *expected = Some(h.seq.wrapping_add(1));
                rx_counters.last_seq.store(h.seq as u64, Ordering::Relaxed);

                ready.clear();
                jitter.push(h.seq, buf[HEADER_LEN..len].to_vec(), &mut ready);
                for frame in ready.drain(..) {
                    // try_send: if the decoder is behind, dropping the newest
                    // frame is better than growing a queue of speech nobody
                    // will want by the time it plays.
                    let _ = audio_in.try_send((from, frame));
                }
            }
        }
    })?;

    Ok(Handle {
        stop,
        counters,
        port,
        peer: peer.to_string(),
        socket: tx_sock,
        seq: AtomicU64::new(0),
        ts: AtomicU64::new(0),
        targets,
        target: Arc::new(Mutex::new(None)),
        names,
        heard,
        last_audio,
        messages,
        next_message_id: message_id,
        sent_report: AtomicU64::new(0),
        threads: vec![hb_thread, rx_thread],
    })
}
