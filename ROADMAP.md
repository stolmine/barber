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

## v0.1 Features (shipped)
1. Open audio files (WAV, AIFF, MP3, FLAC via symphonia)
2. Waveform rendering with zoom/scroll
3. Audio playback with CoreAudio
4. Selection (rubber-band style)
5. Ripple delete and crop operations
6. Export to WAV

## v0.1.1 Features (shipped)
7. Gap delete, cut/copy/paste editing operations
8. Undo/redo history stack
9. Timeline ruler (adaptive, bottom edge)
10. Channel separator line between L/R
11. Auto-restart playback at EOF
12. Drag and drop file open
13. Context-aware zoom-to-fit (selection or all)
14. 0dB normalized waveforms with visual clipping

## Architecture

### Module Map

| File | Lines | Purpose |
|------|-------|---------|
| `Cargo.toml` | ~25 | Dependencies |
| `src/main.rs` | ~20 | Entry point, eframe launch |
| `src/app.rs` | ~340 | Main `BarberApp` struct, orchestration |
| `src/edit.rs` | ~280 | Edit list data structure (regions, all edit ops) |
| `src/edit_tests.rs` | ~210 | Edit list unit tests |
| `src/history.rs` | ~45 | Undo/redo history stack |
| `src/audio/mod.rs` | ~10 | Module re-exports |
| `src/audio/decode.rs` | ~120 | Symphonia-based PCM decoding |
| `src/audio/playback.rs` | ~180 | CoreAudio playback engine |
| `src/audio/export.rs` | ~50 | WAV export via hound |
| `src/audio/peaks.rs` | ~100 | Peak/RMS mipmap computation with rayon |
| `src/ui/mod.rs` | ~5 | Module re-exports |
| `src/ui/waveform.rs` | ~320 | Custom egui waveform widget with ruler |
| `src/ui/toolbar.rs` | ~190 | Toolbar with transport/zoom/edit controls |

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
Project Setup
   |
   +--> Audio Decoding  ----+
   |                        |
   +--> Peak Computation ---+--> Waveform Widget --+
   |                        |                      |
   +--> Edit List ----------+--> Playback ---------+--> App Integration
   |                                               |
   +--> Toolbar UI --------------------------------+
   |                                               |
   +--> Export ------------------------------------+
```

## Implementation Tasks

### Task 1: Project Setup
- Cargo.toml with all dependencies
- `main.rs` — env_logger init, eframe launch
- Module stubs for all files
- **Deliverable:** `cargo run` shows empty egui window titled "Barber"

### Task 2: Audio Decoding (`audio/decode.rs`)
- Open file → MediaSourceStream → probe → decode loop
- Convert all formats to f32 PCM AudioBuffer
- Handle WAV, AIFF, MP3, FLAC via symphonia features

### Task 3: Peak Computation (`audio/peaks.rs`)
- Level 0: block size 256, compute (min, max) per block with rayon
- Build ~12-14 mipmap levels by merging pairs
- `get_peaks()` selects appropriate mip level for requested zoom

### Task 4: Edit List (`edit.rs`)
- Region-based non-destructive edit list
- `from_length()`, `total_frames()`, `resolve()`, `iter_source_frames()`
- `ripple_delete()` and `crop()` operations

### Task 5: Audio Playback (`audio/playback.rs`)
- CoreAudio output AudioUnit with render callback
- `Arc<Mutex<PlaybackState>>` shared between audio thread and UI
- `try_lock()` in callback, output silence on contention
- Play, pause, stop, seek, position query

### Task 6: Waveform Widget (`ui/waveform.rs`)
- Custom egui widget using `allocate_painter`
- Render peaks as vertical min/max lines per pixel
- Walk edit list regions to map edit-space → source-space for peak lookup
- Zoom (Cmd+scroll), scroll (horizontal scroll/shift+scroll)
- Click-and-drag selection, click-to-place playhead
- Playhead rendering as vertical line

### Task 7: Toolbar UI (`ui/toolbar.rs`)
- Horizontal layout in TopBottomPanel
- File: Open, Export | Transport: Play/Pause, Stop | Zoom: In, Out, Fit | Edit: Delete, Crop
- Conditional enable/disable based on state
- Keyboard shortcuts: Space (play/pause), Delete (ripple delete), Cmd+E (export)

### Task 8: Main App Integration (`app.rs`)
- Wire all modules together in `BarberApp`
- Handle toolbar actions → dispatch to appropriate modules
- Sync playhead from playback engine each frame
- Status bar with file info, duration, selection range
- `request_repaint()` during playback for smooth playhead animation

### Task 9: WAV Export (`audio/export.rs`)
- Use hound to write 16-bit WAV
- Walk edit list to resolve source frames
- Convert f32 → i16, write interleaved

## Design Decisions

1. **Synchronous decoding (v0.1):** File open blocks UI. Fine for files <50MB. Async is v0.2.
2. **Mutex in audio callback:** `try_lock()` avoids priority inversion; outputs silence on contention (~5ms). Lock-free ring buffer is v0.2.
3. **Non-destructive editing:** Original AudioBuffer never modified. Undo is trivial to add (snapshot `Vec<Region>`).
4. **Peak data references source frames:** Waveform widget translates edit-space → source-space before querying peaks. No recomputation after edits.
5. **macOS-only (v0.1):** CoreAudio. Cross-platform via `cpal` is future work.
6. **f32 internal format:** Simplifies pipeline, ~4x memory vs i16, fine for modern machines.

## v0.2 Wishlist

### Editing
- **Duplicate region:** Copy selected region and insert it adjacent
- **Reverse selection:** Reverse sample order within selected region
- **Silence selection:** Replace selection with silence (zero samples)
- **Fade in/out on edit boundaries:** Crossfade to prevent clicks at cut points
- **Normalize:** Scale audio to peak at 0dB (or user-specified level)
- **DC offset removal:** Center waveform on zero crossing

### Waveform Display
- **Amplitude ruler:** Per-channel amplitude scale on left side
- **Vertical zoom:** Scale waveform amplitude independently of window height
- **Amplitude control:** Gain adjustment with live waveform preview
- **Anti-aliased waveforms:** Smooth rendering instead of per-pixel lines
- **Snap-to-zero-crossing:** Selection edges snap to nearest zero crossing for click-free edits

### Playback
- **Phantom playhead:** Ghost marker at play-start position while actual playhead advances
- **Follow playhead toggle:** Auto-scroll viewport to keep playhead centered at current zoom
- **Loop playback:** Loop selected region or entire file
- **Play selection only:** Audition just the selected region
- **Speed/pitch control:** Variable playback rate with optional pitch preservation

### Interaction
- **Full hotkey coverage:** Keyboard shortcuts for all operations
- **Trackpad gestures:** Native pinch-to-zoom and two-finger scroll
- **Right-click context menu:** Selection-aware actions (cut, copy, paste, delete, crop, export selection)
- **Prompt to save on quit:** Warning when quitting with unsaved modifications
- **Menu bar:** Standard macOS menu bar for accessibility and discoverability

### UI Polish (last priority)
- **Sexier UI:** Better colors, typography, spacing, custom styling
- **Tabbed concurrent projects:** Open multiple files, splice material between them
- **Metering:** just simple stereo metering with themable colors, could be cute with an ascii option borrowed from monokit
- **Minimap:** adaptive overview of waveform when zoomed

### Infrastructure
- Async file loading with progress bar
- Lock-free audio thread communication
- Cross-platform audio via `cpal`
- 24-bit and 32-bit float WAV export
- AIFF / OGG / MP3 export
- Sample rate conversion on export
- Recent files list
- Optimization - improve load times and snappiness as much as possible. File picker in particular is opening incredibly slowly right now
