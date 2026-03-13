# Barber — Lightweight Audio Editor Roadmap

## Vision
A simple, fast, lightweight audio editor built from purely open source Rust components with minimal functionality.

## Stack
- **UI:** `egui` + `eframe`
- **Audio decode:** `symphonia` (pure Rust)
- **Playback:** `coreaudio-rs` (macOS)
- **WAV export:** `hound`
- **Parallelism:** `rayon`
- **File dialogs:** `rfd`

## Shipped Features

### v0.1
1. Open audio files (WAV, AIFF, MP3, FLAC via symphonia)
2. Waveform rendering with zoom/scroll
3. Audio playback with CoreAudio
4. Selection (rubber-band style)
5. Ripple delete and crop operations
6. Export to WAV

### v0.1.1
7. Gap delete, cut/copy/paste editing operations
8. Undo/redo history stack
9. Timeline ruler (adaptive, bottom edge)
10. Channel separator line between L/R
11. Auto-restart playback at EOF
12. Drag and drop file open
13. Context-aware zoom-to-fit (selection or all)
14. 0dB normalized waveforms with visual clipping

### v0.1.2
15. Loop playback — toggle loop mode; loops selection or full file (L key)
16. Play selection — Shift+Space auditions selected region only
17. Follow playhead — auto-scroll viewport to keep playhead visible (F key)
18. Right-click context menu — selection-aware edit actions on waveform

### v0.1.3
19. Snap-to-zero-crossing — selection edges auto-snap to nearest zero crossing on release
20. Duplicate region — Cmd+D copies selected region and inserts immediately after
21. Phantom playhead — ghost marker at play-start position with adaptive contrast over waveform

### v0.1.4
22. Menu bar — File/Edit/Transport/View menus replacing toolbar buttons
23. Slim transport bar — Play/Pause, Stop, Loop, Follow only
24. Editable keybind system — TOML-configurable at ~/.config/barber/keybinds.toml
25. Reverse selection — Cmd+R reverses sample order in selected region
26. Normalize — per-selection or whole-file 0dB peak normalization via per-region gain
27. Waveform rendering resolves edit→source per pixel via `for_each_source_range` — correct display after all edits
28. Cut/Copy/Paste hotkeys fixed — detect egui `Event::Cut/Copy/Paste` events alongside `key_pressed`

### v0.1.5
29. Region refactor — Region struct with kind enum, gain, dc_offset, fade_in/fade_out fields
30. Boundary fades — auto-apply 128-frame (~3ms) fade in/out at edit splice points to prevent clicks
31. Toggle fades — Edit > Toggle Fades to enable/disable boundary fade envelopes
32. DC offset removal — Edit > Remove DC Offset (Cmd+Shift+D) centers waveform on zero
33. Prompt to save on quit — dirty tracking with confirmation dialog on close/Cmd+Q
34. Select all — Cmd+A or double-click waveform selects entire file
35. Quit keybind — Cmd+Q with macOS native menu disabled to allow app-level intercept

### v0.1.6
36. Play from selection — Play starts from selection start when a region is selected
37. Action history status bar — right-justified last action readout with affected timespan
38. Full-file reverse — Reverse operates on entire file when no selection, available without selection in menu/keybinds
39. Full-file remove DC — Remove DC operates on entire file when no selection

### v0.1.7
40. In/out points — Shift+I / Shift+O set markers at playhead, I / O jump to them. Dashed green/red lines. Clamped after edits, ripple-shifted on ripple delete
41. Keystroke tracking — status bar shows held modifier keys (⌘⇧⌥) live, groundwork for vim-style chained keybinds
42. Custom font — JetBrains Mono Nerd Font bundled for full glyph coverage (modifier symbols, nerd font icons)
43. Keybind forward-compat — new default keybinds merge into existing user config without overwriting customizations

### v0.1.8
44. In/out constrained playback — Play respects in/out bounds: stops at out_point, loops between in/out when loop enabled. Selection still overrides in/out (DAW convention)
45. Arrow key navigation — Up/Down jump to start/end of file, Left/Right nudge playhead by 10ms. All configurable via keybinds.toml
46. Stop respects in/out — Stop resets playhead to in_point when in/out bounds are set, otherwise resets to 0

## Architecture

### Design Decisions
1. **Synchronous decoding (v0.1):** File open blocks UI. Fine for files <50MB. Async is v0.2.
2. **Mutex in audio callback:** `try_lock()` avoids priority inversion; outputs silence on contention (~5ms). Lock-free ring buffer is v0.2.
3. **Non-destructive editing:** Original AudioBuffer never modified. Undo is trivial (snapshot `Vec<Region>`).
4. **Peak data references source frames:** Waveform widget translates edit-space → source-space before querying peaks. No recomputation after edits.
5. **macOS-only (v0.1):** CoreAudio. Cross-platform via `cpal` is future work.
6. **f32 internal format:** Simplifies pipeline, ~4x memory vs i16, fine for modern machines.

### Core Data Types

**AudioBuffer** (`audio/decode.rs`):
- `samples: Vec<Vec<f32>>` — per-channel f32 samples normalized to [-1.0, 1.0]
- `sample_rate: u32`, `channels: u16`, `num_frames: usize`

**PeakData** (`audio/peaks.rs`):
- Multi-level mipmap pyramid (block size 256, doubling per level)
- `get_peaks(channel, start_frame, end_frame, num_pixels) -> Vec<(f32, f32)>`
- Computed once after decode, never recomputed after edits

**EditList** (`edit.rs`):
- `regions: Vec<Region>` where `Region { source_start, source_end }`
- Non-destructive: original AudioBuffer is never modified
- `ripple_delete(start, end)` — remove range in edit-space
- `crop(start, end)` — keep only range in edit-space
- `resolve(edit_frame) -> source_frame` — maps edit-space to source-space

### Dependency Graph

```
Audio Decoding ------+
                     |
Peak Computation ----+--> Waveform Widget --+
                     |                      |
Edit List -----------+--> Playback ---------+--> App Integration
                                            |
Toolbar UI ---------------------------------+
                                            |
Export -------------------------------------+
```

### v0.1.9
47. Volume fader — vertical fader in right side panel with linear throw (0–2x), double-click to reset to unity, unity gain notch indicator
48. Stereo metering — real-time pre-fader peak meters (L/R) with green/yellow/red segments, smoothed decay, and dB ruler (-40 to 0 dB)
49. Lock-free audio levels — AtomicU32-based volume and peak transport between audio callback and UI thread (no mutex contention)
50. Volume keybinds — Cmd+Up/Down adjusts playback volume in 0.05 steps
51. Live navigation during playback — GoToStart, GoToEnd, GoToIn, GoToOut, NudgeLeft, NudgeRight all seek the engine while playing

### v0.2.0
52. Minimap — Ableton-style arrangement overview bar above status bar. Full waveform always visible, viewport rectangle with dimmed outside regions. Click outside to jump, drag inside to pan, vertical drag to zoom, drag edges to resize view, double-click to zoom-to-fit. Press-frame hit detection for reliable edge grab.

## v0.2 Wishlist

### Editing
- **Selection-scoped adjustments:** When a region is selected, apply pitch/speed, reverse, or amplitude changes to only that region (hotkeys or floating controls à la Adobe Audition)
- **Individual L/R channel editing:** Edit left/right channels independently on stereo files. toggle-able
- **Apply fades in or out, with selectable curves:** the question will be how to select curves

### Waveform Display
- **Amplitude ruler:** Per-channel amplitude scale on left side
- **Vertical zoom:** Scale waveform amplitude independently of window height
- **Amplitude control:** Gain adjustment with live waveform preview
- **Anti-aliased waveforms:** Smooth rendering instead of per-pixel lines

### Playback
- **Speed/pitch control:** Variable playback rate with optional pitch preservation

### Interaction
- **Trackpad gestures:** Native pinch-to-zoom and two-finger scroll
- **BPM detection and beat grid:** Adjustable beat grid for quantized edits with quantized selection on hotkey/toggle

### UI Polish (last priority)
- **Custom styling for in/out points:** Current dashed lines are hard to see — need better visual treatment (thicker, labels, triangular markers, or glow). Add shading/tint of the region between in and out points when they are set off defaults
- **Sexier UI:** Better colors, typography, spacing, custom styling
- **Tabbed concurrent projects:** Open multiple files, splice material between them

### Infrastructure
- Async file loading with progress bar
- Lock-free audio thread communication
- Cross-platform audio via `cpal`
- 24-bit and 32-bit float WAV export
- AIFF / OGG / MP3 export
- Sample rate conversion on export
- Recent files list
- Optimization - improve load times and snappiness as much as possible. File picker in particular is opening incredibly slowly right now
