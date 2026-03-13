# Barber

A lightweight, open-source audio editor built in Rust with egui.

## Features

- **Multi-format support** — open WAV, AIFF, MP3, FLAC, and M4A files
- **Non-destructive editing** — cut, copy, paste, crop, ripple/gap delete with full undo/redo history
- **Waveform display** — anti-aliased rendering with zoom, scroll, vertical scaling, and minimap overview
- **Playback** — CoreAudio output with loop, follow-playhead, in/out points, and variable speed/pitch
- **Metering** — stereo peak meter with dB ruler and gain fader
- **Processing** — fade in/out, reverse, normalize, DC offset removal, snap-to-zero-crossing
- **Customizable** — editable keybinds and full color theme via TOML config files
- **Export** — save to WAV

## Install

Download `Barber-macos.zip` from [Releases](../../releases), unzip, and drag `Barber.app` to Applications.

Since the app is ad-hoc signed, on first launch you may need to right-click and choose "Open" to bypass Gatekeeper.

## Building from source

Requires Rust. macOS only (CoreAudio).

```sh
cargo build --release
```

The binary will be at `target/release/barber`.

## Usage

```sh
# Launch and open a file via the file picker
barber

# Or drag and drop an audio file onto the window
```

## Keybindings

All keybinds are customizable via config (see below). Defaults:

| Action | Key |
|---|---|
| Play / Pause | `Space` |
| Play Selection | `Shift+Space` |
| Stop | `Esc` |
| Loop | `L` |
| Follow Playhead | `F` |
| Undo / Redo | `Cmd+Z` / `Cmd+Shift+Z` |
| Cut / Copy / Paste | `Cmd+X` / `Cmd+C` / `Cmd+V` |
| Duplicate | `Cmd+D` |
| Select All | `Cmd+A` |
| Crop | `Cmd+K` |
| Ripple Delete | `Shift+Backspace` |
| Gap Delete | `Backspace` |
| Open / Export | `Cmd+O` / `Cmd+E` |
| Zoom In / Out | `Cmd+=` / `Cmd+-` |
| Zoom to Fit | `Return` |
| Vertical Zoom | `Cmd+Shift+=` / `Cmd+Shift+-` / `Cmd+0` |
| Volume Up / Down | `Cmd+Up` / `Cmd+Down` |
| In / Out Points | `Shift+I` / `Shift+O` |
| Go to In / Out | `I` / `O` |
| Go to Start / End | `Up` / `Down` |
| Fade In / Out | `Cmd+F` / `Cmd+Shift+F` |
| Reverse | `Cmd+R` |
| Normalize | `Cmd+Shift+N` |
| Remove DC Offset | `Cmd+Shift+D` |
| Change Speed | `Cmd+Shift+R` |

## Configuration

Config files are created on first launch:

| File | Purpose |
|---|---|
| `~/Library/Application Support/barber/theme.toml` | UI colors |
| `~/Library/Application Support/barber/keybinds.toml` | Key bindings |

Edit either file and restart Barber to apply changes.
