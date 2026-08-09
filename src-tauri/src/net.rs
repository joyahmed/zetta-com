//! UDP transport.
//!
//! Started and stopped from the UI rather than from the environment, because
//! discovery (step 3) and push-to-talk (step 4) both need to change peers while
//! the app is running, and startup-only configuration would be written twice.
//!
//! Still no audio on the wire: the payload is a placeholder. What this proves
//! is the socket, the ports, the firewall, and that a structured header
//! survives the trip with its sequence intact.

use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::sync::mpsc::SyncSender;
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
pub const HEADER_LEN: usize = 8;

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

/// Dropping this stops both threads, same contract as `audio::Handle`.
pub struct Handle {
    stop: Arc<AtomicBool>,
    counters: Arc<Counters>,
    port: u16,
    peer: String,
    socket: UdpSocket,
    dest: SocketAddr,
    /// Sequence and timestamp live here rather than in the sender thread
    /// because sends are now driven by the encoder, not by a timer.
    seq: AtomicU64,
    ts: AtomicU64,
}

impl Drop for Handle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

impl Handle {
    /// Send one encoded audio frame. `samples` is the sender's frame size, so
    /// the timestamp advances by what this machine actually captured rather
    /// than by a constant the receiver would have to guess at.
    pub fn send_audio(&self, data: &[u8], samples: u32) {
        let mut buf = [0u8; HEADER_LEN + MAX_PAYLOAD];
        if data.len() > MAX_PAYLOAD {
            return;
        }
        let h = Header {
            ver: VER,
            kind: KIND_AUDIO,
            seq: self.seq.fetch_add(1, Ordering::Relaxed) as u16,
            ts: self.ts.fetch_add(samples as u64, Ordering::Relaxed) as u32,
        };
        h.write(&mut buf);
        buf[HEADER_LEN..HEADER_LEN + data.len()].copy_from_slice(data);

        match self.socket.send_to(&buf[..HEADER_LEN + data.len()], self.dest) {
            Ok(_) => {
                self.counters.tx.fetch_add(1, Ordering::Relaxed);
            }
            Err(e) => eprintln!("[net] tx failed: {e}"),
        }
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

/// Resolve to an IPv4 address specifically.
///
/// A Windows PC name resolves IPv6-link-local first, and anything that takes
/// the first address reaches nobody. Both of v1's "text arrived but voice did
/// not" failures were this, on the sending side.
fn resolve_v4(peer: &str) -> Result<SocketAddr> {
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
pub fn start(port: u16, peer: &str, audio_in: SyncSender<Option<Vec<u8>>>) -> Result<Handle> {
    let dest = resolve_v4(peer)?;

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

    eprintln!("[net] bound 0.0.0.0:{port}, peer {peer} -> {dest}");

    let stop = Arc::new(AtomicBool::new(false));
    let counters = Arc::new(Counters::default());

    // There is no sender thread any more. Sends are driven by the encoder
    // through Handle::send_audio, so the microphone sets the packet rate
    // rather than a timer that would have to be kept in step with it.
    let tx_sock = socket.try_clone().context("cloning socket for sender")?;

    let rx_stop = stop.clone();
    let rx_counters = counters.clone();
    thread::Builder::new().name("net-rx".into()).spawn(move || {
        let mut buf = [0u8; 2048];
        let mut expect: Option<u16> = None;
        let mut jitter = Jitter::new();
        let mut ready: Vec<Option<Vec<u8>>> = Vec::with_capacity(SLOTS);

        while !rx_stop.load(Ordering::Relaxed) {
            let len = match socket.recv_from(&mut buf) {
                Ok((len, _from)) => len,
                // Timeouts are how this loop breathes; they are not errors.
                Err(_) => continue,
            };

            let Some(h) = Header::parse(&buf[..len]) else {
                rx_counters.bad.fetch_add(1, Ordering::Relaxed);
                continue;
            };

            if let Some(want) = expect {
                // seq is u16 and wraps every ~22 minutes of talking at 50
                // packets a second, so this is wrapping_sub and not `>`. A
                // plain comparison stalls permanently at the wrap.
                let ahead = h.seq.wrapping_sub(want);
                if ahead > 0 && ahead < 0x8000 {
                    rx_counters.lost.fetch_add(ahead as u64, Ordering::Relaxed);
                }
            }
            expect = Some(h.seq.wrapping_add(1));

            rx_counters.rx.fetch_add(1, Ordering::Relaxed);
            rx_counters.last_seq.store(h.seq as u64, Ordering::Relaxed);

            if h.kind == KIND_AUDIO && len > HEADER_LEN {
                ready.clear();
                jitter.push(h.seq, buf[HEADER_LEN..len].to_vec(), &mut ready);
                for frame in ready.drain(..) {
                    // try_send: if the decoder is behind, dropping the newest
                    // frame is better than growing a queue of speech nobody
                    // will want by the time it plays.
                    let _ = audio_in.try_send(frame);
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
        dest,
        seq: AtomicU64::new(0),
        ts: AtomicU64::new(0),
    })
}
