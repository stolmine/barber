use std::path::PathBuf;
use std::sync::Arc;

use crate::audio::decode::{self, AudioBuffer};
use crate::audio::export;
use crate::audio::peaks::PeakData;
use crate::audio::playback::PlaybackEngine;
use crate::edit::EditList;
use crate::ui::toolbar::{toolbar_ui, ToolbarAction};
use crate::ui::waveform::{WaveformState, WaveformWidget};

pub struct BarberApp {
    audio_buffer: Option<Arc<AudioBuffer>>,
    peak_data: Option<PeakData>,
    edit_list: Option<EditList>,
    playback_engine: Option<PlaybackEngine>,
    waveform_state: WaveformState,
    file_path: Option<PathBuf>,
    error_message: Option<String>,
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
        }
    }
}

impl eframe::App for BarberApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let has_file = self.audio_buffer.is_some();
        let has_selection = self.waveform_state.selection.is_some();
        let is_playing = self.playback_engine.as_ref().map_or(false, |e| e.is_playing());

        if is_playing {
            if let Some(engine) = &self.playback_engine {
                self.waveform_state.playhead = engine.position();
            }
            ctx.request_repaint();
        }

        let mut action = None;

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            action = toolbar_ui(ui, is_playing, has_selection, has_file);
        });

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
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let (Some(peaks), Some(edit_list)) = (&self.peak_data, &self.edit_list) {
                let widget = WaveformWidget::new(peaks, edit_list, &mut self.waveform_state);
                ui.add(widget);
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("Open an audio file to get started");
                });
            }
        });

        if let Some(action) = action {
            self.handle_action(action);
        }
    }
}

impl BarberApp {
    fn handle_action(&mut self, action: ToolbarAction) {
        self.error_message = None;

        match action {
            ToolbarAction::OpenFile => self.open_file(),
            ToolbarAction::Export => self.export_file(),
            ToolbarAction::Play => {
                if let Some(engine) = &self.playback_engine {
                    engine.seek(self.waveform_state.playhead);
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
                    self.waveform_state.playhead = 0;
                }
            }
            ToolbarAction::ZoomIn => self.waveform_state.zoom_in(),
            ToolbarAction::ZoomOut => self.waveform_state.zoom_out(),
            ToolbarAction::ZoomToFit => {
                if let Some(el) = &self.edit_list {
                    self.waveform_state.zoom_to_fit(el.total_frames(), self.waveform_state.last_width);
                }
            }
            ToolbarAction::Delete => self.ripple_delete(),
            ToolbarAction::Crop => self.crop(),
        }
    }

    fn open_file(&mut self) {
        let file = rfd::FileDialog::new()
            .add_filter("Audio", &["wav", "aiff", "aif", "mp3", "flac", "m4a", "ogg"])
            .pick_file();

        let Some(path) = file else { return };

        match decode::decode_file(&path) {
            Ok(buffer) => {
                let buffer = Arc::new(buffer);
                let peaks = PeakData::compute(&buffer);
                let edit_list = EditList::from_length(buffer.num_frames);
                let total = edit_list.total_frames();

                let engine = PlaybackEngine::new(Arc::clone(&buffer), edit_list.clone());
                match engine {
                    Ok(engine) => self.playback_engine = Some(engine),
                    Err(e) => self.error_message = Some(format!("Playback init failed: {}", e)),
                }

                self.audio_buffer = Some(buffer);
                self.peak_data = Some(peaks);
                self.edit_list = Some(edit_list);
                self.file_path = Some(path);
                self.waveform_state = WaveformState::default();
                self.waveform_state.zoom_to_fit(total, self.waveform_state.last_width);
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
            Ok(()) => {}
            Err(e) => {
                self.error_message = Some(format!("Export failed: {}", e));
            }
        }
    }

    fn ripple_delete(&mut self) {
        let Some((start, end)) = self.waveform_state.selection else { return };
        let old_total = self.edit_list.as_ref().map(|el| el.total_frames()).unwrap_or(0);
        if let Some(el) = &mut self.edit_list {
            el.ripple_delete(start, end);
            self.waveform_state.selection = None;
            if self.waveform_state.playhead >= end {
                self.waveform_state.playhead -= end - start;
            } else if self.waveform_state.playhead > start {
                self.waveform_state.playhead = start;
            }
        }
        self.post_edit(old_total);
    }

    fn crop(&mut self) {
        let Some((start, end)) = self.waveform_state.selection else { return };
        let old_total = self.edit_list.as_ref().map(|el| el.total_frames()).unwrap_or(0);
        if let Some(el) = &mut self.edit_list {
            el.crop(start, end);
            self.waveform_state.selection = None;
            self.waveform_state.playhead = 0;
        }
        self.post_edit(old_total);
    }

    fn post_edit(&mut self, old_total: usize) {
        if let Some(el) = &self.edit_list {
            let new_total = el.total_frames();
            if old_total > 0 && new_total > 0 {
                let ratio = new_total as f64 / old_total as f64;
                self.waveform_state.zoom *= ratio;
                self.waveform_state.scroll_offset *= ratio;
            }
            let max_scroll = (new_total as f64 - self.waveform_state.last_width as f64 * self.waveform_state.zoom).max(0.0);
            self.waveform_state.scroll_offset = self.waveform_state.scroll_offset.min(max_scroll);
        }
        self.sync_playback_engine();
    }

    fn sync_playback_engine(&self) {
        if let (Some(engine), Some(el)) = (&self.playback_engine, &self.edit_list) {
            engine.set_edit_list(el.clone());
        }
    }
}
