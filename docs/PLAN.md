# Plan — LAN Intercom v2

A LAN intercom: hold a key, everyone on the network hears you. No internet, no
accounts, no server. This is the Tauri + Rust rewrite of a working batch +
PowerShell version.

Updated 2026-10-07 (app at 1.5; step lists for what landed since 1.4.0 are in
[IMPROVEMENTS.md](IMPROVEMENTS.md)).

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

**Headphones switch: two-way talk is opt-in, half-duplex stays the default.**
Added since 1.4.0. A setting keeps playback on while the talk key is held. It is
live (an atomic read in the playback callbacks, no audio restart), saved per
machine, and **off** by default, because the mute is the echo fix and with
speakers and the switch on everyone hears themselves back. ⛔ Two-way needs
**both** people to turn it on: a person on speakers still mutes their own
playback while talking, so they cannot hear the other side answer. The setting
text says so. While muted, the playback buffer is now emptied rather than left
to fill, so nothing stale plays on release. Steps in
[IMPROVEMENTS.md](IMPROVEMENTS.md) Phase B. `71dfa58`.

**F7 replies to whoever spoke last.** Held like talk: it aims at the machine
whose voice arrived most recently, opens the mic, and leaves the target there so
a typed answer goes to the same person. Voice only; a text does not change who
"spoke last". Nobody heard yet, or that machine has gone offline: the mic stays
closed, never everyone. The lookup reuses the per-machine last-voice time the
roster already kept. `0de5c6f`.

**Sounds are pushed from Rust, not found by polling.** The roster is polled
every 400 ms, which is right for a list and wrong for a sound: a beep for
"someone started talking" would land mid-sentence. The net thread (which has no
window handle) calls into `notify.rs`, which emits `voice-start` (a machine's
audio begins after 400 ms of silence, the same threshold that ends "talking" in
the roster) and `talk-key` (own mic opens or closes, once per change, not per
key repeat). The window plays the sounds through the system default output, as
the message chime always did. A beep cannot come *before* the voice without
delaying every voice, so it plays as the voice starts; that trade was rejected.
The offline/online tone for a watched PC rides the existing roster poll, so it
is about seven seconds late by design: it must not fire on one lost packet. Each
sound has its own switch and Test button, with one volume; the own-key tick is
off by default because on speakers the room hears it. `07db99c`.

**Receipts ride the header's `ts` field, and `VER` was not bumped.** A text
used to send `ts: 0`, a field text never used. It now carries a random message
id, and a receiver answers with a header-only `KIND_ACK` (4) whose `ts` is that
id, every time, so a lost answer gets another chance. The sender's line shows
`✓` or `✓ 3/6`, names on hover, and "nobody online" in red only when it went to
nobody. No answer is never red: a build from before receipts never answers, and
no answer is not the same as not delivered. It means *received by the app*, not
*read*. ⛔ Bumping `VER` was rejected: `Header::parse` drops every packet whose
version differs, so a bump splits the office in two until every machine
updates, with nothing on screen saying why. A new *kind* is skipped harmlessly
by an old build, as `KIND_VERSION` already was. `11fb8cf`.

**Replay guard.** The room seal proves a packet came from inside the room, not
that it is new, so a recorded text sent again appeared again. The same message
id closes this: the last 200 (sender, id) pairs are remembered and a repeat is
not shown (it is still answered, so the sender's receipt completes). Id 0, sent
by older builds, is never dropped, so they keep working. `11fb8cf`.

**Saved groups.** Decided 2026-10-07: groups are **saved named lists**, not
members picked on the spot. The target is `All | One | Group`; members are
stored by address like labels, so a renamed PC stays in. A send goes to the
group members that are live. ⛔ Offline members are left out and logged, and if
none are live it goes to **nobody, never to everyone**, the same rule as one PC.
Deleting a group that is aimed falls back to Everyone as the selection, which
is a choice of aim, not a send. No global key per group by default: every
global key is taken from every other program on the machine. `ba09186`.

**Address one machine or everyone.** ~~No per-person targeting~~ — reversed on
2026-08-09, the same day it was settled twice the other way, once the actual
purpose became clear: this is for **directing devs and staff**, not for chatting
with peers. Telling one person something is the main use, not a variation on it.

Being wrong twice about this was cheap, and only because of the fan-out
decision below: every send was already a unicast to each recipient in turn, so
addressing one person is a shorter list rather than a different protocol. No
change to the wire format, no field in the header. Had this been built on
broadcast, the same reversal would have meant rewriting the transport.

The target lives in Rust rather than in the window, because a global shortcut
fires with the window closed and still has to know who it is talking to. A
target that has gone offline sends to nobody rather than falling back to
everyone: quietly redirecting an instruction meant for one person to the whole
room is worse than not sending it.

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

Eight bytes, big-endian, in front of every payload. `VER` is currently `2`
(raised when the room passphrase began sealing every packet). Message types
added since have been new *kinds*, not new versions.

```
 0        1        2                 4                              8
 ┌────────┬────────┬─────────────────┬──────────────────────────────┐
 │  ver   │  kind  │       seq       │          timestamp           │
 │  u8=2  │  u8    │       u16       │             u32              │
 └────────┴────────┴─────────────────┴──────────────────────────────┘
```

- `ver` — one byte that stops a future build's packets being decoded as noise.
- `kind` — `0` audio, `1` text, `2` heartbeat, `3` version (which build this
  machine runs, sent about every 30 s), `4` ack (a text arrived). An unknown
  kind is skipped by older builds, which is why a new feature adds a kind and
  does not bump `ver`.
- `seq` — wrapping, +1 per packet. **Compare with `wrapping_sub`.** It wraps
  after about 22 minutes of continuous talking, so a plain `>` comparison stalls
  the receiver permanently in a real conversation and never in a test.
- `timestamp` — sample index. Advances by the *sender's* frame size, which is
  `rate / 50` and may be 320 rather than 960. The receiver must never assume it. **For text (`kind` 1) it is not a clock:
  it is the message id**, random per message (0 from older builds), and an ack
  (`kind` 4) echoes it back. The same id feeds the replay guard.

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

> ### ▶ Next: **6 — packaging**
>
> Steps 0 to 5 are complete. What remains is shipping: MSI and the other
> bundles, an all-profile firewall rule created by the installer rather than by
> a prompt somebody might cancel (#11), recovering when the audio device changes
> or disappears (#9), gating the once-a-second stats line, and a LICENSE and
> README for a repo that is already public (#16).
>
> Also outstanding, and worth doing before anyone else sees it: the design pass
> over roster, talk state and messages, now that all three exist to be designed
> together.

| Step | | |
|---|---|---|
| 0 | Skeleton | ✅ done |
| 1 | Audio loopback in-process | ✅ done |
| 2 | Transport | ✅ done |
| 3 | Discovery and presence | ✅ done |
| 4 | Push-to-talk | ✅ done |
| 5 | Text and notifications | ✅ done |
| 6 | Packaging | ☐ **next** |

### 0. Skeleton ✅

Tauri v2, React + TypeScript, tray icon, close-to-hide. Quit is a tray menu
item, because once closing hides the window it is the only way out.

### 1. Audio loopback in-process ✅

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

### 2. Transport ✅

- [x] **2a** Two windows, crossed ports, counters climbing.
- [x] **2b** Opus frames as the payload. Loopback cut; playback fed only by what
      arrives. **Verified between two PCs.**
- [x] **2c** Jitter buffer: holds 3 packets, decodes in `seq` order.
- [x] **2d** Packet-loss concealment: `decode(None, …)` on a gap.

Also landed here, unplanned: the `session` module owning audio and net; capture
and playback made independently optional; and the transport auto-starting from
`transport.json` so a listening PC needs nobody to click anything.

### 3. Discovery and presence ✅

- [x] **3a** `mdns-sd` advertise and browse; peers appear and vanish in the log.
- [x] **3b** Roster in the UI, with the address behind Advanced.
- [x] **3c** Heartbeat on `kind=2` every two seconds, and a seven-second
      timeout. `live` now means "heard from recently" rather than "mDNS
      mentioned it once" — mDNS answers who exists, the socket answers who is
      present.
- [x] **3d** Fan-out to every live peer; per-peer decoder, per-peer reorder
      window, mixer summing in i32. Heartbeats go to everyone *known* and audio
      to everyone *live* — if both went to the live set, two instances that had
      never heard each other would deadlock waiting for the other to speak. The
      manual address is now optional rather than required.
- [x] Peers added manually, merged with discovered ones (#12).

*Done when:* a second instance appears within a couple of seconds and greys out
within the timeout when killed.

This is the thing the old version could never do, and it removes most of the
"is it even working" debugging.

### 4. Push-to-talk ✅

- [x] **4a** F8 held is the only thing that sends. Registered globally in Rust
      so it works unfocused; a failed registration is reported rather than
      discarded, because a shortcut that loses a race to another application
      does nothing and says nothing.
- [x] **4b** The UI shows who is talking, with **no flag in the header**. Audio
      is only sent while a key is held, so audio arriving *is* the fact that
      somebody is speaking — and a flag could have disagreed with whether
      packets were actually turning up.
- [x] **4c** Local playback silenced while transmitting. **This is the whole
      echo strategy.** Capture keeps draining its ring while the key is up, or
      the first thing anyone heard on pressing it would be stale room noise.
- [x] The full shortcut set (#13). It began without per-person keys; the
      numbered keys that talk to or message roster entry N came with the
      reversal of "everyone only" below.

*Done when:* hold to talk, release to listen, no feedback with speakers on.

### 5. Text and notifications ✅

- [x] **5a** `kind=1` UTF-8 messages on the same socket. Text does not ride the
      audio sequence — it is not a stream, and advancing that counter would
      punch holes in the reorder window at the far end.
- [x] **5b** Native notifications, only for messages that arrived while the
      window was hidden — this app lives in the tray, so a toast for a line
      already on screen is noise.
- [x] **5c** Message log. Sent lines are logged whether or not anybody was live,
      so an empty roster cannot be mistaken for a broken keyboard.
- [x] Voice and text kept separate as actions and code paths (#14).
- [x] Preset messages, one shortcut each (#15).

Then the design pass, once roster, talk state and messages all exist to be
designed together rather than four times over.

### 6. Packaging

MSI, deb, AppImage, dmg. The app is now a **single named executable**, which is
the whole point: one firewall identity, one prompt, one rule.

- [ ] macOS: microphone entitlement and notarization, or Gatekeeper blocks it.
- [ ] Linux: PipeWire and PulseAudio through `cpal` — test both.
- [ ] All-profile firewall rule created by the installer (#11).
- [ ] Recover when the audio device changes or disappears (#9).
- [ ] Gate the once-a-second audio stats line behind a debug flag.
- [ ] LICENSE and a real README (#16).

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

## Required, added 2026-08-09

Four things asked for after the plan was first written. They land inside the
existing steps rather than after them.

**Add PCs manually** (step 3). The roster must not depend on mDNS alone: it is
dropped or filtered on plenty of networks, a PC on another subnet is never
discovered, and v1 already kept a hand-maintained roster. The roster becomes the
union of discovered and manually added peers, with the source visible, and
manual entries persist. An address typed manually is also the fix for the v1 case
of a PC whose name would not resolve on the LAN at all.

**A complete shortcut set** (step 4), not one push-to-talk key — and,
since the 2026-08-09 reversal above, **numbered keys for talking to or messaging
one roster entry**, plus later F7 for reply. There is no key per group by
default: every global key is taken from every other program on the machine.
Keys cover talking, sending
each preset message, showing and hiding the window, and muting. They must be
reassignable, persisted, and must report failure to register: a global hotkey
silently losing a registration race to another application is a v1-class silent
failure.

**Voice and text separate** (step 5). In v1 one keypress both pinged someone and
opened the mic to them, and that coupling is exactly what made the failure
unreadable: the ping arrived and the voice did not, because the two halves
resolved the name differently. They stay separate as actions and as code paths,
while still sharing one socket and one header — a port per feature is what let
the halves arrive independently in the first place.

**Auto message** (step 5) means **preset messages on a key**: an editable list,
each entry sendable with one shortcut and no typing. Not auto-replies and not
scheduled announcements; both were considered and neither is wanted.

**Nothing is shipped in that list.** It arrived with "on my way", "lunch?" and
"call me", and those were the wrong register for a tool used to direct staff —
inventing pleasantries on somebody's behalf was presumptuous. The mechanism
stays for anyone who wants a key that fires a sentence they actually use.

**Adding a PC means adding someone who can be addressed**, individually, in a
saved group, or as part of the room. That is the reversal above: the roster is the choice of who is
being spoken to, not merely a list of who exists.

---

## Deferred, not forgotten

All of these are boxes in the step lists above, so they get ticked rather than
remembered. The reasoning that is not obvious from the one-liners:

- **Recover when the audio device changes or disappears** (step 6). `cpal` binds
  a stream to a device and does not follow the Windows default, so swapping
  headsets mid-run leaves the app holding a corpse and failing silently. People
  swap headsets mid-conversation and Bluetooth devices drop on their own.
- **Gate the once-a-second audio stats line** (step 6). It earns its place while
  the pipeline is being tuned and is noise afterwards.
- **`tauri-specta`**, to generate the TypeScript types from the Rust structs.
  The roster type crosses the IPC boundary manually today, in `types.d.ts`, and
  a hand-written twin is a thing that drifts.
