# Plan — LAN Intercom v2

A LAN intercom: hold a key, everyone on the network hears you. No internet, no
accounts, no server. This is the Tauri + Rust rewrite of a working batch +
PowerShell version.

Updated 2026-08-09.

---

## Why a rewrite

The old version worked, and nearly every failure in it was a *substrate* bug
rather than an intercom bug: PowerShell 5.1-only `Add-Type`, UTF-8 BOM parsing,
CRLF, a script base64-embedded inside a `.bat`, `SW_HIDE` inheriting into
WinForms, an installer that swallowed exit codes, a 0-byte WinGet alias passing
`if exist`. A real application deletes that entire category.

The decisive one was the firewall. Inbound BLOCK rules killed a PC because the
network-facing program was `powershell.exe` — a shared system binary anything
can get muzzled, leaving no visible trace. A dedicated signed executable has its
own firewall identity, which is the single biggest reason this exists.

Two other things fall out for free: mDNS replaces Windows name resolution, which
broke two PCs; and a heartbeat makes presence possible, which the old version
could never report.

---

## Settled decisions

These are closed. Reopening one needs a reason that has changed, not a
preference.

**Half-duplex push-to-talk.** One mic open at a time. While transmitting, local
playback is muted — *that mute is the entire echo solution*, because the echo
path never exists, so there is nothing to cancel. Full duplex was rejected: it
needs acoustic echo cancellation via `webrtc-audio-processing` (C++ bindings,
build friction on all three platforms) plus mixing and per-speaker gain, and it
becomes the project rather than a step in it. If real full duplex is ever the
requirement, Mumble already solves it.

**Everyone, always.** Holding the key reaches every live peer. No per-person
targeting, and so no target field in the header. The old version had per-person
hotkeys and that is not how it actually got used.

**Unicast fan-out, not broadcast.** One encode, N sends. Broadcast looks cheaper
and is a trap: some WiFi access points rate-limit or drop broadcast frames, so
one person silently hears nothing while everyone else is fine. Fan-out costs a
few hundred bytes per frame per peer on a LAN, and it fails visibly.

**`cpal` + `audiopus`, not bundled ffmpeg.** About 10 MB against ~70 MB per
platform, and libopus is BSD so there is no GPL question.

**One socket, one header.** Audio, text and heartbeats are distinguished by a
`kind` byte, not by separate ports. The old version used a second port for text
and the two halves could arrive independently — one reached a PC while the other
did not, which took a day to work out.

**Any Opus rate the device offers, not just 48 kHz.** Opus encodes at
8/12/16/24/48 kHz natively, and its decoder can emit a rate different from the
one the encoder consumed, so no resampler is needed anywhere: encode at the
microphone's rate, decode straight to the speaker's. A Bluetooth headset in
Hands-Free mode offers 16 kHz and nothing else, and refusing to start on one is
not acceptable for an app whose whole purpose is other people's PCs.

**Per-peer decoders.** Opus decoder state is a running conversation with one
encoder. Each remote talker needs its own decoder *and* its own jitter buffer;
a mixer sums them into the playback ring. Feeding two peers through one decoder
produces garbage.

---

## Wire format

Eight bytes, big-endian, in front of every payload.

```
 0        1        2                 4                              8
 ┌────────┬────────┬─────────────────┬──────────────────────────────┐
 │  ver   │  kind  │       seq       │          timestamp           │
 │  u8=1  │  u8    │       u16       │             u32              │
 └────────┴────────┴─────────────────┴──────────────────────────────┘
```

- `ver` — one byte that stops a future build's packets being decoded as noise.
- `kind` — `0` audio, `1` text, `2` heartbeat.
- `seq` — wrapping, +1 per packet. **Compare with `wrapping_sub`.** It wraps
  after about 22 minutes of continuous talking, so a plain `>` comparison stalls
  the receiver permanently in a real conversation and never in a test.
- `timestamp` — sample index. Advances by the *sender's* frame size, which is
  `rate / 50` and may be 320 rather than 960. The receiver must never assume it.

One Opus frame per datagram, roughly 80–120 bytes, nowhere near MTU. The old
`pkt_size=1316` constraint was an MPEG-TS artifact and does not apply.

---

## Structure

```
audio  ── capture, encode, decode, playback. Knows nothing about sockets.
net    ── socket, header, peers. Knows nothing about codecs.
session ── owns both and wires them together.
```

The `session` layer exists so neither module imports the other. Without it
`audio` grows networking and `net` grows codecs, and by step 4 neither can be
tested alone.

```
capture → ring A → encoder ──► frames out ──► net tx ──► every live peer
playback ← ring B ← mixer ◄── per-peer decoders ◄── net rx
```

---

## Build order

Sequenced so the riskiest thing is proven first, and every slice runs and shows
something before the next one starts.

### 0. Skeleton — done

Tauri v2, React + TypeScript, tray icon, close-to-hide. Quit is a tray menu
item, because once closing hides the window it is the only way out.

### 1. Audio loopback in-process — done

`cpal` capture → Opus encode → Opus decode → `cpal` playback, one process, no
network. This was the step the whole rewrite was gated on, and it works.

Rings are sized in 20 ms frames, not seconds. A ring's capacity is a latency
ceiling: one that holds a second of audio will hold a second of audio after any
stall and never give it back, which is heard as echo and sends you looking at
the codec. Playback stays quiet until a few frames have banked and then drains
whole callbacks only — zeros sprayed *inside* a frame are heard as a screech,
where the same silence in clean chunks is inaudible.

The callbacks are real-time threads: no allocation, no locks, no logging. A
`println!` takes the stdout lock and is audible.

### 2. Transport — in progress

- **2a** Two windows, crossed ports, counters climbing. *(verifying)*
- **2b** Opus frames as the payload. Cut the loopback; playback is fed only by
  what arrives. **The milestone — you hear the other window.**
- **2c** Jitter buffer: hold 3–5 packets, decode in `seq` order.
- **2d** Packet-loss concealment: `decode(None, …)` on a gap. A skipped frame is
  a click; PLC is a smear you have to listen for. Test by dropping 1-in-20
  deliberately.

*Done when:* two instances on one PC, different ports, pass audio cleanly.

### 3. Discovery and presence

- **3a** `mdns-sd` advertise and browse; peers appear and vanish in the log.
- **3b** Roster in the UI. **The peer input dies here.**
- **3c** Heartbeat and timeout; live or greyed.
- **3d** Fan-out to every live peer; per-peer decoder, per-peer jitter buffer,
  mixer.

*Done when:* a second instance appears within a couple of seconds and greys out
within the timeout when killed.

This is the thing the old version could never do, and it removes most of the
"is it even working" debugging.

### 4. Push-to-talk

- **4a** Global shortcut; nothing is sent unless the key is held.
- **4b** Talking flag in the header; the UI shows who is talking.
- **4c** Mute local playback while transmitting.

*Done when:* hold to talk, release to listen, no feedback with speakers on.

### 5. Text and notifications

- **5a** `kind=1` UTF-8 messages on the same socket.
- **5b** Native notifications.
- **5c** Message log.

Then the design pass, once roster, talk state and messages all exist to be
designed together rather than four times over.

### 6. Packaging

MSI, deb, AppImage, dmg. The app is now a **single named executable**, which is
the whole point: one firewall identity, one prompt, one rule.

- macOS: microphone entitlement and notarization, or Gatekeeper blocks it.
- Linux: PipeWire and PulseAudio through `cpal` — test both.

---

## Carried over from v1 — still true, still bites

**Bind `0.0.0.0` explicitly.** An empty host binds IPv6-only on Windows and then
silently ignores every IPv4 datagram. Every listener in v1 played perfect
silence because of this.

**Resolve peers to IPv4 literals.** A Windows PC name resolves IPv6-link-local
first and anything taking the first address reaches nobody. This is the same bug
on the sending side, and it produced "the text arrived but the voice did not".

**Never elevate the thing that plays audio.** An elevated process lands in a
session with no audio device and plays silence, with no error.

**Bounds-check anything parsed off the socket.** It is the only place bytes from
outside the process are interpreted, and anyone on the LAN can send one byte.

**RDP redirects the remote PC's audio to the client**, so testing over Remote
Desktop shows the wrong result entirely.

**Change the hardware before believing an audio bug.** A rhythmic creak chased
through two code changes turned out to be one Bluetooth headset's link. Wired
hardware is the cheapest baseline — it shares neither the clock nor the
transport.

---

## Deferred, not forgotten

- **Recover when the audio device changes or disappears.** `cpal` binds a stream
  to a device and does not follow the Windows default, so swapping headsets
  mid-run leaves the app holding a corpse and failing silently. People swap
  headsets mid-conversation and Bluetooth devices drop on their own. Before
  packaging.
- **Gate the once-a-second audio stats line** behind a debug flag before
  shipping. It earns its place while the pipeline is being tuned.
- **`tauri-specta`** to generate the TypeScript types from the Rust structs,
  when the roster type first crosses the IPC boundary at step 3.
