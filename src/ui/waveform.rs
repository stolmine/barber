use egui::{Color32, Mesh, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, Vec2};
use egui::epaint::{PathShape, PathStroke};

use crate::audio::peaks::PeakData;
use crate::edit::EditList;
use crate::theme::WaveformTheme;
use crate::ui::toolbar::ToolbarAction;

pub struct WaveformState {
    pub scroll_offset: f64,
    pub zoom: f64,
    pub vertical_zoom: f32,
    pub selection: Option<(usize, usize)>,
    pub playhead: usize,
    pub last_width: f32,
    pub needs_fit: bool,
    pub phantom_playhead: Option<usize>,
    pub in_point: usize,
    pub out_point: usize,
    drag_start: Option<usize>,
}

impl Default for WaveformState {
    fn default() -> Self {
        Self {
            scroll_offset: 0.0,
            zoom: 1.0,
            vertical_zoom: 1.0,
            selection: None,
            playhead: 0,
            last_width: 0.0,
            needs_fit: true,
            phantom_playhead: None,
            in_point: 0,
            out_point: 0,
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

    pub fn center_on(&mut self, frame: usize, total_frames: usize) {
        let view_frames = self.last_width as f64 * self.zoom;
        let max_scroll = (total_frames as f64 - view_frames).max(0.0);
        let target = (frame as f64 - view_frames * 0.5).clamp(0.0, max_scroll);
        self.scroll_offset += (target - self.scroll_offset) * 0.15;
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
    theme: &'a WaveformTheme,
    snap_to_zero: bool,
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
        theme: &'a WaveformTheme,
        snap_to_zero: bool,
    ) -> Self {
        Self { peaks, edit_list, state, sample_rate, action, has_clipboard, audio_samples, theme, snap_to_zero }
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
        let width = waveform_rect.width();

        self.state.last_width = width;

        if self.state.needs_fit && total_frames > 0 && width > 0.0 {
            self.state.zoom_to_fit(total_frames, width);
            self.state.needs_fit = false;
        }

        handle_input(ui, &response, waveform_rect, self.state, total_frames, self.edit_list, self.audio_samples, self.snap_to_zero);

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
            ui.separator();
            if ui.add_enabled(true, egui::Button::new("Reverse")).clicked() {
                *self.action = Some(ToolbarAction::Reverse);
                ui.close_menu();
            }
            if ui.add_enabled(has_sel, egui::Button::new("Normalize")).clicked() {
                *self.action = Some(ToolbarAction::Normalize);
                ui.close_menu();
            }
            if ui.add_enabled(has_sel, egui::Button::new("Remove DC Offset")).clicked() {
                *self.action = Some(ToolbarAction::RemoveDC);
                ui.close_menu();
            }
        });

        let theme = self.theme;
        painter.rect_filled(rect, 0.0, theme.background);
        draw_ruler(&painter, self.state, waveform_rect, rect.bottom(), self.sample_rate, theme);

        let num_channels = self.peaks.channels();
        let channel_height = waveform_rect.height() / num_channels.max(1) as f32;

        if total_frames == 0 || width <= 0.0 {
            return response;
        }

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
            let half_h = channel_height * 0.5 * self.state.vertical_zoom;

            painter.line_segment(
                [Pos2::new(ch_rect.left(), center_y), Pos2::new(ch_rect.right(), center_y)],
                Stroke::new(1.0, theme.center_line),
            );

            if start_frame >= end_frame || num_pixels == 0 {
                continue;
            }

            let mut top_points: Vec<Pos2> = Vec::with_capacity(num_pixels);
            let mut bot_points: Vec<Pos2> = Vec::with_capacity(num_pixels);

            for px in 0..num_pixels {
                let px_frame_start = start_frame + (px as f64 * self.state.zoom) as usize;
                let px_frame_end = start_frame + ((px + 1) as f64 * self.state.zoom) as usize;
                let px_frame_end = px_frame_end.max(px_frame_start + 1);
                let mut lo = f32::INFINITY;
                let mut hi = f32::NEG_INFINITY;
                self.edit_list.for_each_source_range(px_frame_start, px_frame_end, |src_start, src_end, gain| {
                    let (mn, mx) = self.peaks.get_peaks_for_source_range(ch, src_start, src_end);
                    lo = lo.min(mn * gain);
                    hi = hi.max(mx * gain);
                });
                let (lo, hi) = if lo == f32::INFINITY { (0.0f32, 0.0f32) } else { (lo, hi) };
                let x = waveform_rect.left() + px as f32;
                let y_top = (center_y - hi * half_h).max(ch_rect.top());
                let y_bot = (center_y - lo * half_h).min(ch_rect.bottom());
                top_points.push(Pos2::new(x, y_top));
                bot_points.push(Pos2::new(x, y_bot));
            }

            let n = top_points.len();
            if n < 2 { continue; }

            let mut mesh = Mesh::default();
            mesh.reserve_vertices(n * 2);
            mesh.reserve_triangles((n - 1) * 2);
            let fill_color = theme.waveform_fill;
            for i in 0..n {
                mesh.colored_vertex(top_points[i], fill_color);
                mesh.colored_vertex(bot_points[i], fill_color);
            }
            for i in 0..(n - 1) {
                let v = (i * 2) as u32;
                mesh.add_triangle(v, v + 1, v + 2);
                mesh.add_triangle(v + 1, v + 3, v + 2);
            }
            painter.add(Shape::mesh(mesh));

            if theme.waveform_stroke_width > 0.0 {
                let stroke = PathStroke::new(theme.waveform_stroke_width, theme.waveform_stroke);
                painter.add(Shape::Path(PathShape {
                    points: top_points,
                    closed: false,
                    fill: Color32::TRANSPARENT,
                    stroke: stroke.clone(),
                }));
                painter.add(Shape::Path(PathShape {
                    points: bot_points,
                    closed: false,
                    fill: Color32::TRANSPARENT,
                    stroke,
                }));
            }
        }

        for ch in 1..num_channels {
            let y = waveform_rect.top() + ch as f32 * channel_height;
            painter.line_segment(
                [Pos2::new(waveform_rect.left(), y), Pos2::new(waveform_rect.right(), y)],
                Stroke::new(1.0, theme.channel_separator),
            );
        }

        draw_selection(&painter, self.state, waveform_rect, total_frames, theme);
        draw_in_out_points(&painter, self.state, waveform_rect, total_frames, theme);
        draw_phantom_playhead(&painter, self.state, waveform_rect, self.peaks, self.edit_list, num_channels, channel_height, theme);
        draw_playhead(&painter, self.state, waveform_rect, theme);

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
    snap_to_zero: bool,
) {
    if response.hovered() {
        let pinch_delta = ui.input(|i| i.zoom_delta());
        let modifiers = ui.input(|i| i.modifiers);

        if pinch_delta != 1.0 {
            if modifiers.shift {
                state.vertical_zoom = (state.vertical_zoom * pinch_delta).clamp(0.5, 20.0);
            } else {
                let mouse_x = ui.input(|i| i.pointer.hover_pos()).map(|p| p.x).unwrap_or(rect.center().x);
                let frame_at_cursor = state.x_to_frame(mouse_x, &rect);
                state.zoom = (state.zoom / pinch_delta as f64).max(1.0);
                let new_x = state.frame_to_x(frame_at_cursor, &rect);
                let dx = (mouse_x - new_x) as f64;
                state.scroll_offset = (state.scroll_offset - dx * state.zoom).max(0.0);
            }
        }

        let scroll_delta = ui.input(|i| i.smooth_scroll_delta);

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

    let modifiers = ui.input(|i| i.modifiers);

    if response.drag_started() && primary_down && !secondary_down {
        if let Some(pos) = response.interact_pointer_pos() {
            let frame = state.x_to_frame(pos.x, &rect);
            if modifiers.shift {
                let anchor = if let Some((sel_start, sel_end)) = state.selection {
                    let mid = (sel_start + sel_end) / 2;
                    if frame < mid { sel_end } else { sel_start }
                } else {
                    state.playhead
                };
                state.drag_start = Some(anchor);
                let (a, b) = if anchor <= frame { (anchor, frame) } else { (frame, anchor) };
                if b > a { state.selection = Some((a, b)); }
            } else {
                state.drag_start = Some(frame);
                state.selection = None;
            }
        }
    }

    if response.dragged() && state.drag_start.is_some() {
        if let Some(start) = state.drag_start {
            let end = if let Some(pos) = response.interact_pointer_pos() {
                let scroll_margin = 20.0;
                let scroll_speed = 8.0 * state.zoom;
                if pos.x < rect.left() + scroll_margin {
                    state.scroll_offset = (state.scroll_offset - scroll_speed).max(0.0);
                } else if pos.x > rect.right() - scroll_margin {
                    let max_scroll = (total_frames as f64 - rect.width() as f64 * state.zoom).max(0.0);
                    state.scroll_offset = (state.scroll_offset + scroll_speed).min(max_scroll);
                }
                state.x_to_frame(pos.x, &rect).min(total_frames)
            } else {
                start
            };
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
        if let (Some((start, end)), Some(samples)) = (snap_to_zero.then_some(state.selection).flatten(), audio_samples) {
            let snapped_start = crate::audio::zero_crossing::find_nearest_zero_crossing(samples, edit_list, start, 512);
            let snapped_end = crate::audio::zero_crossing::find_nearest_zero_crossing(samples, edit_list, end, 512);
            if snapped_start < snapped_end {
                state.selection = Some((snapped_start, snapped_end));
            }
        }
    }

    if response.double_clicked() && total_frames > 0 {
        state.selection = Some((0, total_frames));
    } else if response.clicked_by(egui::PointerButton::Primary) && state.drag_start.is_none() {
        if let Some(pos) = response.interact_pointer_pos() {
            let frame = state.x_to_frame(pos.x, &rect).min(total_frames);
            if modifiers.shift {
                let anchor = if let Some((sel_start, sel_end)) = state.selection {
                    let mid = (sel_start + sel_end) / 2;
                    if frame < mid { sel_end } else { sel_start }
                } else {
                    state.playhead
                };
                let (a, b) = if anchor <= frame { (anchor, frame) } else { (frame, anchor) };
                if b > a { state.selection = Some((a, b)); }
            } else {
                state.playhead = frame;
                state.selection = None;
            }
        }
    }
}

fn draw_selection(painter: &Painter, state: &WaveformState, rect: Rect, total_frames: usize, theme: &WaveformTheme) {
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
    painter.rect_filled(sel_rect, 0.0, theme.selection_fill);
    painter.rect_stroke(sel_rect, 0.0, Stroke::new(1.0, theme.selection_stroke), StrokeKind::Middle);
}

fn draw_in_out_points(painter: &Painter, state: &WaveformState, rect: Rect, total_frames: usize, theme: &WaveformTheme) {
    let draw_marker = |frame: usize, color: Color32| {
        let x = state.frame_to_x(frame, &rect);
        if x >= rect.left() && x <= rect.right() {
            let dash_len = 4.0;
            let gap_len = 3.0;
            let mut y = rect.top();
            while y < rect.bottom() {
                let end_y = (y + dash_len).min(rect.bottom());
                painter.line_segment(
                    [Pos2::new(x, y), Pos2::new(x, end_y)],
                    Stroke::new(1.0, color),
                );
                y += dash_len + gap_len;
            }
        }
    };
    if state.in_point > 0 {
        draw_marker(state.in_point, theme.in_point);
    }
    if state.out_point > 0 && state.out_point < total_frames {
        draw_marker(state.out_point, theme.out_point);
    }
}

fn draw_phantom_playhead(
    painter: &Painter,
    state: &WaveformState,
    rect: Rect,
    peaks: &PeakData,
    edit_list: &EditList,
    num_channels: usize,
    channel_height: f32,
    theme: &WaveformTheme,
) {
    let Some(frame) = state.phantom_playhead else { return };
    let x = state.frame_to_x(frame, &rect);
    if x < rect.left() || x > rect.right() {
        return;
    }
    let bg_color = theme.phantom_bg;
    let wave_color = theme.phantom_wave;

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

fn draw_playhead(painter: &Painter, state: &WaveformState, rect: Rect, theme: &WaveformTheme) {
    let x = state.frame_to_x(state.playhead, &rect);
    if x >= rect.left() && x <= rect.right() {
        painter.line_segment(
            [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
            Stroke::new(2.0, theme.playhead),
        );
    }
}

fn draw_ruler(painter: &Painter, state: &WaveformState, rect: Rect, ruler_bottom: f32, sample_rate: u32, theme: &WaveformTheme) {
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
    let color = theme.ruler_text;
    let tick_color = theme.ruler_tick;

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

