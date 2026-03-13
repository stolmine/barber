use crate::audio::levels::AudioLevels;
use crate::theme::MeterTheme;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToolbarAction {
    OpenFile,
    Export,
    Play,
    Pause,
    Stop,
    ZoomIn,
    ZoomOut,
    ZoomToFit,
    GapDelete,
    RippleDelete,
    Crop,
    Cut,
    Copy,
    Paste,
    Duplicate,
    Undo,
    Redo,
    PlaySelection,
    ToggleLoop,
    ToggleFollow,
    Reverse,
    Normalize,
    ToggleFade,
    RemoveDC,
    SelectAll,
    SetInPoint,
    SetOutPoint,
    GoToInPoint,
    GoToOutPoint,
    GoToStart,
    GoToEnd,
    NudgeLeft,
    NudgeRight,
    VolumeUp,
    VolumeDown,
    Quit,
    FadeInLinear,
    FadeInExponential,
    FadeInLogarithmic,
    FadeInSCurve,
    FadeOutLinear,
    FadeOutExponential,
    FadeOutLogarithmic,
    FadeOutSCurve,
    VerticalZoomIn,
    VerticalZoomOut,
    VerticalZoomReset,
    ChangeSpeed,
}

impl ToolbarAction {
    pub fn falls_back_to_full_file(&self) -> bool {
        matches!(self, Self::Reverse | Self::Normalize | Self::RemoveDC)
    }
}

pub fn toolbar_ui(
    ui: &mut egui::Ui,
    is_playing: bool,
    has_file: bool,
    is_loop: bool,
    is_following: bool,
) -> Option<ToolbarAction> {
    let mut action = None;

    ui.horizontal(|ui| {
        let play_pause_label = if is_playing { "Pause" } else { "Play" };
        let play_pause_action = if is_playing { ToolbarAction::Pause } else { ToolbarAction::Play };
        if ui.add_enabled(has_file, egui::Button::new(play_pause_label)).clicked() {
            action = Some(play_pause_action);
        }

        if ui.add_enabled(has_file, egui::Button::new("Stop")).clicked() {
            action = Some(ToolbarAction::Stop);
        }

        if ui.add_enabled(has_file, egui::SelectableLabel::new(is_loop, "Loop")).clicked() {
            action = Some(ToolbarAction::ToggleLoop);
        }

        if ui.add_enabled(has_file, egui::SelectableLabel::new(is_following, "Follow")).clicked() {
            action = Some(ToolbarAction::ToggleFollow);
        }
    });

    action
}

pub const DB_TICKS: &[(f32, &str)] = &[
    (1.0, "0"),
    (0.708, "-3"),
    (0.501, "-6"),
    (0.316, "-10"),
    (0.178, "-15"),
    (0.100, "-20"),
    (0.032, "-30"),
    (0.010, "-40"),
];

pub fn meter_panel_ui(ui: &mut egui::Ui, levels: &AudioLevels, theme: &MeterTheme) {
    let full_height = ui.available_height();
    let (peak_l, peak_r) = levels.smoothed_peaks();

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;

        // Fader
        let mut vol = levels.volume();
        let response = ui.scope(|ui| {
            ui.spacing_mut().slider_width = full_height;
            let slider = egui::Slider::new(&mut vol, 0.0..=2.0)
                .show_value(false)
                .vertical()
                .clamping(egui::SliderClamping::Always);
            ui.add(slider)
        }).inner;
        if response.changed() {
            levels.set_volume(vol);
        }
        let dbl = ui.input(|i| {
            i.pointer.button_double_clicked(egui::PointerButton::Primary)
        });
        if dbl && response.rect.contains(ui.input(|i| i.pointer.interact_pos().unwrap_or_default())) {
            levels.set_volume(1.0);
        }

        // Unity gain notch
        let unity_frac = 1.0 / 2.0; // 1.0 in 0..2 range
        let notch_y = response.rect.max.y - response.rect.height() * unity_frac;
        ui.painter().line_segment(
            [
                egui::pos2(response.rect.min.x, notch_y),
                egui::pos2(response.rect.max.x, notch_y),
            ],
            egui::Stroke::new(1.0, theme.unity_notch),
        );

        ui.separator();

        // Meters + ruler
        draw_vertical_meter(ui, peak_l, full_height, theme);
        draw_vertical_meter(ui, peak_r, full_height, theme);
        draw_db_ruler(ui, full_height, theme);
    });
}

fn draw_vertical_meter(ui: &mut egui::Ui, level: f32, height: f32, theme: &MeterTheme) {
    let desired = egui::vec2(10.0, height);
    let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, 1.0, theme.background);

    let clamped = level.clamp(0.0, 1.0);
    if clamped > 0.0 {
        let fill_height = rect.height() * clamped;
        let fill = egui::Rect::from_min_max(
            egui::pos2(rect.min.x, rect.max.y - fill_height),
            rect.max,
        );

        let green = theme.green;
        let yellow = theme.yellow;
        let red = theme.red;

        let thresh_yellow = rect.max.y - rect.height() * 0.7;
        let thresh_red = rect.max.y - rect.height() * 0.9;

        // Green zone
        let green_top = fill.min.y.max(thresh_yellow);
        if green_top < rect.max.y {
            painter.rect_filled(
                egui::Rect::from_min_max(egui::pos2(fill.min.x, green_top), fill.max),
                0.0, green,
            );
        }
        // Yellow zone
        if fill.min.y < thresh_yellow {
            let yellow_top = fill.min.y.max(thresh_red);
            painter.rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(fill.min.x, yellow_top),
                    egui::pos2(fill.max.x, thresh_yellow),
                ),
                0.0, yellow,
            );
        }
        // Red zone
        if fill.min.y < thresh_red {
            painter.rect_filled(
                egui::Rect::from_min_max(
                    fill.min,
                    egui::pos2(fill.max.x, thresh_red),
                ),
                0.0, red,
            );
        }
    }
}

const GAIN_TICKS: &[(f32, &str)] = &[
    (24.0, "+24"),
    (18.0, "+18"),
    (12.0, "+12"),
    (6.0, "+6"),
    (0.0, "0"),
    (-6.0, "-6"),
    (-12.0, "-12"),
    (-18.0, "-18"),
    (-24.0, "-24"),
];

fn draw_gain_ruler(ui: &mut egui::Ui, height: f32, theme: &MeterTheme) {
    let desired = egui::vec2(24.0, height);
    let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let font = egui::FontId::monospace(8.0);
    let color = theme.ruler_text;
    let tick_color = theme.ruler_tick;
    let half_line = 4.0;

    for &(db, label) in GAIN_TICKS {
        let frac = (db + 24.0) / 48.0;
        let y = rect.max.y - rect.height() * frac;
        let label_y = y.clamp(rect.min.y + half_line, rect.max.y - half_line);
        painter.line_segment(
            [egui::pos2(rect.max.x - 4.0, y), egui::pos2(rect.max.x, y)],
            egui::Stroke::new(1.0, tick_color),
        );
        painter.text(
            egui::pos2(rect.min.x, label_y),
            egui::Align2::LEFT_CENTER,
            label,
            font.clone(),
            color,
        );
    }
}

pub fn gain_panel_ui(ui: &mut egui::Ui, gain_db: &mut f32, theme: &MeterTheme) -> (bool, bool) {
    let full_height = ui.available_height();
    let mut changed = false;
    let mut drag_stopped = false;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;

        draw_gain_ruler(ui, full_height, theme);

        let response = ui.scope(|ui| {
            ui.spacing_mut().slider_width = full_height;
            let slider = egui::Slider::new(gain_db, -24.0..=24.0)
                .show_value(false)
                .vertical()
                .clamping(egui::SliderClamping::Always);
            ui.add(slider)
        }).inner;

        if response.changed() {
            changed = true;
        }
        if response.drag_stopped() {
            drag_stopped = true;
        }

        let unity_frac = 24.0 / 48.0;
        let notch_y = response.rect.max.y - response.rect.height() * unity_frac;
        ui.painter().line_segment(
            [
                egui::pos2(response.rect.min.x, notch_y),
                egui::pos2(response.rect.max.x, notch_y),
            ],
            egui::Stroke::new(1.0, theme.unity_notch),
        );

        let dbl = ui.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary));
        if dbl && response.rect.contains(ui.input(|i| i.pointer.interact_pos().unwrap_or_default())) {
            *gain_db = 0.0;
            changed = true;
        }
    });

    (changed, drag_stopped)
}

fn draw_db_ruler(ui: &mut egui::Ui, height: f32, theme: &MeterTheme) {
    let desired = egui::vec2(24.0, height);
    let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
    let painter = ui.painter_at(rect);

    let font = egui::FontId::monospace(8.0);
    let half_line = 4.0;

    for &(linear, label) in DB_TICKS {
        let y = rect.max.y - rect.height() * linear;
        if y < rect.min.y || y > rect.max.y { continue; }
        let label_y = y.clamp(rect.min.y + half_line, rect.max.y - half_line);
        painter.line_segment(
            [egui::pos2(rect.min.x, y), egui::pos2(rect.min.x + 3.0, y)],
            egui::Stroke::new(1.0, theme.ruler_tick),
        );
        painter.text(
            egui::pos2(rect.min.x + 5.0, label_y),
            egui::Align2::LEFT_CENTER,
            label,
            font.clone(),
            theme.ruler_text,
        );
    }
}
