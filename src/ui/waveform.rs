use egui::{Color32, Painter, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::audio::peaks::PeakData;
use crate::edit::EditList;

pub struct WaveformState {
    pub scroll_offset: f64,
    pub zoom: f64,
    pub selection: Option<(usize, usize)>,
    pub playhead: usize,
    drag_start: Option<usize>,
}

impl Default for WaveformState {
    fn default() -> Self {
        Self {
            scroll_offset: 0.0,
            zoom: 1.0,
            selection: None,
            playhead: 0,
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

    pub fn zoom_in(&mut self) {
        self.zoom = (self.zoom / 2.0).max(1.0);
    }

    pub fn zoom_out(&mut self) {
        self.zoom *= 2.0;
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
}

impl<'a> WaveformWidget<'a> {
    pub fn new(peaks: &'a PeakData, edit_list: &'a EditList, state: &'a mut WaveformState) -> Self {
        Self { peaks, edit_list, state }
    }
}

impl<'a> egui::Widget for WaveformWidget<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let available = ui.available_size();
        let (response, painter) = ui.allocate_painter(available, Sense::click_and_drag());
        let rect = response.rect;

        let total_frames = self.edit_list.total_frames();
        let width = rect.width();
        let height = rect.height();

        handle_input(ui, &response, rect, self.state, total_frames);

        painter.rect_filled(rect, 0.0, Color32::from_rgb(20, 20, 24));

        if total_frames == 0 || width <= 0.0 {
            return response;
        }

        let num_channels = self.peaks.channels();
        let channel_height = height / num_channels.max(1) as f32;

        let start_frame = self.state.scroll_offset.max(0.0) as usize;
        let end_frame = (self.state.scroll_offset + width as f64 * self.state.zoom) as usize;
        let end_frame = end_frame.min(total_frames);
        let num_pixels = width as usize;

        for ch in 0..num_channels {
            let ch_top = rect.top() + ch as f32 * channel_height;
            let ch_rect = Rect::from_min_size(
                Pos2::new(rect.left(), ch_top),
                Vec2::new(width, channel_height),
            );
            let center_y = ch_rect.center().y;
            let half_h = channel_height * 0.45;

            painter.line_segment(
                [Pos2::new(ch_rect.left(), center_y), Pos2::new(ch_rect.right(), center_y)],
                Stroke::new(1.0, Color32::from_rgb(50, 50, 60)),
            );

            if start_frame >= end_frame || num_pixels == 0 {
                continue;
            }

            let peaks = self.peaks.get_peaks(ch, start_frame, end_frame, num_pixels);

            for (px, (min_val, max_val)) in peaks.iter().enumerate() {
                let x = rect.left() + px as f32;
                let y_top = center_y - max_val * half_h;
                let y_bot = center_y - min_val * half_h;
                painter.line_segment(
                    [Pos2::new(x, y_top), Pos2::new(x, y_bot)],
                    Stroke::new(1.0, Color32::from_rgb(100, 180, 255)),
                );
            }
        }

        draw_selection(&painter, self.state, rect, total_frames);
        draw_playhead(&painter, self.state, rect);

        response
    }
}

fn handle_input(
    ui: &Ui,
    response: &Response,
    rect: Rect,
    state: &mut WaveformState,
    total_frames: usize,
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

    if response.drag_started() {
        if let Some(pos) = response.interact_pointer_pos() {
            let frame = state.x_to_frame(pos.x, &rect);
            state.drag_start = Some(frame);
            state.selection = None;
        }
    }

    if response.dragged() {
        if let (Some(start), Some(pos)) = (state.drag_start, response.interact_pointer_pos()) {
            let end = state.x_to_frame(pos.x, &rect);
            let (a, b) = if start <= end { (start, end) } else { (end, start) };
            if b > a {
                state.selection = Some((a, b));
            }
        }
    }

    if response.drag_stopped() {
        if state.selection.is_none() {
            if let Some(pos) = response.interact_pointer_pos() {
                let frame = state.x_to_frame(pos.x, &rect);
                state.playhead = frame.min(total_frames);
            }
        }
        state.drag_start = None;
    }

    if response.clicked() && state.drag_start.is_none() {
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

fn draw_playhead(painter: &Painter, state: &WaveformState, rect: Rect) {
    let x = state.frame_to_x(state.playhead, &rect);
    if x >= rect.left() && x <= rect.right() {
        painter.line_segment(
            [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
            Stroke::new(2.0, Color32::from_rgb(255, 220, 60)),
        );
    }
}
