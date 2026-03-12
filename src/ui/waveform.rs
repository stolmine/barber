use egui::{Color32, Painter, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::audio::peaks::PeakData;
use crate::edit::EditList;
use crate::ui::toolbar::ToolbarAction;

pub struct WaveformState {
    pub scroll_offset: f64,
    pub zoom: f64,
    pub selection: Option<(usize, usize)>,
    pub playhead: usize,
    pub last_width: f32,
    pub needs_fit: bool,
    pub phantom_playhead: Option<usize>,
    drag_start: Option<usize>,
}

impl Default for WaveformState {
    fn default() -> Self {
        Self {
            scroll_offset: 0.0,
            zoom: 1.0,
            selection: None,
            playhead: 0,
            last_width: 0.0,
            needs_fit: true,
            phantom_playhead: None,
            drag_start: None,
        }
    }
}

impl WaveformState {
    pub fn zoom_to_fit(&mut self, total_frames: usize, width: f32) {
        if width > 0.0 && total_frames > 0 {
            self.zoom = total_frames as f64 / width as f64;
            self.scroll_offset = 0.0;
        }
    }

    pub fn zoom_to_selection(&mut self, sel_start: usize, sel_end: usize, width: f32) {
        if width > 0.0 && sel_end > sel_start {
            self.zoom = (sel_end - sel_start) as f64 / width as f64;
            self.scroll_offset = sel_start as f64;
        }
    }

    pub fn zoom_in(&mut self) {
        self.zoom = (self.zoom / 2.0).max(1.0);
    }

    pub fn zoom_out(&mut self) {
        self.zoom *= 2.0;
    }

    pub fn ensure_visible(&mut self, frame: usize) {
        let visible_start = self.scroll_offset;
        let visible_end = self.scroll_offset + self.last_width as f64 * self.zoom;
        let frame_f = frame as f64;
        if frame_f < visible_start || frame_f > visible_end {
            self.scroll_offset = (frame_f - self.last_width as f64 * self.zoom * 0.2).max(0.0);
        }
    }

    fn frame_to_x(&self, frame: usize, rect: &Rect) -> f32 {
        let px = (frame as f64 - self.scroll_offset) / self.zoom;
        rect.left() + px as f32
    }

    fn x_to_frame(&self, x: f32, rect: &Rect) -> usize {
        let px = (x - rect.left()) as f64;
        let frame = self.scroll_offset + px * self.zoom;
        frame.max(0.0) as usize
    }
}

pub struct WaveformWidget<'a> {
    peaks: &'a PeakData,
    edit_list: &'a EditList,
    state: &'a mut WaveformState,
    sample_rate: u32,
    action: &'a mut Option<ToolbarAction>,
    has_clipboard: bool,
    audio_samples: Option<&'a [f32]>,
}

impl<'a> WaveformWidget<'a> {
    pub fn new(
        peaks: &'a PeakData,
        edit_list: &'a EditList,
        state: &'a mut WaveformState,
        sample_rate: u32,
        action: &'a mut Option<ToolbarAction>,
        has_clipboard: bool,
        audio_samples: Option<&'a [f32]>,
    ) -> Self {
        Self { peaks, edit_list, state, sample_rate, action, has_clipboard, audio_samples }
    }
}

impl<'a> egui::Widget for WaveformWidget<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let available = ui.available_size();
        let (response, painter) = ui.allocate_painter(available, Sense::click_and_drag());
        let rect = response.rect;
        const RULER_HEIGHT: f32 = 20.0;
        let waveform_rect = Rect::from_min_max(
            Pos2::new(rect.left(), rect.top()),
            Pos2::new(rect.right(), rect.bottom() - RULER_HEIGHT),
        );

        let total_frames = self.edit_list.total_frames();
        let width = rect.width();

        self.state.last_width = width;

        if self.state.needs_fit && total_frames > 0 && width > 0.0 {
            self.state.zoom_to_fit(total_frames, width);
            self.state.needs_fit = false;
        }

        handle_input(ui, &response, waveform_rect, self.state, total_frames, self.edit_list, self.audio_samples);

        let has_sel = self.state.selection.is_some();
        let has_clip = self.has_clipboard;
        response.context_menu(|ui| {
            if ui.add_enabled(has_sel, egui::Button::new("Cut")).clicked() {
                *self.action = Some(ToolbarAction::Cut);
                ui.close_menu();
            }
            if ui.add_enabled(has_sel, egui::Button::new("Copy")).clicked() {
                *self.action = Some(ToolbarAction::Copy);
                ui.close_menu();
            }
            if ui.add_enabled(has_clip, egui::Button::new("Paste")).clicked() {
                *self.action = Some(ToolbarAction::Paste);
                ui.close_menu();
            }
            ui.separator();
            if ui.add_enabled(has_sel, egui::Button::new("Gap Delete")).clicked() {
                *self.action = Some(ToolbarAction::GapDelete);
                ui.close_menu();
            }
            if ui.add_enabled(has_sel, egui::Button::new("Ripple Delete")).clicked() {
                *self.action = Some(ToolbarAction::RippleDelete);
                ui.close_menu();
            }
            if ui.add_enabled(has_sel, egui::Button::new("Crop")).clicked() {
                *self.action = Some(ToolbarAction::Crop);
                ui.close_menu();
            }
            if ui.add_enabled(has_sel, egui::Button::new("Duplicate")).clicked() {
                *self.action = Some(ToolbarAction::Duplicate);
                ui.close_menu();
            }
        });

        painter.rect_filled(rect, 0.0, Color32::from_rgb(20, 20, 24));
        draw_ruler(&painter, self.state, rect, rect.bottom(), self.sample_rate);

        if total_frames == 0 || width <= 0.0 {
            return response;
        }

        let num_channels = self.peaks.channels();
        let channel_height = waveform_rect.height() / num_channels.max(1) as f32;

        let start_frame = self.state.scroll_offset.max(0.0) as usize;
        let end_frame = (self.state.scroll_offset + width as f64 * self.state.zoom) as usize;
        let end_frame = end_frame.min(total_frames);
        let num_pixels = if self.state.zoom > 0.0 {
            (((end_frame - start_frame) as f64 / self.state.zoom) as usize).min(width as usize)
        } else {
            width as usize
        };

        for ch in 0..num_channels {
            let ch_top = waveform_rect.top() + ch as f32 * channel_height;
            let ch_rect = Rect::from_min_size(
                Pos2::new(waveform_rect.left(), ch_top),
                Vec2::new(width, channel_height),
            );
            let center_y = ch_rect.center().y;
            let half_h = channel_height * 0.5;

            painter.line_segment(
                [Pos2::new(ch_rect.left(), center_y), Pos2::new(ch_rect.right(), center_y)],
                Stroke::new(1.0, Color32::from_rgb(50, 50, 60)),
            );

            if start_frame >= end_frame || num_pixels == 0 {
                continue;
            }

            let peaks = self.peaks.get_peaks(ch, start_frame, end_frame, num_pixels);

            for (px, (min_val, max_val)) in peaks.iter().enumerate() {
                let px_frame_start = start_frame + (px as f64 * self.state.zoom) as usize;
                let px_frame_end = start_frame + ((px + 1) as f64 * self.state.zoom) as usize;
                let (lo, hi) = if self.edit_list.is_silence_range(px_frame_start, px_frame_end.max(px_frame_start + 1)) {
                    (0.0f32, 0.0f32)
                } else {
                    (*min_val, *max_val)
                };
                let x = waveform_rect.left() + px as f32;
                let y_top = (center_y - hi * half_h).max(ch_rect.top());
                let y_bot = (center_y - lo * half_h).min(ch_rect.bottom());
                painter.line_segment(
                    [Pos2::new(x, y_top), Pos2::new(x, y_bot)],
                    Stroke::new(1.0, Color32::from_rgb(100, 180, 255)),
                );
            }
        }

        for ch in 1..num_channels {
            let y = waveform_rect.top() + ch as f32 * channel_height;
            painter.line_segment(
                [Pos2::new(waveform_rect.left(), y), Pos2::new(waveform_rect.right(), y)],
                Stroke::new(1.0, Color32::from_rgb(70, 70, 80)),
            );
        }

        draw_selection(&painter, self.state, waveform_rect, total_frames);
        draw_phantom_playhead(&painter, self.state, waveform_rect, self.peaks, self.edit_list, num_channels, channel_height);
        draw_playhead(&painter, self.state, waveform_rect);

        response
    }
}

fn handle_input(
    ui: &Ui,
    response: &Response,
    rect: Rect,
    state: &mut WaveformState,
    total_frames: usize,
    edit_list: &EditList,
    audio_samples: Option<&[f32]>,
) {
    if response.hovered() {
        let scroll_delta = ui.input(|i| i.smooth_scroll_delta);
        let modifiers = ui.input(|i| i.modifiers);

        if modifiers.command {
            if scroll_delta.y != 0.0 {
                let mouse_x = ui.input(|i| i.pointer.hover_pos()).map(|p| p.x).unwrap_or(rect.center().x);
                let frame_at_cursor = state.x_to_frame(mouse_x, &rect);
                let factor = if scroll_delta.y > 0.0 { 0.8 } else { 1.25 };
                state.zoom = (state.zoom * factor).max(1.0);
                let new_x = state.frame_to_x(frame_at_cursor, &rect);
                let dx = (mouse_x - new_x) as f64;
                state.scroll_offset = (state.scroll_offset - dx * state.zoom).max(0.0);
            }
        } else {
            let hscroll = if modifiers.shift {
                scroll_delta.y
            } else {
                scroll_delta.x
            };
            if hscroll != 0.0 {
                let delta_frames = hscroll as f64 * state.zoom;
                state.scroll_offset = (state.scroll_offset - delta_frames).max(0.0);
            }
        }
    }

    let max_scroll = (total_frames as f64 - rect.width() as f64 * state.zoom).max(0.0);
    state.scroll_offset = state.scroll_offset.min(max_scroll);

    let primary_down = ui.input(|i| i.pointer.primary_down());
    let secondary_down = ui.input(|i| i.pointer.secondary_down());
    let _primary_released = ui.input(|i| i.pointer.primary_released());

    if response.drag_started() && primary_down && !secondary_down {
        if let Some(pos) = response.interact_pointer_pos() {
            let frame = state.x_to_frame(pos.x, &rect);
            state.drag_start = Some(frame);
            state.selection = None;
        }
    }

    if response.dragged() && state.drag_start.is_some() {
        if let (Some(start), Some(pos)) = (state.drag_start, response.interact_pointer_pos()) {
            let end = state.x_to_frame(pos.x, &rect);
            let (a, b) = if start <= end { (start, end) } else { (end, start) };
            if b > a {
                state.selection = Some((a, b));
            }
        }
    }

    if response.drag_stopped() && state.drag_start.is_some() {
        if state.selection.is_none() {
            if let Some(pos) = response.interact_pointer_pos() {
                let frame = state.x_to_frame(pos.x, &rect);
                state.playhead = frame.min(total_frames);
            }
        }
        state.drag_start = None;
        if let (Some((start, end)), Some(samples)) = (state.selection, audio_samples) {
            let snapped_start = crate::audio::zero_crossing::find_nearest_zero_crossing(samples, edit_list, start, 512);
            let snapped_end = crate::audio::zero_crossing::find_nearest_zero_crossing(samples, edit_list, end, 512);
            if snapped_start < snapped_end {
                state.selection = Some((snapped_start, snapped_end));
            }
        }
    }

    if response.clicked_by(egui::PointerButton::Primary) && state.drag_start.is_none() {
        if let Some(pos) = response.interact_pointer_pos() {
            let frame = state.x_to_frame(pos.x, &rect);
            state.playhead = frame.min(total_frames);
            state.selection = None;
        }
    }
}

fn draw_selection(painter: &Painter, state: &WaveformState, rect: Rect, total_frames: usize) {
    let Some((sel_start, sel_end)) = state.selection else { return };
    if sel_start >= sel_end || sel_start > total_frames {
        return;
    }
    let x_start = state.frame_to_x(sel_start, &rect).max(rect.left());
    let x_end = state.frame_to_x(sel_end, &rect).min(rect.right());
    if x_end <= x_start {
        return;
    }
    let sel_rect = Rect::from_min_max(
        Pos2::new(x_start, rect.top()),
        Pos2::new(x_end, rect.bottom()),
    );
    painter.rect_filled(sel_rect, 0.0, Color32::from_rgba_unmultiplied(100, 180, 255, 40));
    painter.rect_stroke(sel_rect, 0.0, Stroke::new(1.0, Color32::from_rgba_unmultiplied(100, 180, 255, 120)), StrokeKind::Middle);
}

fn draw_phantom_playhead(
    painter: &Painter,
    state: &WaveformState,
    rect: Rect,
    peaks: &PeakData,
    edit_list: &EditList,
    num_channels: usize,
    channel_height: f32,
) {
    let Some(frame) = state.phantom_playhead else { return };
    let x = state.frame_to_x(frame, &rect);
    if x < rect.left() || x > rect.right() {
        return;
    }
    let bg_color = Color32::from_rgba_unmultiplied(255, 220, 60, 50);
    let wave_color = Color32::from_rgb(255, 120, 0);

    for ch in 0..num_channels.max(1) {
        let ch_top = rect.top() + ch as f32 * channel_height;
        let center_y = ch_top + channel_height * 0.5;
        let half_h = channel_height * 0.5;

        let peak = peaks.get_peaks(ch, frame, frame + 1, 1);
        let (lo, hi) = peak.first().copied().unwrap_or((0.0, 0.0));
        let (lo, hi) = if edit_list.is_silence_range(frame, frame + 1) {
            (0.0, 0.0)
        } else {
            (lo, hi)
        };
        let wave_top = (center_y - hi * half_h).max(ch_top);
        let wave_bot = (center_y - lo * half_h).min(ch_top + channel_height);

        if wave_top > ch_top {
            painter.line_segment(
                [Pos2::new(x, ch_top), Pos2::new(x, wave_top)],
                Stroke::new(1.0, bg_color),
            );
        }
        if wave_bot > wave_top {
            painter.line_segment(
                [Pos2::new(x, wave_top), Pos2::new(x, wave_bot)],
                Stroke::new(1.0, wave_color),
            );
        }
        if wave_bot < ch_top + channel_height {
            painter.line_segment(
                [Pos2::new(x, wave_bot), Pos2::new(x, ch_top + channel_height)],
                Stroke::new(1.0, bg_color),
            );
        }
    }
}

fn draw_playhead(painter: &Painter, state: &WaveformState, rect: Rect) {
    let x = state.frame_to_x(state.playhead, &rect);
    if x >= rect.left() && x <= rect.right() {
        painter.line_segment(
            [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
            Stroke::new(2.0, Color32::from_rgb(255, 220, 60)),
        );
    }
}

fn draw_ruler(painter: &Painter, state: &WaveformState, rect: Rect, ruler_bottom: f32, sample_rate: u32) {
    let visible_frames = rect.width() as f64 * state.zoom;
    let seconds_per_pixel = state.zoom / sample_rate as f64;

    let intervals = [0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0, 30.0, 60.0];
    let interval = intervals.iter().copied()
        .find(|&iv| iv / seconds_per_pixel >= 80.0)
        .unwrap_or(60.0);

    let start_sec = state.scroll_offset / sample_rate as f64;
    let end_sec = (state.scroll_offset + visible_frames) / sample_rate as f64;
    let first_tick = (start_sec / interval).floor() * interval;

    let font = egui::FontId::monospace(10.0);
    let color = Color32::from_rgb(160, 160, 170);
    let tick_color = Color32::from_rgb(80, 80, 90);

    let mut t = first_tick;
    while t <= end_sec {
        let frame = (t * sample_rate as f64) as usize;
        let x = state.frame_to_x(frame, &rect);
        if x >= rect.left() && x <= rect.right() {
            let ruler_top = ruler_bottom - 20.0;
            painter.line_segment(
                [Pos2::new(x, ruler_top), Pos2::new(x, ruler_top + 6.0)],
                Stroke::new(1.0, tick_color),
            );
            let label = if t >= 60.0 {
                format!("{}:{:02}", t as u32 / 60, t as u32 % 60)
            } else if interval < 1.0 {
                format!("{:.1}", t)
            } else {
                format!("{:.0}", t)
            };
            painter.text(
                Pos2::new(x + 3.0, ruler_top + 5.0),
                egui::Align2::LEFT_TOP,
                label,
                font.clone(),
                color,
            );
        }
        t += interval;
    }
}
