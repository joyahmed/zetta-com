# Zetta Com

A LAN intercom. Hold a key, and the people you choose hear you — one person or
the whole room. No internet, no accounts, no server.

Built for directing a team: pick a machine, hold a key, talk. Or type, and it
arrives as a notification on their screen.

---

## What it does

- **Push to talk.** Hold **F8** and whoever is selected hears you. Nothing is
  transmitted unless the key is held.
- **Talk to one person or everyone.** `Ctrl+Alt+1…9` talks to the PC in that
  row; `Ctrl+Alt+0` talks to the room.
- **Messages.** Type, or fire a preset with a key. They arrive as native
  notifications when the window is hidden.
- **Finds everyone by itself.** mDNS discovery — no addresses to type on a
  normal network. Machines that discovery cannot reach can be added manually.
- **Says who is actually there.** A heartbeat every two seconds; somebody who
  goes quiet greys out within seven. It reports absence, not just presence.
- **Names you choose.** `DEVS002` is not Rafi. Rename any machine, correct an
  address you typed wrong, or remove it — all in one list.

## Getting it

Grab an installer from [releases](../../releases), or build it yourself.

**On macOS, the first launch will say the app is "damaged."** It is not. The
builds are unsigned, and Gatekeeper refuses a quarantined bundle that carries no
signature. Move it to `/Applications`, then clear the quarantine flag:

```bash
xattr -cr "/Applications/Zetta Com.app"
```

You will need this again after every new download, until the builds are signed.

**Then open System Settings → Privacy & Security → Local Network and turn Zetta
Com on.** macOS normally asks for local network access the first time an app
wants it, but clearing quarantine can mean the prompt never appears — and
without that permission the app looks like it is running perfectly while hearing
nobody at all.

Building it yourself:

```bash
bun install
bun run tauri build
```

Artifacts land in `src-tauri/target/release/bundle/` — MSI and NSIS on Windows,
`.deb` and `.AppImage` on Linux, `.dmg` on macOS. Each platform builds only its
own; the [release workflow](.github/workflows/release.yml) does all of them.

**Building on Windows also needs** [CMake](https://cmake.org/download/), which
libopus is compiled with. On Linux, `libasound2-dev` for ALSA.

## Using it

It starts with the machine and lives in the tray — closing the window hides it,
because an intercom that stops receiving when you tidy your desktop is not an
intercom. **Quit** is in the tray menu.

| Key | |
|---|---|
| `F8` (hold) | talk to whoever is selected |
| `Ctrl+Alt+1…9` (hold) | talk to that PC |
| `Ctrl+Alt+0` (hold) | talk to everyone |
| `Ctrl+Shift+1…9` | message that PC |
| `Ctrl+Shift+0` | message everyone |
| `Ctrl+Alt+S` | start or stop |
| `Ctrl+Alt+A` | add a PC |
| `Ctrl+Alt+T` | open the window |
| `Ctrl+Alt+K` | show all shortcuts |

Every key is listed in the app, **including the ones that failed to register** —
a global shortcut can lose a race to another application, and when it does the
key silently does nothing.

Windows will ask about the firewall on first start. **Allow it.** If you cancel,
Windows writes a *block* rule that outranks everything afterwards and leaves no
visible trace.

## Why it exists

There was an earlier version built from batch files, PowerShell and ffmpeg. It
worked. Nearly every failure in it came from the substrate rather than the
intercom: PowerShell 5.1-only APIs, BOM parsing, CRLF, a script base64-embedded
inside a `.bat`, an installer that swallowed exit codes.

The one that settled it was the firewall. Inbound block rules killed a PC
because the network-facing program was `powershell.exe` — a shared system binary
anything can get muzzled, silently. A dedicated signed executable has its own
firewall identity, its own prompt, its own rule. That is the whole reason this
is an application.

Two things came free: mDNS replaced Windows name resolution, which had broken
two machines outright; and a heartbeat made it possible to report that somebody
is **not** there, which the old version never could.

`docs/PLAN.md` carries the decisions and the reasoning, including what was
rejected and why.

## How it works

```
audio    capture, encode, decode, playback. Knows nothing about sockets.
net      socket, header, peers. Knows nothing about codecs.
session  owns both and wires them together.
```

Opus at 20 ms frames, one frame per UDP datagram, unicast to each recipient —
not broadcast, because some WiFi access points rate-limit or drop broadcast
frames and one person silently hears nothing. Each remote talker gets its own
decoder and its own reorder window; a missing frame is concealed rather than
skipped.

Half duplex, deliberately. Local playback is muted while you transmit, so the
echo path never exists and there is nothing to cancel. Full duplex would have
meant acoustic echo cancellation, and that becomes the project rather than a
step in it.

## Security

**There is none.** Anyone on the network can send audio and text to anyone
running this, and anyone running it hears whatever is sent to its port. There is
no authentication, no encryption, and no access control.

That is a reasonable trade on a trusted office LAN, which is what it was built
for. It is not one anywhere else. Do not run it on a network you do not control.

## Licence

MIT. See [LICENSE](LICENSE).
