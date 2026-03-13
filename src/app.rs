use std::path::PathBuf;
use std::sync::Arc;

use crate::audio::decode::{self, AudioBuffer};
use crate::audio::export;
use crate::audio::levels::AudioLevels;
use crate::audio::peaks::PeakData;
use crate::audio::playback::PlaybackEngine;
use crate::edit::{EditList, Region};
use crate::history::EditHistory;
use crate::keybinds::Keybinds;
use crate::ui::menu::menu_bar_ui;
use crate::ui::toolbar::{meter_panel_ui, toolbar_ui, ToolbarAction};
use crate::ui::waveform::{WaveformState, WaveformWidget};

pub struct BarberApp {
    audio_buffer: Option<Arc<AudioBuffer>>,
    peak_data: Option<PeakData>,
    edit_list: Option<EditList>,
    playback_engine: Option<PlaybackEngine>,
    waveform_state: WaveformState,
    file_path: Option<PathBuf>,
    error_message: Option<String>,
    history: EditHistory,
    clipboard: Option<Vec<Region>>,
    keybinds: Keybinds,
    audio_levels: AudioLevels,
    last_action: Option<String>,
    loop_enabled: bool,
    follow_playhead: bool,
    was_playing: bool,
    dirty: bool,
    show_quit_dialog: bool,
    prev_modifiers: egui::Modifiers,
}

impl Default for BarberApp {
    fn default() -> Self {
        Self {
            audio_buffer: None,
            peak_data: None,
            edit_list: None,
            playback_engine: None,
            waveform_state: WaveformState::default(),
            file_path: None,
            error_message: None,
            history: EditHistory::new(),
            clipboard: None,
            keybinds: Keybinds::load(),
            audio_levels: AudioLevels::new(),
            last_action: None,
            loop_enabled: false,
            follow_playhead: false,
            was_playing: false,
            dirty: false,
            show_quit_dialog: false,
            prev_modifiers: egui::Modifiers::NONE,
        }
    }
}

impl eframe::App for BarberApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let quit_requested = ctx.input(|i| {
            i.viewport().close_requested()
                || i.events.iter().any(|e| matches!(e,
                    egui::Event::Key { key: egui::Key::Q, pressed: true, modifiers, .. }
                    if modifiers.command && !modifiers.shift && !modifiers.alt
                ))
        });

        if quit_requested && !self.show_quit_dialog {
            if self.dirty {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.show_quit_dialog = true;
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }

        if self.show_quit_dialog {
            egui::Window::new("Unsaved Changes")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("You have unsaved changes. Quit anyway?");
                    ui.horizontal(|ui| {
                        if ui.button("Discard & Quit").clicked() {
                            self.dirty = false;
                            self.show_quit_dialog = false;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_quit_dialog = false;
                        }
                    });
                });
        }

        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if let Some(file) = dropped.first() {
            if let Some(path) = &file.path {
                self.load_file(path.clone());
            }
        }

        let has_file = self.audio_buffer.is_some();
        let has_selection = self.waveform_state.selection.is_some();
        let is_playing = self.playback_engine.as_ref().map_or(false, |e| e.is_playing());

        if is_playing {
            if let Some(engine) = &self.playback_engine {
                self.waveform_state.playhead = engine.position();
                if self.follow_playhead {
                    self.waveform_state.ensure_visible(self.waveform_state.playhead);
                }
            }
            ctx.request_repaint();
        } else if self.was_playing {
            if let Some(engine) = &self.playback_engine {
                let engine_pos = engine.position();
                log::debug!(
                    "Playback just stopped: syncing playhead {} -> engine_pos {}",
                    self.waveform_state.playhead, engine_pos
                );
                self.waveform_state.playhead = engine_pos;
                self.waveform_state.phantom_playhead = None;
                self.audio_levels.set_peaks(0.0, 0.0);
            }
        }
        self.was_playing = is_playing;

        let mut action = None;
        let can_undo = self.history.can_undo();
        let can_redo = self.history.can_redo();
        let has_clipboard = self.clipboard.is_some();

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            action = menu_bar_ui(ui, &self.keybinds, has_file, has_selection, can_undo, can_redo, has_clipboard);
        });

        egui::TopBottomPanel::top("transport").show(ctx, |ui| {
            let toolbar_action = toolbar_ui(ui, is_playing, has_file, self.loop_enabled, self.follow_playhead);
            if action.is_none() {
                action = toolbar_action;
            }
        });

        let mods = ctx.input(|i| i.modifiers);
        if mods != self.prev_modifiers {
            self.prev_modifiers = mods;
            ctx.request_repaint();
        }
        let mut held = String::new();
        if mods.command { held.push_str("\u{2318}"); }
        if mods.shift { held.push_str("\u{21e7}"); }
        if mods.alt { held.push_str("\u{2325}"); }

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(path) = &self.file_path {
                    ui.label(path.file_name().unwrap_or_default().to_string_lossy().to_string());
                    ui.separator();
                }
                if let Some(buf) = &self.audio_buffer {
                    ui.label(format!("{}Hz", buf.sample_rate));
                    ui.separator();
                    ui.label(format!("{}ch", buf.channels));
                    ui.separator();
                }
                if let Some(el) = &self.edit_list {
                    if let Some(buf) = &self.audio_buffer {
                        let total = el.total_frames();
                        let secs = total as f64 / buf.sample_rate as f64;
                        ui.label(format!("{:.1}s", secs));
                        ui.separator();
                    }
                }
                if let Some((start, end)) = self.waveform_state.selection {
                    if let Some(buf) = &self.audio_buffer {
                        let sr = buf.sample_rate as f64;
                        ui.label(format!("sel: {:.2}s - {:.2}s", start as f64 / sr, end as f64 / sr));
                    }
                }
                if let Some(err) = &self.error_message {
                    ui.colored_label(egui::Color32::RED, err);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(action_name) = &self.last_action {
                        ui.label(action_name.as_str());
                    }
                    if !held.is_empty() {
                        ui.label(&held);
                    }
                });
            });
        });

        egui::SidePanel::right("meter_panel")
            .resizable(false)
            .exact_width(94.0)
            .show(ctx, |ui| {
                meter_panel_ui(ui, &self.audio_levels);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let (Some(peaks), Some(edit_list)) = (&self.peak_data, &self.edit_list) {
                let sample_rate = self.audio_buffer.as_ref().map_or(44100, |b| b.sample_rate);
                let audio_samples = self.audio_buffer.as_ref().and_then(|b| b.samples.get(0).map(|s| s.as_slice()));
                let widget = WaveformWidget::new(peaks, edit_list, &mut self.waveform_state, sample_rate, &mut action, self.clipboard.is_some(), audio_samples);
                ui.add(widget);
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("Open an audio file to get started");
                });
            }
        });

        if action.is_none() {
            action = self.keybinds.check_input(ctx, is_playing, has_selection, has_file, can_undo, can_redo, has_clipboard);
        }

        if let Some(action) = action {
            if action == ToolbarAction::Quit {
                if self.dirty {
                    self.show_quit_dialog = true;
                } else {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            } else {
                self.handle_action(action);
            }
        }
    }
}

impl BarberApp {
    fn handle_action(&mut self, action: ToolbarAction) {
        self.error_message = None;
        if let Some(label) = action_label(&action) {
            let sr = self.audio_buffer.as_ref().map(|b| b.sample_rate as f64);
            let range = self.waveform_state.selection.or_else(|| {
                if action.falls_back_to_full_file() {
                    self.edit_list.as_ref().map(|el| (0, el.total_frames()))
                } else {
                    None
                }
            });
            self.last_action = Some(match (range, sr) {
                (Some((s, e)), Some(sr)) => format!("{} ({:.2}s - {:.2}s)", label, s as f64 / sr, e as f64 / sr),
                _ => label.to_string(),
            });
        }

        match action {
            ToolbarAction::OpenFile => self.open_file(),
            ToolbarAction::Export => self.export_file(),
            ToolbarAction::Play => {
                if let (Some(engine), Some(el)) = (&self.playback_engine, &self.edit_list) {
                    let ws = &self.waveform_state;
                    let total = el.total_frames();
                    let has_io = ws.in_point > 0 || ws.out_point < total;
                    let selection = ws.selection;

                    let play_from = selection.map(|(s, _)| s)
                        .unwrap_or(ws.playhead);
                    let play_from = if play_from >= total && total > 0 { 0 } else { play_from };

                    engine.seek(play_from);

                    if self.loop_enabled {
                        let (start, end) = selection
                            .map(|(s, e)| (s, Some(e)))
                            .or_else(|| has_io.then(|| (ws.in_point, Some(ws.out_point))))
                            .unwrap_or((0, None));
                        engine.set_loop(true, start, end);
                        engine.set_stop_at(None);
                    } else {
                        engine.set_loop(false, 0, None);
                        let stop = selection
                            .map(|(_, e)| e)
                            .or_else(|| has_io.then(|| ws.out_point));
                        engine.set_stop_at(stop);
                    }

                    self.waveform_state.phantom_playhead = Some(play_from);
                    engine.play();
                }
            }
            ToolbarAction::Pause => {
                if let Some(engine) = &self.playback_engine {
                    engine.pause();
                }
            }
            ToolbarAction::Stop => {
                if let Some(engine) = &self.playback_engine {
                    engine.stop();
                    let total = self.edit_list.as_ref().map_or(0, |el| el.total_frames());
                    let has_io = self.waveform_state.in_point > 0 || self.waveform_state.out_point < total;
                    self.waveform_state.playhead = if has_io { self.waveform_state.in_point } else { 0 };
                    self.waveform_state.phantom_playhead = None;
                }
            }
            ToolbarAction::ZoomIn => self.waveform_state.zoom_in(),
            ToolbarAction::ZoomOut => self.waveform_state.zoom_out(),
            ToolbarAction::ZoomToFit => {
                if let Some(el) = &self.edit_list {
                    let width = self.waveform_state.last_width;
                    if let Some((start, end)) = self.waveform_state.selection {
                        self.waveform_state.zoom_to_selection(start, end, width);
                    } else {
                        self.waveform_state.zoom_to_fit(el.total_frames(), width);
                    }
                }
            }
            ToolbarAction::GapDelete => self.gap_delete(),
            ToolbarAction::RippleDelete => self.ripple_delete(),
            ToolbarAction::Crop => self.crop(),
            ToolbarAction::Cut => self.cut(),
            ToolbarAction::Copy => self.copy(),
            ToolbarAction::Paste => self.paste(),
            ToolbarAction::Duplicate => self.duplicate(),
            ToolbarAction::Undo => self.undo(),
            ToolbarAction::Redo => self.redo(),
            ToolbarAction::ToggleLoop => {
                self.loop_enabled = !self.loop_enabled;
                if let (Some(engine), Some(el)) = (&self.playback_engine, &self.edit_list) {
                    let total = el.total_frames();
                    let has_io = self.waveform_state.in_point > 0 || self.waveform_state.out_point < total;
                    let (start, end) = self.waveform_state.selection
                        .map(|(s, e)| (s, Some(e)))
                        .or_else(|| has_io.then(|| (self.waveform_state.in_point, Some(self.waveform_state.out_point))))
                        .unwrap_or((0, None));
                    engine.set_loop(self.loop_enabled, start, end);
                }
            }
            ToolbarAction::ToggleFollow => {
                self.follow_playhead = !self.follow_playhead;
            }
            ToolbarAction::PlaySelection => {
                if let (Some(engine), Some((start, end))) = (&self.playback_engine, self.waveform_state.selection) {
                    engine.set_stop_at(Some(end));
                    engine.seek(start);
                    self.waveform_state.phantom_playhead = Some(start);
                    engine.play();
                }
            }
            ToolbarAction::Reverse => self.reverse_selection(),
            ToolbarAction::Normalize => self.normalize(),
            ToolbarAction::RemoveDC => self.remove_dc_offset(),
            ToolbarAction::ToggleFade => self.toggle_fades(),
            ToolbarAction::SetInPoint => {
                self.waveform_state.in_point = self.waveform_state.playhead;
            }
            ToolbarAction::SetOutPoint => {
                self.waveform_state.out_point = self.waveform_state.playhead;
            }
            ToolbarAction::GoToInPoint => {
                self.waveform_state.playhead = self.waveform_state.in_point;
                self.seek_if_playing(self.waveform_state.playhead);
                self.waveform_state.ensure_visible(self.waveform_state.playhead);
            }
            ToolbarAction::GoToOutPoint => {
                self.waveform_state.playhead = self.waveform_state.out_point;
                self.seek_if_playing(self.waveform_state.playhead);
                self.waveform_state.ensure_visible(self.waveform_state.playhead);
            }
            ToolbarAction::GoToStart => {
                self.waveform_state.playhead = 0;
                self.seek_if_playing(0);
                self.waveform_state.ensure_visible(0);
            }
            ToolbarAction::GoToEnd => {
                if let Some(el) = &self.edit_list {
                    let end = el.total_frames();
                    self.waveform_state.playhead = end;
                    self.seek_if_playing(end);
                    self.waveform_state.ensure_visible(end);
                }
            }
            ToolbarAction::NudgeLeft => {
                let sr = self.audio_buffer.as_ref().map_or(44100, |b| b.sample_rate);
                let step = (sr / 100).max(1) as usize;
                self.waveform_state.playhead = self.waveform_state.playhead.saturating_sub(step);
                self.seek_if_playing(self.waveform_state.playhead);
                self.waveform_state.ensure_visible(self.waveform_state.playhead);
            }
            ToolbarAction::NudgeRight => {
                let sr = self.audio_buffer.as_ref().map_or(44100, |b| b.sample_rate);
                let step = (sr / 100).max(1) as usize;
                let total = self.edit_list.as_ref().map_or(0, |el| el.total_frames());
                self.waveform_state.playhead = (self.waveform_state.playhead + step).min(total);
                self.seek_if_playing(self.waveform_state.playhead);
                self.waveform_state.ensure_visible(self.waveform_state.playhead);
            }
            ToolbarAction::VolumeUp => {
                let vol = (self.audio_levels.volume() + 0.05).min(2.0);
                self.audio_levels.set_volume(vol);
            }
            ToolbarAction::VolumeDown => {
                let vol = (self.audio_levels.volume() - 0.05).max(0.0);
                self.audio_levels.set_volume(vol);
            }
            ToolbarAction::SelectAll => {
                if let Some(el) = &self.edit_list {
                    let total = el.total_frames();
                    if total > 0 {
                        self.waveform_state.selection = Some((0, total));
                    }
                }
            }
            ToolbarAction::Quit => {}
        }
    }

    fn open_file(&mut self) {
        let file = rfd::FileDialog::new()
            .add_filter("Audio", &["wav", "aiff", "aif", "mp3", "flac", "m4a", "ogg"])
            .pick_file();
        if let Some(path) = file {
            self.load_file(path);
        }
    }

    fn load_file(&mut self, path: PathBuf) {
        match decode::decode_file(&path) {
            Ok(buffer) => {
                let buffer = Arc::new(buffer);
                let peaks = PeakData::compute(&buffer);
                let edit_list = EditList::from_length(buffer.num_frames);

                let engine = PlaybackEngine::new(Arc::clone(&buffer), edit_list.clone(), self.audio_levels.clone());
                match engine {
                    Ok(engine) => self.playback_engine = Some(engine),
                    Err(e) => self.error_message = Some(format!("Playback init failed: {}", e)),
                }

                let num_frames = buffer.num_frames;
                self.audio_buffer = Some(buffer);
                self.peak_data = Some(peaks);
                self.edit_list = Some(edit_list);
                self.file_path = Some(path);
                self.waveform_state = WaveformState::default();
                self.waveform_state.out_point = num_frames;
                self.history.clear();
                self.last_action = None;
                self.dirty = false;
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to decode: {}", e));
            }
        }
    }

    fn export_file(&mut self) {
        let (Some(buffer), Some(edit_list)) = (&self.audio_buffer, &self.edit_list) else {
            return;
        };

        let file = rfd::FileDialog::new()
            .add_filter("WAV", &["wav"])
            .set_file_name("export.wav")
            .save_file();

        let Some(path) = file else { return };

        match export::export_wav(&path, buffer, edit_list) {
            Ok(()) => { self.dirty = false; }
            Err(e) => {
                self.error_message = Some(format!("Export failed: {}", e));
            }
        }
    }

    fn gap_delete(&mut self) {
        let Some((start, end)) = self.waveform_state.selection else { return };
        if let Some(el) = &self.edit_list {
            self.history.push(el.clone());
        }
        let old_total = self.edit_list.as_ref().map(|el| el.total_frames()).unwrap_or(0);
        if let Some(el) = &mut self.edit_list {
            el.gap_delete(start, end);
            self.waveform_state.selection = None;
        }
        self.post_edit(old_total);
    }

    fn ripple_delete(&mut self) {
        let Some((start, end)) = self.waveform_state.selection else { return };
        if let Some(el) = &self.edit_list {
            self.history.push(el.clone());
        }
        let old_total = self.edit_list.as_ref().map(|el| el.total_frames()).unwrap_or(0);
        if let Some(el) = &mut self.edit_list {
            el.ripple_delete(start, end);
            self.waveform_state.selection = None;
            for point in [&mut self.waveform_state.playhead, &mut self.waveform_state.in_point, &mut self.waveform_state.out_point] {
                if *point >= end {
                    *point -= end - start;
                } else if *point > start {
                    *point = start;
                }
            }
        }
        self.post_edit(old_total);
    }

    fn crop(&mut self) {
        let Some((start, end)) = self.waveform_state.selection else { return };
        if let Some(el) = &self.edit_list {
            self.history.push(el.clone());
        }
        let old_total = self.edit_list.as_ref().map(|el| el.total_frames()).unwrap_or(0);
        if let Some(el) = &mut self.edit_list {
            el.crop(start, end);
            self.waveform_state.selection = None;
            self.waveform_state.playhead = 0;
        }
        self.post_edit(old_total);
    }

    fn copy(&mut self) {
        let Some((start, end)) = self.waveform_state.selection else { return };
        if let Some(el) = &self.edit_list {
            self.clipboard = Some(el.extract_regions(start, end));
        }
    }

    fn cut(&mut self) {
        self.copy();
        self.ripple_delete();
    }

    fn paste(&mut self) {
        let Some(regions) = self.clipboard.clone() else { return };
        if let Some(el) = &self.edit_list {
            self.history.push(el.clone());
        }
        let old_total = self.edit_list.as_ref().map(|el| el.total_frames()).unwrap_or(0);
        if let Some(el) = &mut self.edit_list {
            el.insert(self.waveform_state.playhead, &regions);
            self.waveform_state.selection = None;
        }
        self.post_edit(old_total);
    }

    fn duplicate(&mut self) {
        let Some((start, end)) = self.waveform_state.selection else { return };
        if let Some(el) = &self.edit_list {
            self.history.push(el.clone());
        }
        let old_total = self.edit_list.as_ref().map(|el| el.total_frames()).unwrap_or(0);
        if let Some(el) = &mut self.edit_list {
            let regions = el.extract_regions(start, end);
            el.insert(end, &regions);
            let dup_len = end - start;
            self.waveform_state.selection = Some((end, end + dup_len));
        }
        self.post_edit(old_total);
    }

    fn undo(&mut self) {
        if let Some(current) = self.edit_list.clone() {
            let old_total = current.total_frames();
            if let Some(prev) = self.history.undo(current) {
                self.edit_list = Some(prev);
                self.waveform_state.selection = None;
                self.waveform_state.playhead = self.waveform_state.playhead.min(
                    self.edit_list.as_ref().map_or(0, |el| el.total_frames()),
                );
                self.post_edit(old_total);
            }
        }
    }

    fn redo(&mut self) {
        if let Some(current) = self.edit_list.clone() {
            let old_total = current.total_frames();
            if let Some(next) = self.history.redo(current) {
                self.edit_list = Some(next);
                self.waveform_state.selection = None;
                self.waveform_state.playhead = self.waveform_state.playhead.min(
                    self.edit_list.as_ref().map_or(0, |el| el.total_frames()),
                );
                self.post_edit(old_total);
            }
        }
    }

    fn post_edit(&mut self, old_total: usize) {
        self.dirty = true;
        if let Some(el) = &self.edit_list {
            let new_total = el.total_frames();
            if old_total > 0 && new_total > 0 {
                let ratio = new_total as f64 / old_total as f64;
                self.waveform_state.zoom *= ratio;
                self.waveform_state.scroll_offset *= ratio;
            }
            let max_scroll = (new_total as f64 - self.waveform_state.last_width as f64 * self.waveform_state.zoom).max(0.0);
            self.waveform_state.scroll_offset = self.waveform_state.scroll_offset.min(max_scroll);
            self.waveform_state.in_point = self.waveform_state.in_point.min(new_total);
            self.waveform_state.out_point = self.waveform_state.out_point.min(new_total).max(self.waveform_state.in_point);
        }
        self.sync_playback_engine();
    }

    fn reverse_selection(&mut self) {
        let Some(el) = &self.edit_list else { return };
        let (start, end) = self.waveform_state.selection.unwrap_or((0, el.total_frames()));
        if start >= end { return; }
        self.history.push(el.clone());
        if let Some(el) = &mut self.edit_list {
            el.reverse_selection(start, end);
        }
        self.dirty = true;
        self.sync_playback_engine();
    }

    fn normalize(&mut self) {
        let (Some(buffer), Some(el)) = (&self.audio_buffer, &self.edit_list) else {
            log::debug!("normalize: no buffer or edit list");
            return;
        };
        let selection = self.waveform_state.selection;
        let (start, end) = selection.unwrap_or((0, el.total_frames()));
        log::debug!("normalize: selection={:?} range={}..{} total={}", selection, start, end, el.total_frames());
        if start >= end {
            log::debug!("normalize: empty range, bailing");
            return;
        }
        self.history.push(el.clone());
        let mut peak: f32 = 0.0;
        let mut frame_count: usize = 0;
        for maybe_frame in el.iter_source_frames(start, end - start) {
            if let Some((sf, gain, _dc_offset)) = maybe_frame {
                for ch in 0..buffer.channels as usize {
                    let val = (buffer.samples[ch].get(sf).copied().unwrap_or(0.0) * gain).abs();
                    if val > peak { peak = val; }
                }
                frame_count += 1;
            }
        }
        let gain_factor = if peak > 0.0 { 1.0 / peak } else { 1.0 };
        log::debug!("normalize: scanned {} frames, peak={}, gain_factor={}", frame_count, peak, gain_factor);
        if peak > 0.0 {
            if let Some(el) = &mut self.edit_list {
                el.set_gain_range(start, end, gain_factor);
            }
            self.dirty = true;
            self.sync_playback_engine();
            log::debug!("normalize: applied gain_factor={} to range {}..{}", gain_factor, start, end);
        } else {
            log::debug!("normalize: peak is 0, nothing to do");
        }
    }

    fn remove_dc_offset(&mut self) {
        let (Some(buffer), Some(el)) = (&self.audio_buffer, &self.edit_list) else { return };
        let selection = self.waveform_state.selection;
        let (start, end) = selection.unwrap_or((0, el.total_frames()));
        if start >= end { return; }
        self.history.push(el.clone());
        let mut sum: f64 = 0.0;
        let mut count: u64 = 0;
        for maybe_frame in el.iter_source_frames(start, end - start) {
            if let Some((sf, _gain, _dc)) = maybe_frame {
                for ch in 0..buffer.channels as usize {
                    sum += buffer.samples[ch].get(sf).copied().unwrap_or(0.0) as f64;
                    count += 1;
                }
            }
        }
        if count == 0 { return; }
        let mean = (sum / count as f64) as f32;
        if let Some(el) = &mut self.edit_list {
            el.set_dc_offset_range(start, end, mean);
        }
        self.dirty = true;
        self.sync_playback_engine();
    }

    fn toggle_fades(&mut self) {
        if let Some(el) = &mut self.edit_list {
            el.fades_enabled = !el.fades_enabled;
        }
        self.sync_playback_engine();
    }

    fn seek_if_playing(&self, frame: usize) {
        if let Some(engine) = &self.playback_engine {
            if engine.is_playing() {
                engine.seek(frame);
            }
        }
    }

    fn sync_playback_engine(&self) {
        if let (Some(engine), Some(el)) = (&self.playback_engine, &self.edit_list) {
            engine.set_edit_list(el.clone());
        }
    }
}

fn action_label(action: &ToolbarAction) -> Option<&'static str> {
    match action {
        ToolbarAction::GapDelete => Some("Gap Delete"),
        ToolbarAction::RippleDelete => Some("Ripple Delete"),
        ToolbarAction::Crop => Some("Crop"),
        ToolbarAction::Cut => Some("Cut"),
        ToolbarAction::Copy => Some("Copy"),
        ToolbarAction::Paste => Some("Paste"),
        ToolbarAction::Duplicate => Some("Duplicate"),
        ToolbarAction::Undo => Some("Undo"),
        ToolbarAction::Redo => Some("Redo"),
        ToolbarAction::Reverse => Some("Reverse"),
        ToolbarAction::Normalize => Some("Normalize"),
        ToolbarAction::RemoveDC => Some("Remove DC"),
        ToolbarAction::SelectAll => Some("Select All"),
        ToolbarAction::Export => Some("Exported"),
        ToolbarAction::SetInPoint => Some("Set In Point"),
        ToolbarAction::SetOutPoint => Some("Set Out Point"),
        ToolbarAction::GoToInPoint => Some("Go to In"),
        ToolbarAction::GoToOutPoint => Some("Go to Out"),
        ToolbarAction::GoToStart => Some("Go to Start"),
        ToolbarAction::GoToEnd => Some("Go to End"),
        _ => None,
    }
}
