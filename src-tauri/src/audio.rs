//! Capture, encode, decode, playback. Knows nothing about sockets — `session`
//! wires the two ends to the network.
//!
//! Thread layout — this is the design:
//!
//!   [input callback]   real-time   mic → mono i16 → ring A
//!   [encoder]          normal      ring A → 20 ms frame → Opus → frames_out
//!   [decoder]          normal      frames_in → Opus → ring B
//!   [output callback]  real-time   ring B → speakers, silence on underrun
//!
//! The two callbacks run on real-time threads owned by the audio driver. No
//! allocation, no locks, no logging inside them — `println!` takes the stdout
//! lock and is audible as a crackle.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use audiopus::coder::{Decoder, Encoder};
use audiopus::{Application, Bitrate, Channels, SampleRate};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, SampleFormat, SupportedStreamConfig};
use ringbuf::traits::{Consumer, Observer, Producer, Split};
use ringbuf::HeapRb;

/// Every rate Opus accepts, best first. We take the best the device offers
/// rather than demanding 48 kHz, because a Bluetooth headset in Hands-Free
/// mode offers 16 kHz and nothing else, and refusing to start on it is not
/// acceptable behaviour for an app whose whole purpose is other people's PCs.
///
/// Note these are plain `u32`s: in cpal 0.18 `SampleRate` is a type alias, not
/// the tuple struct it used to be, so there is no `.0` anywhere below.
const OPUS_RATES: [u32; 5] = [48_000, 24_000, 16_000, 12_000, 8_000];
/// Formats the stream callbacks below implement, best first. F32 is what
/// WASAPI hands out natively on Windows, so it costs no conversion.
const USABLE_FORMATS: [SampleFormat; 2] = [SampleFormat::F32, SampleFormat::I16];
/// 20 ms at the highest rate — the largest frame any buffer here has to hold.
const MAX_FRAME: usize = 960;
/// A 20 ms Opus packet never exceeds 1275 bytes. This is slack.
const MAX_PACKET: usize = 4_000;
/// How many 20 ms frames a ring may hold — roughly 160 ms of slack.
const RING_FRAMES: usize = 8;
const BITRATE: i32 = 32_000;
/// How often to check whether the default devices have changed underneath us.
const DEVICE_POLL: Duration = Duration::from_millis(1_000);
/// Pause before rebuilding, so a device that is flapping cannot spin the loop
/// and so the previous generation's threads have let go first.
const RETRY_DELAY: Duration = Duration::from_millis(700);
/// Consecutive one-second polls with no playback callback at all before the
/// stream is declared dead. Three rather than one: a machine waking from sleep
/// or a driver reloading can stall briefly and recover on its own, and a
/// rebuild is an audible interruption.
const DEAD_STREAM_POLLS: u32 = 3;
/// How many rebuilds a dead capture stream gets at the fast cadence above
/// before the retry slows down.
///
/// It never stops retrying, because stopping is the bug this exists to remove:
/// a microphone that cannot be recovered leaves you unable to talk for as long
/// as the app runs. But a rebuild interrupts playback for about a second, so
/// retrying every four seconds forever would trade not being able to talk for
/// not being able to listen either. Two fast attempts cover the case this was
/// written for — endpoints that are not ready yet at login or straight after an
/// install — and anything still dead after those is not in a hurry.
const CAPTURE_REBUILDS: u32 = 2;
/// The slow cadence: a second of interrupted playback a minute, against a
/// microphone that has already failed to come back twice.
const CAPTURE_BACKOFF_POLLS: u32 = 60;

/// Which devices to use, by name, or the system default when `None`.
///
/// Names rather than indices: an index is meaningless the moment a device is
/// plugged in or removed, and it would silently start pointing at somebody
/// else's headset. A name that disappears falls back to the default and says
/// so, which is recoverable; an index that shifts is not even detectable.
#[derive(Clone, Debug, Default)]
pub struct Prefs {
    pub input: Option<String>,
    pub output: Option<String>,
}

/// Everything the host can offer, for the picker in Settings.
pub fn devices() -> (Vec<String>, Vec<String>) {
    let host = cpal::default_host();
    let ins = host
        .input_devices()
        .map(|it| it.map(|d| device_name(&d)).collect())
        .unwrap_or_default();
    let outs = host
        .output_devices()
        .map(|it| it.map(|d| device_name(&d)).collect())
        .unwrap_or_default();
    (ins, outs)
}

/// A fingerprint of the devices actually in use, so the poll notices a change.
///
/// Polled rather than only signalled, because a device *appearing* raises no
/// stream error: the app that started with no microphone is exactly the one
/// that will never be told when one is plugged in.
///
/// It accounts for the preference, not just the system default: with a device
/// pinned by name, the default changing underneath is none of our business, and
/// rebuilding on it would tear the stream down for nothing.
fn describe_devices(prefs: &Prefs) -> (Option<String>, Option<String>) {
    let host = cpal::default_host();
    let resolve = |want: &Option<String>, input: bool| -> Option<String> {
        if let Some(name) = want {
            return Some(name.clone());
        }
        if input {
            host.default_input_device().map(|d| device_name(&d))
        } else {
            host.default_output_device().map(|d| device_name(&d))
        }
    };
    (resolve(&prefs.input, true), resolve(&prefs.output, false))
}

/// Dropping this stops the pipeline: the audio thread sees the flag, returns,
/// and the streams drop with it.
pub struct Handle {
    stop: Arc<AtomicBool>,
}

impl Drop for Handle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// Streams stay on the thread that built them and are never moved off it —
/// `cpal::Stream` is not `Send` on every backend. Either may be absent: a PC
/// with no microphone still listens, and one with no output still sends.
struct Streams {
    _input: Option<cpal::Stream>,
    _output: Option<cpal::Stream>,
    /// Handed back so the poll loop can watch the playback callback for signs
    /// of life. A stream can be open, healthy by every API it exposes, and
    /// never called again — which is what happens when the app starts at login
    /// before the audio endpoint is ready.
    stats: Arc<Stats>,
}

/// Counters, so a glitch says which link produced it instead of being guessed
/// at by ear. Atomic increments are wait-free on x86, which is why they are
/// tolerable inside the real-time callbacks when a lock or a log would not be.
#[derive(Default)]
struct Stats {
    /// Samples the capture callback threw away because ring A was full: the
    /// encoder is not keeping up.
    in_drops: AtomicU64,
    /// Every capture callback, full or silent. `out_calls` for the microphone,
    /// and the only counter that can tell a dead capture stream from a quiet
    /// room: `frames` and `in_drops` below only advance while the talk key is
    /// held, so with the key up they read zero either way.
    in_calls: AtomicU64,
    /// Frames encoded.
    frames: AtomicU64,
    /// Output callbacks that produced silence because ring B was short.
    underruns: AtomicU64,
    /// Every output callback, silent or not. Counts as a pulse: if this stops
    /// advancing while a stream is open, the driver is no longer asking for
    /// audio and playback is dead however healthy it looks.
    out_calls: AtomicU64,
    /// Times playback had to re-bank after running dry. A steady count here is
    /// the signature of a periodic stutter.
    primes: AtomicU64,
    /// Latest ring occupancies, published for the reporter thread.
    a_occ: AtomicU64,
    b_occ: AtomicU64,
}

/// One encoded 20 ms frame, carrying the sample count it represents. The count
/// travels with the frame because the packet header's timestamp advances by the
/// *sender's* frame size, and that depends on the sender's microphone rate —
/// 320 on a 16 kHz headset, 960 at 48 kHz. The receiver must never assume it.
pub struct Frame {
    pub data: Vec<u8>,
    pub samples: u32,
}

/// Audio with both ends exposed: frames leaving the microphone, and frames
/// arriving to be played. `net` drains one and fills the other, and neither
/// module knows the other exists.
pub struct Pipeline {
    pub frames_out: Receiver<Frame>,
    /// Keyed by who sent it. Each remote talker needs its own decoder, because
    /// Opus decoder state is a running conversation with one encoder — feeding
    /// two people's packets through a single decoder produces garbage, not a
    /// mix.
    ///
    /// `None` means a frame the transport knows is missing. It is not the same
    /// as no frame at all: the decoder conceals a gap it is told about, and a
    /// concealed 20 ms is far less audible than a skipped one.
    pub frames_in: SyncSender<(SocketAddr, Option<Vec<u8>>)>,
    pub handle: Handle,
}

/// `transmit` is push-to-talk: true only while the key is held.
///
/// It gates both directions, and the second one is the point of the whole
/// architecture. Nothing is encoded or sent while it is false, and *local
/// playback is silenced while it is true* — so your speakers cannot be playing
/// someone else's voice into your own open microphone. That mute is the entire
/// echo strategy, and the reason full duplex and acoustic echo cancellation
/// were rejected at the start: the echo path never exists, so there is nothing
/// to cancel.
pub fn start(transmit: Arc<AtomicBool>, prefs: Prefs) -> Result<Pipeline> {
    let stop = Arc::new(AtomicBool::new(false));
    let (ready_tx, ready_rx) = mpsc::channel::<Result<()>>();

    // Bounded on purpose. If the network side stalls, dropping speech is the
    // correct response and an unbounded queue is not: audio that arrives late
    // enough is worth less than the memory holding it.
    let (out_tx, out_rx) = mpsc::sync_channel::<Frame>(8);
    let (in_tx, in_rx) = mpsc::sync_channel::<(SocketAddr, Option<Vec<u8>>)>(32);

    // The receiver is shared rather than moved, because the pipeline is rebuilt
    // whenever devices change and each generation needs it back. Only one
    // decoder runs at a time, so the lock is never contended — a new generation
    // simply waits for the old one to let go.
    let in_rx = Arc::new(Mutex::new(in_rx));

    let thread_stop = stop.clone();
    thread::Builder::new()
        .name("audio".into())
        .spawn(move || {
            let mut first = true;
            // Outside the loop, because it counts rebuilds *across* generations
            // — the whole question it answers is whether the last rebuild fixed
            // anything.
            let mut cap_attempts = 0u32;
            while !thread_stop.load(Ordering::Relaxed) {
                // Signals this generation to stop, independently of the whole
                // pipeline stopping.
                let generation = Arc::new(AtomicBool::new(false));
                let rebuild = Arc::new(AtomicBool::new(false));

                let built = build(
                    &thread_stop,
                    &generation,
                    &rebuild,
                    &transmit,
                    out_tx.clone(),
                    in_rx.clone(),
                    &prefs,
                );

                let streams = match built {
                    Ok(s) => {
                        if first {
                            let _ = ready_tx.send(Ok(()));
                            first = false;
                        }
                        s
                    }
                    Err(e) => {
                        if first {
                            let _ = ready_tx.send(Err(e));
                            return;
                        }
                        // Later failures are usually a device mid-reconnect.
                        // Report and try again rather than giving up: a
                        // headset that dropped will be back in a moment.
                        eprintln!("[audio] rebuild failed, retrying: {e:#}");
                        thread::sleep(RETRY_DELAY);
                        continue;
                    }
                };

                // Watch for the reason to start over.
                let watching = describe_devices(&prefs);
                let has_output = streams._output.is_some();
                let mut last_pulse = streams.stats.out_calls.load(Ordering::Relaxed);
                let mut quiet = 0u32;
                let has_input = streams._input.is_some();
                let mut last_in = streams.stats.in_calls.load(Ordering::Relaxed);
                let mut in_quiet = 0u32;

                while !thread_stop.load(Ordering::Relaxed)
                    && !rebuild.load(Ordering::Relaxed)
                {
                    thread::sleep(DEVICE_POLL);
                    // Polled as well as signalled, because a device *appearing*
                    // raises no stream error at all — and that is the case that
                    // bites: the app starts with no microphone, one is plugged
                    // in, and nothing ever tells it.
                    if describe_devices(&prefs) != watching {
                        eprintln!("[audio] default devices changed, rebuilding");
                        break;
                    }

                    // A stream can be open, report no error, and never be
                    // called again. That is what an app started at login before
                    // the audio endpoint was ready looks like from in here: the
                    // device name has not changed, so the check above is happy,
                    // and playback is silent until the session is restarted.
                    // Logging out and back in should not be the remedy for
                    // anything.
                    //
                    // The playback callback runs on the driver's clock whether
                    // or not there is anything to play, so it stopping is not
                    // ambiguous the way a silent counter would be.
                    if has_output {
                        let pulse = streams.stats.out_calls.load(Ordering::Relaxed);
                        if pulse == last_pulse {
                            quiet += 1;
                            if quiet >= DEAD_STREAM_POLLS {
                                eprintln!(
                                    "[audio] playback has not been called for {DEAD_STREAM_POLLS}s, rebuilding"
                                );
                                break;
                            }
                        } else {
                            quiet = 0;
                            last_pulse = pulse;
                        }
                    }

                    // The same test, for the microphone — and this is the one
                    // that was missing. A capture stream can open, report no
                    // error, and never be called, which is what a machine that
                    // started before its audio endpoints were ready looks like
                    // from in here. Nothing above notices: the device name has
                    // not changed, playback is fine, and the only capture
                    // counters move solely while the talk key is held. So the
                    // app went on believing it had a microphone, everyone else
                    // heard silence when the key went down, and the state
                    // survived until the process did — which is exactly why
                    // logging out and back in looked like the remedy.
                    if has_input {
                        // Fast while the cause still looks like a device that
                        // has not finished waking up, slow once it does not.
                        let patience = if cap_attempts < CAPTURE_REBUILDS {
                            DEAD_STREAM_POLLS
                        } else {
                            CAPTURE_BACKOFF_POLLS
                        };
                        let pulse = streams.stats.in_calls.load(Ordering::Relaxed);
                        if pulse == last_in {
                            in_quiet += 1;
                            if in_quiet >= patience {
                                cap_attempts += 1;
                                eprintln!(
                                    "[audio] capture has not been called for {patience}s, rebuilding (attempt {cap_attempts})"
                                );
                                break;
                            }
                        } else {
                            in_quiet = 0;
                            last_in = pulse;
                            // Heard from, so the attempts above count a run of
                            // consecutive failures rather than the lifetime of
                            // the app: the next time this happens it gets the
                            // fast cadence again.
                            cap_attempts = 0;
                        }
                    }
                }

                generation.store(true, Ordering::Relaxed);
                drop(streams);
                if !thread_stop.load(Ordering::Relaxed) {
                    // Long enough for the old worker threads to notice and let
                    // go of the shared receiver, and to stop a flapping device
                    // from spinning this loop.
                    thread::sleep(RETRY_DELAY);
                }
            }
        })?;

    // Two `?`: one for the channel dying, one for the build failing.
    ready_rx
        .recv()
        .context("audio thread died before reporting")??;

    Ok(Pipeline {
        frames_out: out_rx,
        frames_in: in_tx,
        handle: Handle { stop },
    })
}

type SharedFrames = Arc<Mutex<Receiver<(SocketAddr, Option<Vec<u8>>)>>>;

fn build(
    stop: &Arc<AtomicBool>,
    generation: &Arc<AtomicBool>,
    rebuild: &Arc<AtomicBool>,
    transmit: &Arc<AtomicBool>,
    out_tx: SyncSender<Frame>,
    in_rx: SharedFrames,
    prefs: &Prefs,
) -> Result<Streams> {
    let host = cpal::default_host();

    // Capture and playback are independently optional. Most people on an
    // intercom never talk, so a PC with no microphone — or one whose headset
    // has not connected, or whose mic is off in privacy settings — must still
    // hear everyone. Losing playback because there is no capture device is
    // exactly backwards.
    let in_sel = select(&host, true, prefs.input.as_deref());
    let out_sel = select(&host, false, prefs.output.as_deref());

    if in_sel.is_none() && out_sel.is_none() {
        eprintln!("[audio] no usable audio device at all — text only");
    }

    let stats = Arc::new(Stats::default());

    // A stream that refuses to build is treated exactly like a device that was
    // not there. It is the same situation from the app's point of view, and
    // failing the whole pipeline over one direction is how a stereo-only
    // speaker ended up preventing text messages from working.
    let input_stream = match in_sel {
        Some((device, cfg)) => {
            match build_capture(
                &device, &cfg, stop, generation, rebuild, transmit, &stats, out_tx,
            ) {
                Ok(s) => Some(s),
                Err(e) => {
                    eprintln!("[audio] capture unavailable, listening only: {e:#}");
                    None
                }
            }
        }
        None => {
            eprintln!("[audio] no usable input device — listening only");
            None
        }
    };

    let output_stream = match out_sel {
        Some((device, cfg)) => {
            match build_playback(
                &device, &cfg, stop, generation, rebuild, transmit, &stats, in_rx,
            ) {
                Ok(s) => Some(s),
                Err(e) => {
                    eprintln!("[audio] playback unavailable, sending only: {e:#}");
                    None
                }
            }
        }
        None => {
            eprintln!("[audio] no usable output device — sending only");
            None
        }
    };

    // One line a second. Cheap enough to leave in while the pipeline is being
    // tuned, and it turns "it sounds wrong" into a number that names the link.
    let report_stop = stop.clone();
    let report_stats = stats.clone();
    thread::Builder::new()
        .name("audio-stats".into())
        .spawn(move || {
            let stats = report_stats;
            let (mut p_drops, mut p_frames, mut p_under, mut p_primes) = (0u64, 0u64, 0u64, 0u64);
            let mut p_mic = 0u64;
            while !report_stop.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_secs(1));
                let drops = stats.in_drops.load(Ordering::Relaxed);
                let frames = stats.frames.load(Ordering::Relaxed);
                let under = stats.underruns.load(Ordering::Relaxed);
                let primes = stats.primes.load(Ordering::Relaxed);
                // `mic` is capture callbacks in the last second, and it is the
                // first number to read when somebody cannot be heard: it is the
                // microphone's pulse, and unlike `frames` it does not depend on
                // anyone holding the talk key. Zero here with a device listed
                // above means the stream is open and dead.
                let mic = stats.in_calls.load(Ordering::Relaxed);
                eprintln!(
                    "[audio] 1s mic {} frames {} drops {} underruns {} reprimes {} ringA {} ringB {}",
                    mic - p_mic,
                    frames - p_frames,
                    drops - p_drops,
                    under - p_under,
                    primes - p_primes,
                    stats.a_occ.load(Ordering::Relaxed),
                    stats.b_occ.load(Ordering::Relaxed),
                );
                (p_drops, p_frames, p_under, p_primes) = (drops, frames, under, primes);
                p_mic = mic;
            }
        })?;

    if let Some(s) = &input_stream {
        s.play()?;
    }
    if let Some(s) = &output_stream {
        s.play()?;
    }

    Ok(Streams {
        _input: input_stream,
        _output: output_stream,
        stats,
    })
}

/// Builds the error callback for a stream.
///
/// A stream error is almost always the device going away, so it asks for a
/// rebuild rather than only complaining. Returned fresh per call because it
/// captures the flag, and cpal 0.18 collapsed StreamError and friends into one
/// `cpal::Error`.
fn on_err(rebuild: &Arc<AtomicBool>) -> impl FnMut(cpal::Error) + Send + 'static {
    let rebuild = rebuild.clone();
    move |e| {
        eprintln!("[audio] stream error, will rebuild: {e}");
        rebuild.store(true, Ordering::Relaxed);
    }
}

/// Microphone → ring A → Opus → `out_tx`. Owns ring A entirely, so nothing
/// about it exists when there is no input device.
fn build_capture(
    device: &Device,
    cfg: &SupportedStreamConfig,
    stop: &Arc<AtomicBool>,
    generation: &Arc<AtomicBool>,
    rebuild: &Arc<AtomicBool>,
    transmit: &Arc<AtomicBool>,
    stats: &Arc<Stats>,
    out_tx: SyncSender<Frame>,
) -> Result<cpal::Stream> {
    let hz = cfg.sample_rate();
    // What Opus will be fed. Equal to the device rate on a device that offers
    // an Opus rate, which is the common case and costs nothing.
    let opus_hz = if OPUS_RATES.contains(&hz) {
        hz
    } else {
        nearest_opus(hz)
    };
    // Ring A holds device-rate samples; the encoder consumes a device-rate
    // 20 ms and produces an Opus-rate 20 ms.
    let frame = (hz / 50) as usize; // 20 ms at the device rate
    let opus_frame = (opus_hz / 50) as usize; // 20 ms at the Opus rate
    let rate = opus_rate(opus_hz)?;
    let ch = cfg.channels() as usize;
    let stream_cfg = cfg.config();

    eprintln!(
        "[audio] in  {:?} {}Hz {:?} {}ch frame {}",
        device_name(device),
        hz,
        cfg.sample_format(),
        cfg.channels(),
        frame
    );

    // Sized in frames, not seconds. A ring's capacity is a latency ceiling: if
    // it can hold a second of audio then one transient stall fills it with a
    // second of audio and it never gives that back, so you hear yourself late
    // for the rest of the session. At RING_FRAMES the same stall costs 160 ms
    // and is paid back by dropping, which is the right trade for speech.
    let (mut prod, mut cons) = HeapRb::<i16>::new(frame * RING_FRAMES).split();

    let st = stats.clone();
    let stream = match cfg.sample_format() {
        SampleFormat::F32 => device.build_input_stream(
            stream_cfg.clone(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                st.in_calls.fetch_add(1, Ordering::Relaxed);
                for f in data.chunks_exact(ch) {
                    let sum: f32 = f.iter().sum();
                    let v = (sum / ch as f32).clamp(-1.0, 1.0);
                    // Full ring drops the sample. Correct behaviour here:
                    // never block a real-time callback.
                    if prod.try_push((v * i16::MAX as f32) as i16).is_err() {
                        st.in_drops.fetch_add(1, Ordering::Relaxed);
                    }
                }
            },
            on_err(rebuild),
            None,
        )?,
        SampleFormat::I16 => device.build_input_stream(
            stream_cfg.clone(),
            move |data: &[i16], _: &cpal::InputCallbackInfo| {
                st.in_calls.fetch_add(1, Ordering::Relaxed);
                for f in data.chunks_exact(ch) {
                    let sum: i32 = f.iter().map(|s| *s as i32).sum();
                    if prod.try_push((sum / ch as i32) as i16).is_err() {
                        st.in_drops.fetch_add(1, Ordering::Relaxed);
                    }
                }
            },
            on_err(rebuild),
            None,
        )?,
        f => return Err(anyhow!("unsupported input sample format {f:?}")),
    };

    let enc_stop = stop.clone();
    let enc_generation = generation.clone();
    let enc_transmit = transmit.clone();
    let enc_stats = stats.clone();
    thread::Builder::new().name("encoder".into()).spawn(move || {
        let mut enc = match Encoder::new(rate, Channels::Mono, Application::Voip) {
            Ok(e) => e,
            Err(e) => return eprintln!("[audio] encoder: {e}"),
        };
        if let Err(e) = enc.set_bitrate(Bitrate::BitsPerSecond(BITRATE)) {
            eprintln!("[audio] set_bitrate: {e}");
        }

        // Sized for the worst case, sliced to the real frame. Allocated once,
        // outside the loop, so the loop itself never allocates.
        let mut pcm = [0i16; MAX_FRAME];
        let mut resampled = [0i16; MAX_FRAME];
        let mut packet = [0u8; MAX_PACKET];
        // None when the device already runs at an Opus rate, so the common path
        // has no resampling in it at all.
        let mut rs = (hz != opus_hz).then(|| Resampler::new(hz, opus_hz));

        while !enc_stop.load(Ordering::Relaxed) && !enc_generation.load(Ordering::Relaxed) {
            let have = cons.occupied_len();
            enc_stats.a_occ.store(have as u64, Ordering::Relaxed);
            if have < frame {
                thread::sleep(Duration::from_millis(2));
                continue;
            }
            cons.pop_slice(&mut pcm[..frame]);

            // Drained either way. Skipping the pop would let the ring fill
            // while the key is up, and the first thing anyone heard after
            // pressing it would be a hundred milliseconds of stale room noise.
            if !enc_transmit.load(Ordering::Relaxed) {
                continue;
            }

            // Opus will only accept exactly 20 ms at its own rate, so a short
            // resample is padded rather than encoded at the wrong length.
            let payload: &[i16] = match rs.as_mut() {
                None => &pcm[..frame],
                Some(r) => {
                    let n = r.process(&pcm[..frame], &mut resampled[..opus_frame]);
                    for s in resampled.iter_mut().take(opus_frame).skip(n) {
                        *s = 0;
                    }
                    &resampled[..opus_frame]
                }
            };

            let n = match enc.encode(payload, &mut packet) {
                Ok(n) => n,
                Err(e) => {
                    eprintln!("[audio] encode: {e}");
                    continue;
                }
            };
            enc_stats.frames.fetch_add(1, Ordering::Relaxed);

            // try_send, not send: a full channel means the network side has
            // stalled, and blocking here would back the stall up into the
            // capture ring and then into dropped microphone samples.
            // The Opus-rate count, not the device-rate one. This travels in the
            // packet header as the timestamp increment, and the receiver has no
            // idea what rate the sender's microphone runs at — only what was
            // encoded.
            let _ = out_tx.try_send(Frame {
                data: packet[..n].to_vec(),
                samples: opus_frame as u32,
            });
        }
    })?;

    Ok(stream)
}

/// `in_rx` → Opus → ring B → speakers. Owns ring B entirely.
fn build_playback(
    device: &Device,
    cfg: &SupportedStreamConfig,
    stop: &Arc<AtomicBool>,
    generation: &Arc<AtomicBool>,
    rebuild: &Arc<AtomicBool>,
    transmit: &Arc<AtomicBool>,
    stats: &Arc<Stats>,
    in_rx: SharedFrames,
) -> Result<cpal::Stream> {
    let hz = cfg.sample_rate();
    // Decode at an Opus rate, then resample up or down to whatever the speakers
    // actually run at. Ring B holds device-rate samples, so the real-time
    // callback stays exactly as simple as it was.
    let opus_hz = if OPUS_RATES.contains(&hz) {
        hz
    } else {
        nearest_opus(hz)
    };
    let frame = (hz / 50) as usize; // 20 ms at the device rate
    let opus_frame = (opus_hz / 50) as usize; // 20 ms at the Opus rate
    let rate = opus_rate(opus_hz)?;
    let ch = cfg.channels() as usize;
    let stream_cfg = cfg.config();

    eprintln!(
        "[audio] out {:?} {}Hz {:?} {}ch frame {}",
        device_name(device),
        hz,
        cfg.sample_format(),
        cfg.channels(),
        frame
    );

    let (mut prod, mut cons) = HeapRb::<i16>::new(frame * RING_FRAMES).split();

    // Playback priming. The decoder delivers a whole frame at once every 20 ms
    // while this callback asks for samples on its own schedule, so the ring is
    // routinely a few samples short of what is being asked for. Filling that
    // shortfall with zeros puts a sub-millisecond gap inside a frame, and a gap
    // that often is heard as a screech, not as silence. So: stay quiet until a
    // few frames have banked, then drain whole callbacks only; if it ever runs
    // dry, go quiet and bank again. Silence in clean chunks is inaudible.
    let prime = frame * 3;

    let stream = match cfg.sample_format() {
        SampleFormat::F32 => {
            // Captured by the FnMut closure. Only this callback touches it, so
            // it needs no atomic and no lock — which matters on an RT thread.
            let mut primed = false;
            let st = stats.clone();
            let out_transmit = transmit.clone();
            device.build_output_stream(
                stream_cfg.clone(),
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    st.out_calls.fetch_add(1, Ordering::Relaxed);
                    // Silent while transmitting. This is the echo prevention:
                    // without it the speakers play the far end into the open
                    // microphone and everyone hears themselves back.
                    if out_transmit.load(Ordering::Relaxed) {
                        data.fill(0.0);
                        return;
                    }
                    let have = cons.occupied_len();
                    st.b_occ.store(have as u64, Ordering::Relaxed);
                    let need = data.len() / ch;
                    if !primed {
                        if have < prime {
                            st.underruns.fetch_add(1, Ordering::Relaxed);
                            data.fill(0.0);
                            return;
                        }
                        primed = true;
                        st.primes.fetch_add(1, Ordering::Relaxed);
                    }
                    if have < need {
                        primed = false;
                        st.underruns.fetch_add(1, Ordering::Relaxed);
                        data.fill(0.0);
                        return;
                    }
                    for f in data.chunks_exact_mut(ch) {
                        let v = cons.try_pop().unwrap_or(0) as f32 / i16::MAX as f32;
                        f.fill(v);
                    }
                },
                on_err(rebuild),
                None,
            )?
        }
        SampleFormat::I16 => {
            let mut primed = false;
            let st = stats.clone();
            let out_transmit = transmit.clone();
            device.build_output_stream(
                stream_cfg.clone(),
                move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                    st.out_calls.fetch_add(1, Ordering::Relaxed);
                    if out_transmit.load(Ordering::Relaxed) {
                        data.fill(0);
                        return;
                    }
                    let have = cons.occupied_len();
                    st.b_occ.store(have as u64, Ordering::Relaxed);
                    let need = data.len() / ch;
                    if !primed {
                        if have < prime {
                            st.underruns.fetch_add(1, Ordering::Relaxed);
                            data.fill(0);
                            return;
                        }
                        primed = true;
                        st.primes.fetch_add(1, Ordering::Relaxed);
                    }
                    if have < need {
                        primed = false;
                        st.underruns.fetch_add(1, Ordering::Relaxed);
                        data.fill(0);
                        return;
                    }
                    for f in data.chunks_exact_mut(ch) {
                        f.fill(cons.try_pop().unwrap_or(0));
                    }
                },
                on_err(rebuild),
                None,
            )?
        }
        f => return Err(anyhow!("unsupported output sample format {f:?}")),
    };

    let dec_stop = stop.clone();
    let dec_generation = generation.clone();
    thread::Builder::new().name("mixer".into()).spawn(move || {
        // Held for this generation's lifetime. The next one blocks here until
        // we let go, which is what keeps a rebuild from running two mixers.
        let in_rx = match in_rx.lock() {
            Ok(r) => r,
            Err(e) => e.into_inner(),
        };
        // One decoder and one pending queue per talker. The decoder is the
        // reason: Opus decoder state is a running conversation with a single
        // encoder, so pushing two people's packets through one produces
        // garbage rather than a mix.
        let mut sources: HashMap<SocketAddr, (Decoder, Vec<i16>)> = HashMap::new();
        let mut out = [0i16; MAX_FRAME];
        let mut mixed = [0i32; MAX_FRAME];
        let mut speakers = [0i16; MAX_FRAME];
        // None when the speakers already run at an Opus rate.
        let mut rs = (hz != opus_hz).then(|| Resampler::new(opus_hz, hz));

        while !dec_stop.load(Ordering::Relaxed) && !dec_generation.load(Ordering::Relaxed) {
            // Timed out rather than blocking, so the thread notices the stop
            // flag instead of parking forever on a silent peer.
            let (from, packet) = match in_rx.recv_timeout(Duration::from_millis(200)) {
                Ok(p) => p,
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => break,
            };

            let entry = match sources.entry(from) {
                std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
                std::collections::hash_map::Entry::Vacant(e) => {
                    match Decoder::new(rate, Channels::Mono) {
                        Ok(d) => e.insert((d, Vec::with_capacity(MAX_FRAME * 4))),
                        Err(err) => {
                            eprintln!("[audio] decoder for {from}: {err}");
                            continue;
                        }
                    }
                }
            };

            // A `None` is a frame the transport knows never arrived, and
            // handing that to Opus is packet-loss concealment: it synthesises a
            // plausible 20 ms from what it just heard. Skipping the frame
            // instead gives a click; this gives a smear you have to listen for.
            //
            // Decoding to the local frame size rather than the sender's is also
            // what converts the rate: libopus resamples internally, so nothing
            // here has to, and the two machines never have to agree on a rate.
            let decoded = match &packet {
                Some(p) => entry.0.decode(Some(&p[..]), &mut out[..opus_frame], false),
                None => entry.0.decode(None::<&[u8]>, &mut out[..opus_frame], false),
            };
            match decoded {
                Ok(got) => entry.1.extend_from_slice(&out[..got]),
                Err(e) => eprintln!("[audio] decode from {from}: {e}"),
            }

            // Mix whatever every source has ready. Half-duplex means one person
            // should be talking at a time, but nothing enforces that until
            // push-to-talk exists, and two people overlapping must sound like
            // two people rather than like corruption.
            loop {
                let ready = sources
                    .values()
                    .filter(|(_, q)| q.len() >= opus_frame)
                    .count();
                if ready == 0 {
                    break;
                }
                mixed[..opus_frame].fill(0);
                for (_, q) in sources.values_mut() {
                    if q.len() < opus_frame {
                        continue;
                    }
                    for (m, s) in mixed[..opus_frame].iter_mut().zip(q.drain(..opus_frame)) {
                        *m += s as i32;
                    }
                }
                // Sum in i32 and clamp once. Summing in i16 would wrap, and a
                // wrap is heard as a crack far louder than the clipping it
                // replaces.
                for (o, m) in out[..opus_frame].iter_mut().zip(&mixed[..opus_frame]) {
                    *o = (*m).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
                }

                // Mixed at the Opus rate, played at the device rate. Resampling
                // after the mix rather than per source does the work once
                // however many people are talking.
                match rs.as_mut() {
                    None => {
                        prod.push_slice(&out[..opus_frame]);
                    }
                    Some(r) => {
                        let n = r.process(&out[..opus_frame], &mut speakers[..frame]);
                        prod.push_slice(&speakers[..n]);
                    }
                }
            }
        }
    })?;

    Ok(stream)
}

/// The default device for a direction and a config we can actually use, or
/// `None` with the reason logged. Returning `None` rather than an error is the
/// point: one missing direction must not take the other down with it.
fn select(
    host: &Host,
    input: bool,
    want: Option<&str>,
) -> Option<(Device, SupportedStreamConfig)> {
    let which = if input { "input" } else { "output" };

    // A chosen device that is no longer present falls back to the default and
    // says so. Refusing to start because a headset was unplugged would be the
    // wrong trade for an app whose job is to be reachable.
    let chosen = want.and_then(|name| {
        let list = if input {
            host.input_devices().ok()
        } else {
            host.output_devices().ok()
        };
        let found = list.and_then(|mut it| it.find(|d| device_name(d) == name));
        if found.is_none() {
            eprintln!("[audio] chosen {which} device {name:?} is not here, using the default");
        }
        found
    });

    let device = match chosen {
        Some(d) => Some(d),
        None if input => host.default_input_device(),
        None => host.default_output_device(),
    };
    let device = match device {
        Some(d) => d,
        None => {
            eprintln!("[audio] no default {which} device");
            return None;
        }
    };
    match pick(&device, input) {
        Ok(cfg) => Some((device, cfg)),
        Err(e) => {
            eprintln!("[audio] {e:#}");
            None
        }
    }
}

/// The Opus rate to run at for a device that does not offer one itself.
///
/// The nearest at or below the device's rate, so we never invent bandwidth the
/// microphone never captured — 44.1 kHz becomes 24 kHz rather than 48 kHz.
/// Speech is entirely intelligible at 24 kHz and this halves the work.
fn nearest_opus(hz: u32) -> u32 {
    OPUS_RATES
        .iter()
        .copied()
        .find(|&r| r <= hz)
        .unwrap_or(OPUS_RATES[OPUS_RATES.len() - 1])
}

/// Linear resampling between a device's rate and an Opus rate.
///
/// Needed because WASAPI shared mode accepts only the device's mix format, and
/// plenty of machines mix at 44.1 kHz — a rate Opus does not have. Without this
/// the whole direction was discarded: `pick` returned an error, `select` turned
/// that into `None`, and a PC with a 44.1 kHz microphone silently could not
/// talk at all. Being heard slightly imperfectly beats not being heard.
///
/// Linear, and deliberately so. A sinc resampler is audibly better on music and
/// close to inaudible on 32 kbps speech that has been through Opus, and it
/// would cost a dependency, a fixed block size to design the rings around, and
/// latency. This runs on the encoder and decoder threads, never in a real-time
/// callback.
///
/// `pos` persists across calls. Resetting it per chunk would put a
/// discontinuity at every buffer boundary — fifty clicks a second.
struct Resampler {
    ratio: f64,
    pos: f64,
    last: i16,
}

impl Resampler {
    fn new(from_hz: u32, to_hz: u32) -> Self {
        Self {
            ratio: from_hz as f64 / to_hz as f64,
            pos: 0.0,
            last: 0,
        }
    }

    /// Fills `out` from `input`, which must hold one chunk's worth at the
    /// source rate. Returns how many samples were written.
    fn process(&mut self, input: &[i16], out: &mut [i16]) -> usize {
        if input.is_empty() {
            return 0;
        }
        let last = input.len() - 1;
        let mut written = 0;
        for slot in out.iter_mut() {
            let i = self.pos.floor() as usize;
            if i > last {
                break;
            }
            let frac = self.pos - i as f64;
            let a = input[i] as f64;
            // At the final sample there is no next one yet, so hold it rather
            // than stopping short. Stopping short left the caller a sample
            // under a full frame, which it then padded with a zero — a
            // discontinuity fifty times a second. Holding is inaudible.
            let b = input[if i < last { i + 1 } else { last }] as f64;
            *slot = (a + (b - a) * frac) as i16;
            self.pos += self.ratio;
            written += 1;
        }
        // Carry the fraction into the next chunk. Resetting to zero would put a
        // fractional jump at every buffer boundary and slowly drift the rate.
        self.pos -= input.len() as f64;
        if self.pos < 0.0 {
            self.pos = 0.0;
        }
        self.last = input[last];
        written
    }
}

/// Map a rate to Opus's enum. Anything outside its five rates is a bug in
/// `pick`, not something a device can cause.
fn opus_rate(hz: u32) -> Result<SampleRate> {
    Ok(match hz {
        48_000 => SampleRate::Hz48000,
        24_000 => SampleRate::Hz24000,
        16_000 => SampleRate::Hz16000,
        12_000 => SampleRate::Hz12000,
        8_000 => SampleRate::Hz8000,
        other => return Err(anyhow!("{other} Hz is not an Opus rate")),
    })
}

/// cpal 0.18 replaced `Device::name()` with a whole `DeviceDescription`.
fn device_name(device: &Device) -> String {
    device
        .description()
        .map(|d| d.name().to_string())
        .unwrap_or_else(|_| "<unknown>".to_string())
}

/// The best Opus-compatible config the device offers, highest rate first and
/// fewest channels within a rate. Reports what the device *does* offer when
/// nothing matches, because a device that cannot be used is worth naming — a
/// wrong-rate stream would otherwise run happily and just sound wrong.
fn pick(device: &Device, input: bool) -> Result<SupportedStreamConfig> {
    let which = if input { "input" } else { "output" };

    // The device's *default* config is its shared-mode mix format, and WASAPI
    // shared mode will accept nothing else — not another rate and not another
    // channel count. Asking for mono on a stereo device fails with "stream
    // configuration is not supported in shared mode", which reads like a codec
    // problem and is really a request Windows was never going to grant.
    //
    // So this is tried first, and the range search below is only a fallback for
    // hosts that do not work this way.
    let default = if input {
        device.default_input_config()
    } else {
        device.default_output_config()
    };
    if let Ok(cfg) = default {
        if OPUS_RATES.contains(&cfg.sample_rate()) && USABLE_FORMATS.contains(&cfg.sample_format())
        {
            return Ok(cfg);
        }
        eprintln!(
            "[audio] {which} default is {}Hz {:?} {}ch, which Opus cannot take directly — looking for another",
            cfg.sample_rate(),
            cfg.sample_format(),
            cfg.channels()
        );
    }

    let ranges: Vec<_> = if input {
        device.supported_input_configs()?.collect()
    } else {
        device.supported_output_configs()?.collect()
    };

    // Rate first, then format, then fewest channels. Format has to be part of
    // the choice: devices list several, and picking one the callbacks do not
    // implement gets rejected by the stream builder rather than by pick().
    for &hz in &OPUS_RATES {
        for &fmt in &USABLE_FORMATS {
            if let Some(r) = ranges
                .iter()
                .filter(|r| {
                    r.sample_format() == fmt
                        && r.min_sample_rate() <= hz
                        && hz <= r.max_sample_rate()
                })
                .min_by_key(|r| r.channels())
            {
                return Ok(r.clone().with_sample_rate(hz));
            }
        }
    }

    // Nothing at an Opus rate. Take the device's own default and resample
    // around it rather than refusing the device — this is the 44.1 kHz case,
    // and returning an error here meant a PC with a 44.1 kHz microphone could
    // not talk at all, reported once at startup and silent forever after.
    let fallback = if input {
        device.default_input_config().ok()
    } else {
        device.default_output_config().ok()
    };
    if let Some(cfg) = fallback {
        if USABLE_FORMATS.contains(&cfg.sample_format()) {
            eprintln!(
                "[audio] {which} runs at {}Hz, resampling to {}Hz for Opus",
                cfg.sample_rate(),
                nearest_opus(cfg.sample_rate())
            );
            return Ok(cfg);
        }
    }

    let offered: Vec<String> = ranges
        .iter()
        .map(|r| {
            format!(
                "{}ch {}-{}Hz {:?}",
                r.channels(),
                r.min_sample_rate(),
                r.max_sample_rate(),
                r.sample_format()
            )
        })
        .collect();
    Err(anyhow!(
        "{which} device offers no usable config (needs F32 or I16); it offers: {}",
        offered.join(", ")
    ))
}
