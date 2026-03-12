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
    Quit,
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
