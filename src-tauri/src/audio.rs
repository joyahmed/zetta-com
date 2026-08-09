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

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender};
use std::sync::Arc;
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
}

/// Counters, so a glitch says which link produced it instead of being guessed
/// at by ear. Atomic increments are wait-free on x86, which is why they are
/// tolerable inside the real-time callbacks when a lock or a log would not be.
#[derive(Default)]
struct Stats {
    /// Samples the capture callback threw away because ring A was full: the
    /// encoder is not keeping up.
    in_drops: AtomicU64,
    /// Frames encoded.
    frames: AtomicU64,
    /// Output callbacks that produced silence because ring B was short.
    underruns: AtomicU64,
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
    /// `None` means a frame the transport knows is missing. It is not the same
    /// as no frame at all: the decoder conceals a gap it is told about, and a
    /// concealed 20 ms is far less audible than a skipped one.
    pub frames_in: SyncSender<Option<Vec<u8>>>,
    pub handle: Handle,
}

pub fn start() -> Result<Pipeline> {
    let stop = Arc::new(AtomicBool::new(false));
    let (ready_tx, ready_rx) = mpsc::channel::<Result<()>>();

    // Bounded on purpose. If the network side stalls, dropping speech is the
    // correct response and an unbounded queue is not: audio that arrives late
    // enough is worth less than the memory holding it.
    let (out_tx, out_rx) = mpsc::sync_channel::<Frame>(8);
    let (in_tx, in_rx) = mpsc::sync_channel::<Option<Vec<u8>>>(16);

    let thread_stop = stop.clone();
    thread::Builder::new()
        .name("audio".into())
        .spawn(move || match build(&thread_stop, out_tx, in_rx) {
            Ok(streams) => {
                let _ = ready_tx.send(Ok(()));
                while !thread_stop.load(Ordering::Relaxed) {
                    thread::sleep(Duration::from_millis(50));
                }
                drop(streams);
            }
            Err(e) => {
                let _ = ready_tx.send(Err(e));
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

fn build(
    stop: &Arc<AtomicBool>,
    out_tx: SyncSender<Frame>,
    in_rx: Receiver<Option<Vec<u8>>>,
) -> Result<Streams> {
    let host = cpal::default_host();

    // Capture and playback are independently optional. Most people on an
    // intercom never talk, so a PC with no microphone — or one whose headset
    // has not connected, or whose mic is off in privacy settings — must still
    // hear everyone. Losing playback because there is no capture device is
    // exactly backwards.
    let in_sel = select(&host, true);
    let out_sel = select(&host, false);

    if in_sel.is_none() && out_sel.is_none() {
        return Err(anyhow!(
            "no usable audio device for either capture or playback"
        ));
    }

    let stats = Arc::new(Stats::default());

    let input_stream = match in_sel {
        Some((device, cfg)) => Some(build_capture(&device, &cfg, stop, &stats, out_tx)?),
        None => {
            eprintln!("[audio] no usable input device — listening only");
            drop(out_tx);
            None
        }
    };

    let output_stream = match out_sel {
        Some((device, cfg)) => Some(build_playback(&device, &cfg, stop, &stats, in_rx)?),
        None => {
            eprintln!("[audio] no usable output device — sending only");
            drop(in_rx);
            None
        }
    };

    // One line a second. Cheap enough to leave in while the pipeline is being
    // tuned, and it turns "it sounds wrong" into a number that names the link.
    let report_stop = stop.clone();
    thread::Builder::new()
        .name("audio-stats".into())
        .spawn(move || {
            let (mut p_drops, mut p_frames, mut p_under, mut p_primes) = (0u64, 0u64, 0u64, 0u64);
            while !report_stop.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_secs(1));
                let drops = stats.in_drops.load(Ordering::Relaxed);
                let frames = stats.frames.load(Ordering::Relaxed);
                let under = stats.underruns.load(Ordering::Relaxed);
                let primes = stats.primes.load(Ordering::Relaxed);
                eprintln!(
                    "[audio] 1s frames {} drops {} underruns {} reprimes {} ringA {} ringB {}",
                    frames - p_frames,
                    drops - p_drops,
                    under - p_under,
                    primes - p_primes,
                    stats.a_occ.load(Ordering::Relaxed),
                    stats.b_occ.load(Ordering::Relaxed),
                );
                (p_drops, p_frames, p_under, p_primes) = (drops, frames, under, primes);
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
    })
}

// A plain fn, not a closure, so it can be used in more than one match arm.
// cpal 0.18 collapsed StreamError and friends into one `cpal::Error`.
fn on_err(e: cpal::Error) {
    eprintln!("[audio] stream error: {e}");
}

/// Microphone → ring A → Opus → `out_tx`. Owns ring A entirely, so nothing
/// about it exists when there is no input device.
fn build_capture(
    device: &Device,
    cfg: &SupportedStreamConfig,
    stop: &Arc<AtomicBool>,
    stats: &Arc<Stats>,
    out_tx: SyncSender<Frame>,
) -> Result<cpal::Stream> {
    let hz = cfg.sample_rate();
    let frame = (hz / 50) as usize; // 20 ms
    let rate = opus_rate(hz)?;
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
            on_err,
            None,
        )?,
        SampleFormat::I16 => device.build_input_stream(
            stream_cfg.clone(),
            move |data: &[i16], _: &cpal::InputCallbackInfo| {
                for f in data.chunks_exact(ch) {
                    let sum: i32 = f.iter().map(|s| *s as i32).sum();
                    if prod.try_push((sum / ch as i32) as i16).is_err() {
                        st.in_drops.fetch_add(1, Ordering::Relaxed);
                    }
                }
            },
            on_err,
            None,
        )?,
        f => return Err(anyhow!("unsupported input sample format {f:?}")),
    };

    let enc_stop = stop.clone();
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
        let mut packet = [0u8; MAX_PACKET];

        while !enc_stop.load(Ordering::Relaxed) {
            let have = cons.occupied_len();
            enc_stats.a_occ.store(have as u64, Ordering::Relaxed);
            if have < frame {
                thread::sleep(Duration::from_millis(2));
                continue;
            }
            cons.pop_slice(&mut pcm[..frame]);

            let n = match enc.encode(&pcm[..frame], &mut packet) {
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
            let _ = out_tx.try_send(Frame {
                data: packet[..n].to_vec(),
                samples: frame as u32,
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
    stats: &Arc<Stats>,
    in_rx: Receiver<Option<Vec<u8>>>,
) -> Result<cpal::Stream> {
    let hz = cfg.sample_rate();
    let frame = (hz / 50) as usize;
    let rate = opus_rate(hz)?;
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
            device.build_output_stream(
                stream_cfg.clone(),
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
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
                on_err,
                None,
            )?
        }
        SampleFormat::I16 => {
            let mut primed = false;
            let st = stats.clone();
            device.build_output_stream(
                stream_cfg.clone(),
                move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
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
                on_err,
                None,
            )?
        }
        f => return Err(anyhow!("unsupported output sample format {f:?}")),
    };

    let dec_stop = stop.clone();
    thread::Builder::new().name("decoder".into()).spawn(move || {
        let mut dec = match Decoder::new(rate, Channels::Mono) {
            Ok(d) => d,
            Err(e) => return eprintln!("[audio] decoder: {e}"),
        };
        let mut out = [0i16; MAX_FRAME];

        while !dec_stop.load(Ordering::Relaxed) {
            // Timed out rather than blocking, so the thread notices the stop
            // flag instead of parking forever on a silent peer.
            let packet = match in_rx.recv_timeout(Duration::from_millis(200)) {
                Ok(p) => p,
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => break,
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
                Some(p) => dec.decode(Some(&p[..]), &mut out[..frame], false),
                None => dec.decode(None::<&[u8]>, &mut out[..frame], false),
            };
            let got = match decoded {
                Ok(n) => n,
                Err(e) => {
                    eprintln!("[audio] decode: {e}");
                    continue;
                }
            };
            prod.push_slice(&out[..got]);
        }
    })?;

    Ok(stream)
}

/// The default device for a direction and a config we can actually use, or
/// `None` with the reason logged. Returning `None` rather than an error is the
/// point: one missing direction must not take the other down with it.
fn select(host: &Host, input: bool) -> Option<(Device, SupportedStreamConfig)> {
    let which = if input { "input" } else { "output" };
    let device = if input {
        host.default_input_device()
    } else {
        host.default_output_device()
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
        "{} device offers no usable config (needs F32 or I16 at 48/24/16/12/8 kHz); it offers: {}",
        if input { "input" } else { "output" },
        offered.join(", ")
    ))
}
