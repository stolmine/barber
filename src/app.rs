use std::path::PathBuf;
use std::sync::Arc;

use crate::audio::decode::{self, AudioBuffer};
use crate::audio::export;
use crate::audio::levels::AudioLevels;
use crate::audio::peaks::PeakData;
use crate::audio::playback::PlaybackEngine;
use crate::edit::{EditList, FadeCurve, Region};
use crate::history::EditHistory;
use crate::keybinds::Keybinds;
use crate::ui::menu::menu_bar_ui;
use crate::ui::minimap::{minimap_ui, MinimapDrag};
use crate::ui::toolbar::{gain_panel_ui, meter_panel_ui, toolbar_ui, ToolbarAction};
use crate::theme::AppTheme;
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
    snap_to_zero: bool,
    was_playing: bool,
    dirty: bool,
    show_quit_dialog: bool,
    show_speed_dialog: bool,
    speed_pct: f32,
    speed_semitones: f32,
    speed_cents: f32,
    speed_preview: bool,
    speed_interp: u32,
    prev_modifiers: egui::Modifiers,
    minimap_drag: MinimapDrag,
    pending_file_op: Option<std::sync::mpsc::Receiver<Option<PathBuf>>>,
    pending_export_op: Option<std::sync::mpsc::Receiver<Option<PathBuf>>>,
    theme: AppTheme,
    gain_db: f32,
    gain_db_start: f32,
    gain_dragging: bool,
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
            snap_to_zero: true,
            was_playing: false,
            dirty: false,
            show_quit_dialog: false,
            show_speed_dialog: false,
            speed_pct: 100.0,
            speed_semitones: 0.0,
            speed_cents: 0.0,
            speed_preview: false,
            speed_interp: 1,
            prev_modifiers: egui::Modifiers::NONE,
            minimap_drag: MinimapDrag::None,
            pending_file_op: None,
            pending_export_op: None,
            theme: AppTheme::load(),
            gain_db: 0.0,
            gain_db_start: 0.0,
            gain_dragging: false,
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

        if self.show_speed_dialog {
            egui::Window::new("Change Speed")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Speed:");
                        let resp = ui.add(egui::DragValue::new(&mut self.speed_pct)
                            .range(10.0..=400.0)
                            .speed(0.5)
                            .suffix("%"));
                        if resp.changed() {
                            let total_st = 12.0 * (self.speed_pct / 100.0).log2();
                            self.speed_semitones = total_st.round();
                            self.speed_cents = (total_st - self.speed_semitones) * 100.0;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Semitones:");
                        let st_resp = ui.add(egui::DragValue::new(&mut self.speed_semitones)
                            .range(-24.0..=24.0)
                            .speed(0.1));
                        ui.label("Cents:");
                        let ct_resp = ui.add(egui::DragValue::new(&mut self.speed_cents)
                            .range(-50.0..=50.0)
                            .speed(0.5));
                        if st_resp.changed() || ct_resp.changed() {
                            self.speed_pct = 100.0 * 2.0f32.powf((self.speed_semitones + self.speed_cents / 100.0) / 12.0);
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Interpolation:");
                        let labels = ["Nearest", "Linear", "Cubic"];
                        for (i, label) in labels.iter().enumerate() {
                            if ui.selectable_label(self.speed_interp == i as u32, *label).clicked() {
                                self.speed_interp = i as u32;
                                self.audio_levels.set_interpolation(self.speed_interp);
                            }
                        }
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        let preview_label = if self.speed_preview { "Stop Preview" } else { "Preview" };
                        if ui.button(preview_label).clicked() {
                            self.speed_preview = !self.speed_preview;
                            if self.speed_preview {
                                let speed = self.speed_pct / 100.0;
                                self.audio_levels.set_speed(speed);
                                self.audio_levels.set_interpolation(self.speed_interp);
                                if let Some(engine) = &self.playback_engine {
                                    let (start, stop) = self.waveform_state.selection
                                        .map(|(s, e)| (s, Some(e)))
                                        .unwrap_or((0, None));
                                    engine.seek(start);
                                    engine.set_loop(false, 0, None);
                                    engine.set_stop_at(stop);
                                    engine.play();
                                }
                            } else {
                                if let Some(engine) = &self.playback_engine {
                                    engine.stop();
                                }
                                self.audio_levels.set_speed(1.0);
                            }
                        }
                        if ui.button("OK").clicked() {
                            let speed = self.speed_pct / 100.0;
                            if let Some(edit_list) = &mut self.edit_list {
                                self.history.push("Change Speed", edit_list.clone());
                                let total = edit_list.total_frames();
                                let (start, end) = self.waveform_state.selection
                                    .unwrap_or((0, total));
                                edit_list.apply_speed_range(start, end, speed);
                                if let Some(engine) = &self.playback_engine {
                                    engine.set_edit_list(edit_list.clone());
                                }
                                let new_total = edit_list.total_frames();
                                let new_end = if end == total { new_total } else {
                                    start + ((end - start) as f64 / speed as f64).round() as usize
                                };
                                self.waveform_state.selection = Some((start, new_end.min(new_total)));
                                self.waveform_state.out_point = self.waveform_state.out_point.min(new_total);
                                self.dirty = true;
                            }
                            self.audio_levels.set_speed(1.0);
                            if self.speed_preview {
                                if let Some(engine) = &self.playback_engine {
                                    engine.stop();
                                }
                            }
                            self.show_speed_dialog = false;
                            self.speed_preview = false;
                        }
                        if ui.button("Cancel").clicked() {
                            if self.speed_preview {
                                if let Some(engine) = &self.playback_engine {
                                    engine.stop();
                                }
                            }
                            self.audio_levels.set_speed(1.0);
                            self.show_speed_dialog = false;
                            self.speed_preview = false;
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

        if let Some(rx) = &self.pending_file_op {
            if let Ok(result) = rx.try_recv() {
                if let Some(path) = result {
                    self.load_file(path);
                }
                self.pending_file_op = None;
            }
        }

        if let Some(rx) = &self.pending_export_op {
            if let Ok(result) = rx.try_recv() {
                if let Some(path) = result {
                    if let (Some(buffer), Some(edit_list)) = (&self.audio_buffer, &self.edit_list) {
                        match export::export_wav(&path, buffer, edit_list) {
                            Ok(()) => { self.dirty = false; }
                            Err(e) => {
                                self.error_message = Some(format!("Export failed: {}", e));
                            }
                        }
                    }
                }
                self.pending_export_op = None;
            }
        }

        if self.pending_file_op.is_some() || self.pending_export_op.is_some() {
            ctx.request_repaint();
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
            let undo_label = self.history.undo_label();
            let redo_label = self.history.redo_label();
            action = menu_bar_ui(ui, &self.keybinds, has_file, has_selection, undo_label, redo_label, has_clipboard);
        });

        egui::TopBottomPanel::top("transport").show(ctx, |ui| {
            let toolbar_action = toolbar_ui(ui, is_playing, has_file, self.loop_enabled, self.follow_playhead, self.snap_to_zero);
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
                    ui.colored_label(self.theme.error_text, err);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(action_name) = &self.last_action {
                        ui.label(action_name.as_str());
                    }
                    if !held.is_empty() {
                        ui.label(&held);
                    }
                    if let Some(el) = &self.edit_list {
                        let total = el.total_frames();
                        let width = self.waveform_state.last_width as f64;
                        if width > 0.0 && total > 0 {
                            let fit_zoom = total as f64 / width;
                            let h_pct = (fit_zoom / self.waveform_state.zoom * 100.0).round();
                            let v_pct = (self.waveform_state.vertical_zoom * 100.0).round();
                            ui.label(format!("H:{:.0}% V:{:.0}%", h_pct, v_pct));
                        }
                    }
                });
            });
        });

        egui::TopBottomPanel::bottom("minimap")
            .exact_height(32.0)
            .show(ctx, |ui| {
                if let (Some(peaks), Some(edit_list)) = (&self.peak_data, &self.edit_list) {
                    minimap_ui(ui, peaks, edit_list, &mut self.waveform_state, &mut self.minimap_drag, &self.theme.minimap);
                }
            });

        egui::SidePanel::right("meter_panel")
            .resizable(false)
            .exact_width(94.0)
            .show(ctx, |ui| {
                meter_panel_ui(ui, &self.audio_levels, &self.theme.meter);
            });

        egui::SidePanel::left("gain_panel")
            .resizable(false)
            .exact_width(60.0)
            .show(ctx, |ui| {
                if let Some(edit_list) = &mut self.edit_list {
                    if !self.gain_dragging {
                        let total = edit_list.total_frames();
                        let (start, end) = self.waveform_state.selection.unwrap_or((0, total));
                        let avg_gain = edit_list.average_gain(start, end);
                        self.gain_db = 20.0 * avg_gain.log10();
                    }
                    let (changed, released) = gain_panel_ui(ui, &mut self.gain_db, &self.theme.meter);
                    if changed && !self.gain_dragging {
                        self.history.push("Gain", edit_list.clone());
                        self.gain_db_start = self.gain_db;
                        self.gain_dragging = true;
                    }
                    if changed {
                        if let Some(snapshot) = self.history.peek_undo() {
                            let db_delta = self.gain_db - self.gain_db_start;
                            let gain_factor = 10.0f32.powf(db_delta / 20.0);
                            let total = snapshot.total_frames();
                            let (start, end) = self.waveform_state.selection.unwrap_or((0, total));
                            let mut adjusted = snapshot.extract_regions(start, end);
                            for region in &mut adjusted {
                                region.gain *= gain_factor;
                            }
                            edit_list.ripple_delete_inner(start, end, false);
                            edit_list.insert_inner(start, &adjusted, false);
                            self.dirty = true;
                        }
                    }
                    if released && self.gain_dragging {
                        if let Some(engine) = &self.playback_engine {
                            engine.set_edit_list(edit_list.clone());
                        }
                        self.last_action = Some(format!("Gain {:.1} dB", self.gain_db));
                        self.gain_dragging = false;
                    }
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let (Some(peaks), Some(edit_list)) = (&self.peak_data, &self.edit_list) {
                let sample_rate = self.audio_buffer.as_ref().map_or(44100, |b| b.sample_rate);
                let audio_samples = self.audio_buffer.as_ref().and_then(|b| b.samples.get(0).map(|s| s.as_slice()));
                let widget = WaveformWidget::new(peaks, edit_list, &mut self.waveform_state, sample_rate, &mut action, self.clipboard.is_some(), audio_samples, &self.theme.waveform, self.snap_to_zero);
                ui.add(widget);
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("Open an audio file to get started");
                });
            }
        });

        if action.is_none() && !self.show_speed_dialog && !self.show_quit_dialog {
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
            ToolbarAction::ToggleSnapZero => {
                self.snap_to_zero = !self.snap_to_zero;
            }
            ToolbarAction::PlaySelection => {
                if let Some(engine) = &self.playback_engine {
                    let total = self.edit_list.as_ref().map_or(0, |el| el.total_frames());
                    let (start, end) = self.waveform_state.selection.unwrap_or((0, total));
                    self.loop_enabled = true;
                    engine.set_loop(true, start, Some(end));
                    engine.set_stop_at(None);
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
            ToolbarAction::FadeInLinear => self.apply_fade(true, FadeCurve::Linear),
            ToolbarAction::FadeInExponential => self.apply_fade(true, FadeCurve::Exponential),
            ToolbarAction::FadeInLogarithmic => self.apply_fade(true, FadeCurve::Logarithmic),
            ToolbarAction::FadeInSCurve => self.apply_fade(true, FadeCurve::SCurve),
            ToolbarAction::FadeOutLinear => self.apply_fade(false, FadeCurve::Linear),
            ToolbarAction::FadeOutExponential => self.apply_fade(false, FadeCurve::Exponential),
            ToolbarAction::FadeOutLogarithmic => self.apply_fade(false, FadeCurve::Logarithmic),
            ToolbarAction::FadeOutSCurve => self.apply_fade(false, FadeCurve::SCurve),
            ToolbarAction::VerticalZoomIn => {
                self.waveform_state.vertical_zoom = (self.waveform_state.vertical_zoom * 1.1).clamp(0.5, 20.0);
            }
            ToolbarAction::VerticalZoomOut => {
                self.waveform_state.vertical_zoom = (self.waveform_state.vertical_zoom * 0.9).clamp(0.5, 20.0);
            }
            ToolbarAction::VerticalZoomReset => {
                self.waveform_state.vertical_zoom = 1.0;
                if let Some(el) = &self.edit_list {
                    self.waveform_state.zoom_to_fit(el.total_frames(), self.waveform_state.last_width);
                }
            }
            ToolbarAction::Quit => {}
            ToolbarAction::ChangeSpeed => {
                let current_speed = self.audio_levels.speed();
                self.speed_pct = current_speed * 100.0;
                let total_st = 12.0 * (self.speed_pct / 100.0).log2();
                self.speed_semitones = total_st.round();
                self.speed_cents = (total_st - self.speed_semitones) * 100.0;
                self.speed_interp = self.audio_levels.interpolation();
                self.speed_preview = false;
                self.show_speed_dialog = true;
            }
        }
    }

    fn open_file(&mut self) {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let file = rfd::FileDialog::new()
                .add_filter("Audio", &["wav", "aiff", "aif", "mp3", "flac", "m4a"])
                .pick_file();
            let _ = tx.send(file);
        });
        self.pending_file_op = Some(rx);
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
        if self.audio_buffer.is_none() || self.edit_list.is_none() {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let file = rfd::FileDialog::new()
                .add_filter("WAV", &["wav"])
                .set_file_name("export.wav")
                .save_file();
            let _ = tx.send(file);
        });
        self.pending_export_op = Some(rx);
    }

    fn gap_delete(&mut self) {
        let Some((start, end)) = self.waveform_state.selection else { return };
        if let Some(el) = &self.edit_list {
            self.history.push("Gap Delete", el.clone());
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
            self.history.push("Ripple Delete", el.clone());
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
            self.history.push("Crop", el.clone());
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
            self.history.push("Paste", el.clone());
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
            self.history.push("Duplicate", el.clone());
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
                self.last_action = self.history.redo_label().map(|l| format!("Undo {}", l));
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
                self.last_action = self.history.undo_label().map(|l| format!("Redo {}", l));
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
        self.history.push("Reverse", el.clone());
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
        self.history.push("Normalize", el.clone());
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
        self.history.push("Remove DC", el.clone());
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

    fn apply_fade(&mut self, is_fade_in: bool, curve: FadeCurve) {
        let Some((start, end)) = self.waveform_state.selection else { return };
        let fade_label = format!("{} ({})", if is_fade_in { "Fade In" } else { "Fade Out" }, match curve {
            FadeCurve::Linear => "Linear",
            FadeCurve::Exponential => "Exponential",
            FadeCurve::Logarithmic => "Logarithmic",
            FadeCurve::SCurve => "S-Curve",
        });
        if let Some(el) = &self.edit_list {
            self.history.push(fade_label, el.clone());
        }
        if let Some(el) = &mut self.edit_list {
            if is_fade_in { el.apply_fade_in(start, end, curve); }
            else { el.apply_fade_out(start, end, curve); }
        }
        self.dirty = true;
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
        ToolbarAction::ChangeSpeed => Some("Change Speed"),
        ToolbarAction::SetInPoint => Some("Set In Point"),
        ToolbarAction::SetOutPoint => Some("Set Out Point"),
        ToolbarAction::GoToInPoint => Some("Go to In"),
        ToolbarAction::GoToOutPoint => Some("Go to Out"),
        ToolbarAction::GoToStart => Some("Go to Start"),
        ToolbarAction::GoToEnd => Some("Go to End"),
        ToolbarAction::FadeInLinear => Some("Fade In (Linear)"),
        ToolbarAction::FadeInExponential => Some("Fade In (Exponential)"),
        ToolbarAction::FadeInLogarithmic => Some("Fade In (Logarithmic)"),
        ToolbarAction::FadeInSCurve => Some("Fade In (S-Curve)"),
        ToolbarAction::FadeOutLinear => Some("Fade Out (Linear)"),
        ToolbarAction::FadeOutExponential => Some("Fade Out (Exponential)"),
        ToolbarAction::FadeOutLogarithmic => Some("Fade Out (Logarithmic)"),
        ToolbarAction::FadeOutSCurve => Some("Fade Out (S-Curve)"),
        _ => None,
    }
}
