# Improvements after 1.4.0

What to build next, most important first, with every line checked against the
code on 2026-10-07 (`main` at `546ec96`). The reasoning behind the app as it
stands lives in [PLAN.md](PLAN.md); this file is only what changes next.

A box is ticked when it is true in a **released build**, not when the code is
written.

---

## Baseline, measured 2026-10-07

| Gate | Result |
|---|---|
| `tsc --noEmit` | ✅ passes (after a broken local install was repaired — see the end) |
| `cargo test` | 0 tests exist. "ok" means nothing ran. |
| `cargo clippy` | 10 warnings, 0 errors |
| Release workflow | builds only. No test, typecheck or lint step. |

---

## Verdict

| # | Asked for | What it actually is | Phase |
|---|---|---|---|
| 1 | Tests + a gate before release | True: 0 tests across ~4,700 lines of Rust, and `release.yml` runs straight into `tauri-action`. The risky parts are all pure functions, so they test without a sound card. | A |
| 2a | Headphones switch for two-way talk | True, and small: the only thing stopping two-way is one early `return` in each playback callback. ⛔ But it only works when **both** people turn it on. | B |
| 2b | Reply to whoever just spoke | True. `net.rs` already records when each machine last sent voice; nothing reads "who was last". | B |
| 3 | Sound alerts | Partly built: one chime, only for text that lands while the window is hidden. ⚠️ A beep can not come *before* the voice without delaying every voice. | C |
| 4 | "Heard by" receipts | True, and needs no wire-format change: text already sends a header field that is always 0. ⛔ Bumping the version byte would be the wrong way to do it. | D |
| 5 | Groups | True. Targeting is one address or everyone. Needs one product decision first. | E |
| — | Do-not-disturb, per-peer volume, PLAN.md refresh, tauri-specta | Can wait. | F |

---

## Phase A — Tests and a gate before release

> **Status:** built — `d97e87c`, `eb8f976`. CI green. Not in a release yet.


Everything below lives in `net.rs`, `room.rs`, `audio.rs` and the workflows, and
changes no behaviour. It goes first because Phases B–E all edit `net.rs`, and
they should land on top of tests that would notice a break.

Tests go in `#[cfg(test)] mod tests` inside each file, so private types like
`Jitter` can be tested without being made public.

### A.1 — Wire header (`net.rs:93-120`)

- [ ] `write` then `parse` gives back the same `ver`, `kind`, `seq`, `ts`.
- [ ] A buffer shorter than 8 bytes returns `None`, including 0 and 1 byte. This is the one place outside bytes are read (`net.rs:101-104`).
- [ ] A packet with any version other than `VER` returns `None`.
- [ ] Bytes are big-endian: `seq = 0x0102` writes `01 02`.

### A.2 — Reorder window and the 22-minute wrap (`net.rs:544-615`, `net.rs:916-925`)

- [ ] In-order packets come out one for one.
- [ ] Two swapped packets come out in order.
- [ ] A missing packet comes out as `None` only once 3 more are banked behind it (`JITTER_TARGET`, `net.rs:529`).
- [ ] Across the wrap: `65534, 65535, 0, 1` come out in order, nothing reported missing. The code comment says this "arrives after about 22 minutes of talking … never in a test" — this makes it one.
- [ ] A packet older than what was already released is dropped.
- [ ] A jump of 32 or more starts the window over instead of emitting 32 gaps.
- [ ] The lost-packet count in the receive loop (`net.rs:916-925`) is inline inside the thread. Pull the gap sum out into a small `fn gap(want, seq) -> u16` and test it across the wrap. ⛔ Keep it `wrapping_sub` — a plain `>` stalls forever at the wrap.

### A.3 — Room encryption (`room.rs`)

- [ ] `seal` then `open` with the same passphrase gives back the payload.
- [ ] A different passphrase fails to open.
- [ ] One changed byte in the header fails to open (the header is signed but not hidden, `room.rs:131-137`).
- [ ] A body shorter than 12 bytes returns `None` without panicking.
- [ ] Two seals of the same payload never share a nonce.
- [ ] `code()` gives the same answer for the same passphrase, and `generate()` gives words from the list.
- [ ] `frame(None, …)` (`net.rs:30`) sends the payload unchanged when no room is set.

### A.4 — Audio maths (`audio.rs:1034-1120`)

- [ ] `nearest_opus` rounds down: 44,100 → 24,000 (on purpose, `audio.rs:1030-1032`), 48,000 and 16,000 stay, anything under 8,000 → 8,000.
- [ ] `Resampler::process` 48k → 44.1k returns about 0.919× the samples, and 1:1 returns the input.

### A.5 — Clean lint

- [ ] Fix the 10 clippy warnings: 5 needless `.clone()` on `Copy` configs, 2 `is_multiple_of`, 3 "too many arguments" (allow these three with a reason, do not bundle the arguments into a struct just to quiet it).

### A.6 — The gate

- [ ] In `release.yml`, before `tauri-action`, on every platform: `bunx tsc --noEmit`, `cargo test`, `cargo clippy -- -D warnings`. A red step stops that platform's upload.
- [ ] A new `check.yml` on every push to `main` and `joy`, Windows only, running the same three. ⚠️ Windows rather than Ubuntu: `cargo test` compiles all of Tauri, which on Linux needs the whole webkit/ALSA install from `release.yml`. Windows needs nothing extra.
- [ ] ⛔ Do not set `fail-fast: true` in the release matrix to "save minutes". It was set `false` on purpose (`release.yml:17`) so one platform's failure does not hide the others'.

---

## Phase B — Two-way talk and reply

> **Status:** built — `71dfa58`, `0de5c6f`. CI green. Not in a release yet; not yet heard on two machines.


Same files: `audio.rs` playback, `keys.rs`, `net.rs`, `config.rs`, Settings UI.

### B.1 — Headphones switch (`audio.rs:801-807`, `audio.rs:843-846`)

- [ ] ⛔ **It only works when both people have it on.** If I am on headphones and you are on speakers, your app still mutes your speakers while you talk, so you can not hear me answer. The setting text must say this, not just "two-way talk".
- [ ] A live `Arc<AtomicBool>` beside `transmit`, read in both playback callbacks. When it is on, skip the early `return`. Live, so flipping it does not restart the audio pipeline.
- [ ] Saved in `config.rs` (`#[serde(default)]`, off).
- [ ] Toggle in Settings, labelled "I'm on headphones — hear others while I talk".
- [ ] ⛔ Default stays **off**. The mute is the whole echo fix (PLAN.md, "Half-duplex"). With speakers and the switch on, everyone hears themselves back.
- [ ] ⚠️ Found while reading: while the mute is on, the callback returns without taking anything out of the playback buffer (8 frames, `audio.rs:47`, `audio.rs:779`). When you let go of the key you hear up to 160 ms of old audio. Drain and throw it away while muted. Same lines, same change.
- [ ] Fix the stale comment at `audio.rs:935-938` ("nothing enforces that until push-to-talk exists" — it exists).

### B.2 — Reply to whoever just spoke (`net.rs:221`, `keys.rs:18-52`, `keys.rs:72-96`)

- [ ] `net::Handle::last_speaker()` → the address with the newest time in `last_audio`.
- [ ] `Action::Reply`, held like Talk: aim at the last speaker, open the mic, leave the target there on release (same as `TalkTo`, `keys.rs:24-27`).
- [ ] Add `("reply", "Talk back to whoever spoke last", "F7")` to `EDITABLE`; the array goes from 6 to 7. F7 sits next to F8. The shortcut list in the UI reads from Rust (`useShortcuts.ts:15`), so it shows up with no UI change.
- [ ] Nobody has spoken yet → log it and do not open the mic, same as an empty slot (`keys.rs:338`).
- [ ] The last speaker went offline → nothing is sent, same rule as any other target (`net.rs:288`). Do not fall back to everyone.
- [ ] Decision taken unless overruled: voice only. A text message does not change who "spoke last".

---

## Phase C — Sound alerts

> **Status:** built — `07db99c`. CI green. Not in a release yet; tones not yet heard on real speakers.


Same files: `src/utils/chime.ts` (becomes a small set of sounds), `net.rs` receive loop, `keys.rs`, `config.rs`, Settings UI.

All sounds play through the webview like the chime does now — the system default
output, not the intercom's chosen device. That was chosen on purpose
(`chime.ts:13-16`) and stays.

### C.1 — Beep when someone starts talking to you (`net.rs:908-916`)

- [ ] ⚠️ It plays **as** the voice starts, not before. A true "before" means delaying every voice by the length of the beep. Rejected: latency on every sentence to save the first syllable.
- [ ] Detect the start in the receive loop: audio from an address whose last packet is older than 400 ms (`TALKING_TIMEOUT`, `net.rs:75`).
- [ ] The net thread has no `AppHandle`. Pass in a small callback or channel at `net::start` and emit a `voice-start` event to the window. ⛔ Do not detect it from the roster poll — that runs every 400 ms (`useTransport.ts:12`) and the beep would land mid-sentence.

### C.2 — Click on your own talk key (`keys.rs:298-340`)

- [ ] Emit `talk-key` (down/up) from `dispatch` for Talk, TalkTo, TalkAll, Reply. Not from the 100 ms `ptt_held` poll (`usePtt.ts:7`).
- [ ] ⚠️ On speakers the microphone is open the moment the key goes down, so a click on press is sent to the room. Keep it very short and quiet, and play the "up" sound on release only when the switch in C.4 says so.

### C.3 — Tone when a chosen person goes offline

- [ ] A "watch" mark per PC, saved in `config.rs` beside `labels` (keyed by address, the same as labels, `config.rs:73`).
- [ ] In `useTransport.ts`, which already polls `net_peers` with `live`, play a falling tone when a watched PC goes from live to not live, and a rising one when it comes back.
- [ ] ⚠️ "Offline" means no heartbeat for 7 seconds (`net.rs:71`). The tone is 7 s late by design; that is what stops it firing on one lost packet.

### C.4 — Sound settings

- [ ] One toggle per sound (message chime, voice start, own key, offline/online) and one volume, saved in `config.rs`.
- [ ] The existing message chime keeps its current behaviour by default (on, only while hidden).
- [ ] A "Test" button per sound, so the volume can be set without waiting for an event.

---

## Phase D — "Heard by" receipts for text

> **Status:** built — `11fb8cf`. Tested over real UDP on loopback. Not in a release yet.
> The UI says `✓` / `✓ 3/6`, not "heard by": shorter, so lines do not wrap.


Same files: `net.rs` (`send_text` `:402`, receive `:884`, `Message` `:150`), `Messages.tsx`, `types.d.ts`.

### D.1 — Wire

- [ ] Text already sends `ts: 0` (`net.rs:416`). Put a random message id in `ts` instead. Old builds ignore that field for text, so nothing breaks.
- [ ] New `KIND_ACK = 4`, header only, `ts` = the id being answered. Sent back to the sender on every text received.
- [ ] Old builds ignore a kind they do not know — the receive loop only handles kinds 0–3 (`net.rs:847-908`) and drops the rest.
- [ ] ⛔ **Do not bump `VER`.** `Header::parse` throws away every packet whose version differs (`net.rs:116`). A bump splits the office in two until every machine updates, and nothing on screen says why.

### D.2 — What it shows

- [ ] `Message` gains `to: Vec<String>` (who it was sent to) and `heard: Vec<String>` (who answered).
- [ ] The log shows "heard by 2 of 3", with the names on hover.
- [ ] ⚠️ A machine on an older build never answers. Show "sent" for it, never a red mark — no answer is not the same as not delivered.
- [ ] "Heard" means the app received it, not that a person read it. Say "received" in the UI. Read receipts are a separate decision and not in this plan.

### D.3 — Found while verifying: replayed packets

- [ ] Encrypted packets have no replay check (`room.rs:166-181`). Someone on the LAN can record a text packet and send it again, and it shows up again. With ids from D.1: keep the last ~200 `(from, id)` pairs and drop repeats.

---

## Phase E — Groups

> **Status:** built — `ba09186`. Decided 2026-10-07: **saved groups**. Not in a release yet.


Same files: `net.rs` (`set_target` `:260`, `recipients` `:274`), `keys.rs` `aim` `:286`, `config.rs`, `Targets.tsx`.

### E.1 — ⚠️ Decision needed before starting

- [x] Are groups **fixed lists** ("Devs" = these 4 PCs, saved) or **picked on the spot** (tick 3 people, talk)? ✅ Answered 2026-10-07: saved lists.

### E.2 — Build (assuming saved lists)

- [ ] Target becomes `All | One(addr) | Group(name)`. `recipients()` sends to the group members that are live.
- [ ] Members offline are left out and logged, not replaced by anyone. Same rule as one person (`net.rs:288`).
- [ ] Groups saved in `config.rs` by address, like labels.
- [ ] Shown in the target row next to the PCs, with add/edit in Settings.
- [ ] No global key per group by default. Each global key is taken from every other program on the machine (`keys.rs:407-412`). An optional key per group, set by hand, like presets.
- [ ] Fix the doc comment on `recipients()` (`net.rs:268-273`) while here: it says a gone target "falls back", the code sends to nobody (`net.rs:288`). The code is right; the comment is wrong.

---

## Phase F — Can wait

> **Status:** Do-not-disturb built — `6dfcaca`. New `KIND_STATUS = 5` beside the heartbeat, not in it, and no `VER` bump. Not saved: off at every launch. Not in a release yet; not yet tried on two machines.
> **Status:** Per-person volume built — `39c92d5` (0–200% per PC, in Settings → PCs → Edit; live, saved by address). Not yet heard with real audio.

- [x] Do-not-disturb: a tray switch that stops playback and sounds, and tells others in the heartbeat.
- [ ] Per-person volume: a gain per source in the mixer (`audio.rs:940-951`).
- [ ] Refresh PLAN.md. It says "Updated 2026-08-09", and the "Required" section still says "no per-person keys" (`PLAN.md:296-297`), which the same file reverses higher up.
- [x] `tauri-specta` — replaced by the `ipc_shape` test (40cde81): it fails when a Rust struct and `types.d.ts` disagree on field names. tauri-specta is still rc.25 with no stable 2.x; revisit when one ships.

---

## Found while verifying, not in the request

- ✅ **This box's `node_modules` was broken.** `@tauri-apps/plugin-autostart` and `plugin-notification` had empty `dist-js` folders, so `tsc` failed with 2 errors, and `bun install` said "no changes". Deleting the two folders and `bun install --force` fixed it. CI installs fresh, so releases were not affected.
- The `recipients()` comment contradicts the code — fixed in E.2.
- The mixer comment says push-to-talk does not exist yet — fixed in B.1.
- Up to 160 ms of old audio plays after you let go of the talk key — fixed in B.1.
- No replay check on encrypted packets — fixed in D.3.
