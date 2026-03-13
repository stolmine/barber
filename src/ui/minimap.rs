use egui::{Color32, Pos2, Rect, Sense, Stroke, Ui, Vec2};

use crate::audio::peaks::PeakData;
use crate::edit::EditList;
use crate::ui::waveform::WaveformState;

const MINIMAP_HEIGHT: f32 = 32.0;
const EDGE_HOTZONE: f32 = 5.0;
const DIM_COLOR: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 120);

#[derive(Clone, Copy, PartialEq)]
pub enum MinimapDrag {
    None,
    Pan { offset_frames: f64, start_y: f32, start_zoom: f64 },
    LeftEdge { right_anchor: f64, grab_offset: f64 },
    RightEdge { left_anchor: f64, grab_offset: f64 },
}

pub fn minimap_ui(
    ui: &mut Ui,
    peaks: &PeakData,
    edit_list: &EditList,
    state: &mut WaveformState,
    drag: &mut MinimapDrag,
) {
    let available_width = ui.available_width();
    let desired = Vec2::new(available_width, MINIMAP_HEIGHT);
    let (rect, response) = ui.allocate_exact_size(desired, Sense::click_and_drag());
    let painter = ui.painter_at(rect);

    let total = edit_list.total_frames();
    if total == 0 || rect.width() <= 0.0 {
        painter.rect_filled(rect, 0.0, Color32::from_gray(20));
        return;
    }

    painter.rect_filled(rect, 0.0, Color32::from_rgb(16, 16, 20));

    // Draw full waveform compressed to fit
    let overview_zoom = total as f64 / rect.width() as f64;
    let num_channels = peaks.channels();
    let channel_height = rect.height() / num_channels.max(1) as f32;
    let num_pixels = rect.width() as usize;

    for ch in 0..num_channels {
        let ch_top = rect.top() + ch as f32 * channel_height;
        let center_y = ch_top + channel_height * 0.5;
        let half_h = channel_height * 0.5;

        for px in 0..num_pixels {
            let px_start = (px as f64 * overview_zoom) as usize;
            let px_end = (((px + 1) as f64 * overview_zoom) as usize).max(px_start + 1);
            let mut lo = f32::INFINITY;
            let mut hi = f32::NEG_INFINITY;
            edit_list.for_each_source_range(px_start, px_end, |src_start, src_end, gain| {
                let (mn, mx) = peaks.get_peaks_for_source_range(ch, src_start, src_end);
                lo = lo.min(mn * gain);
                hi = hi.max(mx * gain);
            });
            if lo == f32::INFINITY { continue; }
            let x = rect.left() + px as f32;
            let y_top = (center_y - hi * half_h).max(ch_top);
            let y_bot = (center_y - lo * half_h).min(ch_top + channel_height);
            painter.line_segment(
                [Pos2::new(x, y_top), Pos2::new(x, y_bot)],
                Stroke::new(1.0, Color32::from_rgb(70, 130, 190)),
            );
        }
    }

    // Viewport rectangle
    let fpx = rect.width() as f64 / total as f64;
    let view_frames = state.last_width as f64 * state.zoom;
    let vp_left = (rect.left() + (state.scroll_offset * fpx) as f32).max(rect.left());
    let vp_right = (vp_left + (view_frames * fpx) as f32).min(rect.right());

    // Dim outside viewport
    if vp_left > rect.left() {
        painter.rect_filled(
            Rect::from_min_max(rect.min, Pos2::new(vp_left, rect.max.y)),
            0.0, DIM_COLOR,
        );
    }
    if vp_right < rect.right() {
        painter.rect_filled(
            Rect::from_min_max(Pos2::new(vp_right, rect.min.y), rect.max),
            0.0, DIM_COLOR,
        );
    }

    let vp_rect = Rect::from_min_max(
        Pos2::new(vp_left, rect.top()),
        Pos2::new(vp_right, rect.bottom()),
    );
    painter.rect_stroke(vp_rect, 0.0, Stroke::new(1.0, Color32::from_gray(200)), egui::StrokeKind::Inside);

    // Playhead
    let ph_x = rect.left() + (state.playhead as f64 * fpx) as f32;
    if ph_x >= rect.left() && ph_x <= rect.right() {
        painter.line_segment(
            [Pos2::new(ph_x, rect.top()), Pos2::new(ph_x, rect.bottom())],
            Stroke::new(1.0, Color32::from_rgb(255, 220, 60)),
        );
    }

    // Helper: convert minimap pixel x to frame
    let x_to_frame = |x: f32| -> f64 {
        ((x - rect.left()) as f64 / fpx).clamp(0.0, total as f64)
    };

    // Cursor hints
    let hover_pos = ui.input(|i| i.pointer.hover_pos());
    if let Some(pos) = hover_pos {
        if rect.contains(pos) {
            match drag {
                MinimapDrag::LeftEdge { .. } | MinimapDrag::RightEdge { .. } => {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                }
                MinimapDrag::Pan { .. } => {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                }
                MinimapDrag::None => {
                    let inside_vp = pos.x >= vp_left && pos.x <= vp_right;
                    let on_edge = (pos.x - vp_left).abs() < EDGE_HOTZONE
                        || (pos.x - vp_right).abs() < EDGE_HOTZONE;
                    if inside_vp && on_edge {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                    } else if inside_vp {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                    }
                }
            }
        }
    }

    // Detect press on first frame (before drag movement) to lock drag mode
    let primary_pressed = ui.input(|i| i.pointer.primary_pressed());
    if primary_pressed && *drag == MinimapDrag::None {
        if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
            if rect.contains(pos) {
                let inside = pos.x >= vp_left && pos.x <= vp_right;
                let on_left = inside && (pos.x - vp_left).abs() < EDGE_HOTZONE;
                let on_right = inside && (pos.x - vp_right).abs() < EDGE_HOTZONE;

                if on_left {
                    let right_frame = state.scroll_offset + view_frames;
                    let cursor_frame = x_to_frame(pos.x);
                    let grab_offset = cursor_frame - state.scroll_offset;
                    *drag = MinimapDrag::LeftEdge { right_anchor: right_frame, grab_offset };
                } else if on_right {
                    let cursor_frame = x_to_frame(pos.x);
                    let right_frame = state.scroll_offset + view_frames;
                    let grab_offset = cursor_frame - right_frame;
                    *drag = MinimapDrag::RightEdge { left_anchor: state.scroll_offset, grab_offset };
                } else if inside {
                    let click_frame = x_to_frame(pos.x);
                    let offset = click_frame - state.scroll_offset;
                    *drag = MinimapDrag::Pan { offset_frames: offset, start_y: pos.y, start_zoom: state.zoom };
                } else {
                    // Click outside viewport: jump to center on click
                    let click_frame = x_to_frame(pos.x);
                    state.scroll_offset = (click_frame - view_frames * 0.5).max(0.0);
                    let max_scroll = (total as f64 - view_frames).max(0.0);
                    state.scroll_offset = state.scroll_offset.min(max_scroll);
                    *drag = MinimapDrag::Pan { offset_frames: view_frames * 0.5, start_y: pos.y, start_zoom: state.zoom };
                }
            }
        }
    }

    // Drag update
    if response.dragged() {
        if let Some(pos) = response.interact_pointer_pos() {
            let cursor_frame = x_to_frame(pos.x);

            match *drag {
                MinimapDrag::Pan { offset_frames, start_y, start_zoom } => {
                    // Vertical: zoom (drag down = zoom in, up = zoom out)
                    let dy = pos.y - start_y;
                    let zoom_factor = (1.0 + dy as f64 * 0.02).max(0.1);
                    state.zoom = (start_zoom / zoom_factor).max(1.0);

                    // Horizontal: pan
                    let current_view = state.last_width as f64 * state.zoom;
                    state.scroll_offset = (cursor_frame - offset_frames).max(0.0);
                    let max_scroll = (total as f64 - current_view).max(0.0);
                    state.scroll_offset = state.scroll_offset.min(max_scroll);
                }
                MinimapDrag::LeftEdge { right_anchor, grab_offset } => {
                    // New left edge = cursor position minus the grab offset
                    let new_left = (cursor_frame - grab_offset).clamp(0.0, right_anchor - state.last_width as f64);
                    let new_view = right_anchor - new_left;
                    state.zoom = (new_view / state.last_width as f64).max(1.0);
                    state.scroll_offset = new_left;
                }
                MinimapDrag::RightEdge { left_anchor, grab_offset } => {
                    // New right edge = cursor position minus the grab offset
                    let new_right = (cursor_frame - grab_offset).clamp(left_anchor + state.last_width as f64, total as f64);
                    let new_view = new_right - left_anchor;
                    state.zoom = (new_view / state.last_width as f64).max(1.0);
                    state.scroll_offset = left_anchor;
                }
                MinimapDrag::None => {}
            }
        }
    }

    let primary_released = ui.input(|i| i.pointer.primary_released());
    if response.drag_stopped() || primary_released {
        *drag = MinimapDrag::None;
    }

    // Double-click: zoom to fit
    let dbl = ui.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary));
    if dbl && hover_pos.map_or(false, |p| rect.contains(p)) {
        state.zoom_to_fit(total, state.last_width);
    }
}
