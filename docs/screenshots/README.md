# Screenshots

What to capture, and where each one goes in the top-level `README.md`. Save with
these exact filenames — the README references them by name.

Take them from a **release build after a full restart**, not from `tauri dev`:
the custom title bar, the window size and the global shortcuts all come from
`tauri.conf.json` and are not applied by a hot reload.

Capture the window only, not the desktop behind it. No annotations — no boxes,
arrows or highlights. These are the product, not a bug report.

| File | What is in it | Where it goes |
|---|---|---|
| `home.png` | The main window, running, with two or three PCs on the strip and a few messages in the log. The one that has to sell it. | Top of the README, under the intro |
| `talking.png` | The same window with `F8` held — the talk bar filled teal, reading "Talking to …". The one moment the app looks like what it is. | Beside `home.png` |
| `add-pc.png` | The **Add a PC** modal with an address and a name filled in | Install / Using it |
| `pcs.png` | **Settings** — the port, and the PCs list with a couple of machines | Using it |
| `shortcuts.png` | The **Shortcuts** modal, showing the collapsed `1…9` ranges | Shortcuts |
| `diagnostics.png` | **Diagnostics**, ideally with counters that have moved rather than five zeroes | Using it |

A dark-theme set is the priority since that is what most of these will be seen
in, but a light `home.png` is worth having if it is easy — the palette is built
for both and it is worth showing.

Two or three PCs with real names read far better than one called `aaa`. What is
in the message log will be read by everyone who looks at the repository.
