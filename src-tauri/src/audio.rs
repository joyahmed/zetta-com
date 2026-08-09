//! In-process audio loopback: mic → Opus encode → Opus decode → speakers.
//!
//! Thread layout — this is the design:
//!
//!   [input callback]   real-time   mic → mono i16 → ring A
//!   [codec worker]     normal      ring A → 20 ms frame → encode → decode → ring B
//!   [output callback]  real-time   ring B → speakers, silence on underrun
//!
//! The two callbacks run on real-time threads owned by the audio driver. No
//! allocation, no locks, no logging inside them — `println!` takes the stdout
//! lock and is audible as a crackle.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use audiopus::coder::{Decoder, Encoder};
use audiopus::{Application, Bitrate, Channels, SampleRate};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, SupportedStreamConfig};
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

/// Dropping this stops the loopback: the audio thread sees the flag, returns,
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
/// `cpal::Stream` is not `Send` on every backend.
struct Streams {
    _input: cpal::Stream,
    _output: cpal::Stream,
}

/// Counters, so a glitch says which link produced it instead of being guessed
/// at by ear. Atomic increments are wait-free on x86, which is why they are
/// tolerable inside the real-time callbacks when a lock or a log would not be.
#[derive(Default)]
struct Stats {
    /// Samples the capture callback threw away because ring A was full: the
    /// codec worker is not keeping up.
    in_drops: AtomicU64,
    /// Frames encoded and decoded.
    frames: AtomicU64,
    /// Output callbacks that produced silence because ring B was short: the
    /// pipeline is not keeping up with the sound card.
    underruns: AtomicU64,
    /// Times playback had to re-bank after running dry. A steady count here is
    /// the signature of a periodic stutter.
    primes: AtomicU64,
    /// Latest ring occupancies, published for the reporter thread.
    a_occ: AtomicU64,
    b_occ: AtomicU64,
}

pub fn start() -> Result<Handle> {
    let stop = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel::<Result<()>>();

    let thread_stop = stop.clone();
    thread::Builder::new()
        .name("audio".into())
        .spawn(move || match build(&thread_stop) {
            Ok(streams) => {
                let _ = tx.send(Ok(()));
                while !thread_stop.load(Ordering::Relaxed) {
                    thread::sleep(Duration::from_millis(50));
                }
                drop(streams);
            }
            Err(e) => {
                let _ = tx.send(Err(e));
            }
        })?;

    // Two `?`: one for the channel dying, one for the build failing.
    rx.recv().context("audio thread died before reporting")??;
    Ok(Handle { stop })
}

fn build(stop: &Arc<AtomicBool>) -> Result<Streams> {
    let host = cpal::default_host();
    let input = host
        .default_input_device()
        .ok_or_else(|| anyhow!("no default input device"))?;
    let output = host
        .default_output_device()
        .ok_or_else(|| anyhow!("no default output device"))?;

    let in_cfg = pick(&input, true)?;
    let out_cfg = pick(&output, false)?;

    // Opus can encode at one rate and decode at another, so a 16 kHz headset
    // mic feeding 48 kHz speakers needs no resampler anywhere in this file.
    let in_hz = in_cfg.sample_rate();
    let out_hz = out_cfg.sample_rate();
    let in_frame = (in_hz / 50) as usize; // 20 ms
    let out_frame = (out_hz / 50) as usize;
    let enc_rate = opus_rate(in_hz)?;
    let dec_rate = opus_rate(out_hz)?;

    eprintln!(
        "[audio] in  {:?} {}Hz {:?} {}ch frame {}",
        device_name(&input),
        in_hz,
        in_cfg.sample_format(),
        in_cfg.channels(),
        in_frame
    );
    eprintln!(
        "[audio] out {:?} {}Hz {:?} {}ch frame {}",
        device_name(&output),
        out_hz,
        out_cfg.sample_format(),
        out_cfg.channels(),
        out_frame
    );

    // Sized in frames, not seconds. A ring's capacity is a latency ceiling: if
    // it can hold a second of audio then one transient stall fills it with a
    // second of audio and it never gives that back, so you hear yourself late
    // for the rest of the session. At RING_FRAMES the same stall costs 160 ms
    // and is paid back by dropping, which is the right trade for speech.
    let (mut prod_a, mut cons_a) = HeapRb::<i16>::new(in_frame * RING_FRAMES).split();
    let (mut prod_b, mut cons_b) = HeapRb::<i16>::new(out_frame * RING_FRAMES).split();

    let in_ch = in_cfg.channels() as usize;
    let out_ch = out_cfg.channels() as usize;
    let in_stream_cfg = in_cfg.config();
    let out_stream_cfg = out_cfg.config();

    // A plain fn, not a closure, so it can be used in more than one match arm.
    // cpal 0.18 collapsed StreamError and friends into one `cpal::Error`.
    fn on_err(e: cpal::Error) {
        eprintln!("[audio] stream error: {e}");
    }

    let stats = Arc::new(Stats::default());

    let st = stats.clone();
    let input_stream = match in_cfg.sample_format() {
        SampleFormat::F32 => input.build_input_stream(
            in_stream_cfg.clone(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                for frame in data.chunks_exact(in_ch) {
                    let sum: f32 = frame.iter().sum();
                    let v = (sum / in_ch as f32).clamp(-1.0, 1.0);
                    // Full ring drops the sample. Correct behaviour here:
                    // never block a real-time callback.
                    if prod_a.try_push((v * i16::MAX as f32) as i16).is_err() {
                        st.in_drops.fetch_add(1, Ordering::Relaxed);
                    }
                }
            },
            on_err,
            None,
        )?,
        SampleFormat::I16 => input.build_input_stream(
            in_stream_cfg.clone(),
            move |data: &[i16], _: &cpal::InputCallbackInfo| {
                for frame in data.chunks_exact(in_ch) {
                    let sum: i32 = frame.iter().map(|s| *s as i32).sum();
                    if prod_a.try_push((sum / in_ch as i32) as i16).is_err() {
                        st.in_drops.fetch_add(1, Ordering::Relaxed);
                    }
                }
            },
            on_err,
            None,
        )?,
        f => return Err(anyhow!("unsupported input sample format {f:?}")),
    };

    // Playback priming. The decoder delivers a whole frame at once every 20 ms
    // while this callback asks for samples on its own schedule, so the ring is
    // routinely a few samples short of what is being asked for. Filling that
    // shortfall with zeros puts a sub-millisecond gap inside a frame, and a gap
    // that often is heard as a screech, not as silence. So: stay quiet until a
    // few frames have banked, then drain whole callbacks only; if it ever runs
    // dry, go quiet and bank again. Silence in clean chunks is inaudible.
    let prime = out_frame * 3;

    let output_stream = match out_cfg.sample_format() {
        SampleFormat::F32 => {
            // Captured by the FnMut closure. Only this callback touches it, so
            // it needs no atomic and no lock — which matters on an RT thread.
            let mut primed = false;
            let st = stats.clone();
            output.build_output_stream(
                out_stream_cfg.clone(),
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let have = cons_b.occupied_len();
                    st.b_occ.store(have as u64, Ordering::Relaxed);
                    let need = data.len() / out_ch;
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
                    for frame in data.chunks_exact_mut(out_ch) {
                        let v = cons_b.try_pop().unwrap_or(0) as f32 / i16::MAX as f32;
                        frame.fill(v);
                    }
                },
                on_err,
                None,
            )?
        }
        SampleFormat::I16 => {
            let mut primed = false;
            let st = stats.clone();
            output.build_output_stream(
                out_stream_cfg.clone(),
                move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                    let have = cons_b.occupied_len();
                    st.b_occ.store(have as u64, Ordering::Relaxed);
                    let need = data.len() / out_ch;
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
                    for frame in data.chunks_exact_mut(out_ch) {
                        frame.fill(cons_b.try_pop().unwrap_or(0));
                    }
                },
                on_err,
                None,
            )?
        }
        f => return Err(anyhow!("unsupported output sample format {f:?}")),
    };

    let codec_stop = stop.clone();
    let codec_stats = stats.clone();
    thread::Builder::new().name("codec".into()).spawn(move || {
        let mut enc = match Encoder::new(enc_rate, Channels::Mono, Application::Voip) {
            Ok(e) => e,
            Err(e) => return eprintln!("[audio] encoder: {e}"),
        };
        if let Err(e) = enc.set_bitrate(Bitrate::BitsPerSecond(BITRATE)) {
            eprintln!("[audio] set_bitrate: {e}");
        }
        let mut dec = match Decoder::new(dec_rate, Channels::Mono) {
            Ok(d) => d,
            Err(e) => return eprintln!("[audio] decoder: {e}"),
        };

        // Sized for the worst case, sliced to the real frame. Allocated once,
        // outside the loop, so the loop itself never allocates.
        let mut pcm = [0i16; MAX_FRAME];
        let mut packet = [0u8; MAX_PACKET];
        let mut out = [0i16; MAX_FRAME];

        while !codec_stop.load(Ordering::Relaxed) {
            let have = cons_a.occupied_len();
            codec_stats.a_occ.store(have as u64, Ordering::Relaxed);
            if have < in_frame {
                thread::sleep(Duration::from_millis(2));
                continue;
            }
            codec_stats.frames.fetch_add(1, Ordering::Relaxed);
            cons_a.pop_slice(&mut pcm[..in_frame]);

            let n = match enc.encode(&pcm[..in_frame], &mut packet) {
                Ok(n) => n,
                Err(e) => {
                    eprintln!("[audio] encode: {e}");
                    continue;
                }
            };
            // Decoding to `out_frame` rather than `in_frame` is what converts
            // the rate: libopus resamples internally, so nothing here does.
            let got = match dec.decode(Some(&packet[..n]), &mut out[..out_frame], false) {
                Ok(n) => n,
                Err(e) => {
                    eprintln!("[audio] decode: {e}");
                    continue;
                }
            };
            prod_b.push_slice(&out[..got]);
        }
    })?;

    // One line a second. Cheap enough to leave in while the pipeline is being
    // tuned, and it turns "it sounds wrong" into a number that names the link.
    let report_stop = stop.clone();
    thread::Builder::new().name("audio-stats".into()).spawn(move || {
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

    input_stream.play()?;
    output_stream.play()?;

    Ok(Streams {
        _input: input_stream,
        _output: output_stream,
    })
}

/// cpal 0.18 replaced `Device::name()` with a whole `DeviceDescription`.
fn device_name(device: &Device) -> String {
    device
        .description()
        .map(|d| d.name().to_string())
        .unwrap_or_else(|_| "<unknown>".to_string())
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
    // the choice: devices list several, and picking one the callbacks below do
    // not implement gets rejected by the stream builder rather than by pick().
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
