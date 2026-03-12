use egui::Key;

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
    Delete,
    Crop,
}

pub fn toolbar_ui(
    ui: &mut egui::Ui,
    is_playing: bool,
    has_selection: bool,
    has_file: bool,
) -> Option<ToolbarAction> {
    let mut action = None;

    let space_pressed = ui.input(|i| i.key_pressed(Key::Space));
    let delete_pressed = ui.input(|i| {
        i.key_pressed(Key::Backspace) || i.key_pressed(Key::Delete)
    });

    if space_pressed {
        action = Some(if is_playing {
            ToolbarAction::Pause
        } else {
            ToolbarAction::Play
        });
    }

    if delete_pressed && has_selection {
        action = Some(ToolbarAction::Delete);
    }

    ui.horizontal(|ui| {
        if ui.button("Open").clicked() {
            action = Some(ToolbarAction::OpenFile);
        }

        if ui
            .add_enabled(has_file, egui::Button::new("Export"))
            .clicked()
        {
            action = Some(ToolbarAction::Export);
        }

        ui.separator();

        let play_pause_label = if is_playing { "Pause" } else { "Play" };
        if ui
            .add_enabled(has_file, egui::Button::new(play_pause_label))
            .clicked()
        {
            action = Some(if is_playing {
                ToolbarAction::Pause
            } else {
                ToolbarAction::Play
            });
        }

        if ui
            .add_enabled(has_file, egui::Button::new("Stop"))
            .clicked()
        {
            action = Some(ToolbarAction::Stop);
        }

        ui.separator();

        if ui
            .add_enabled(has_file, egui::Button::new("Zoom In"))
            .clicked()
        {
            action = Some(ToolbarAction::ZoomIn);
        }

        if ui
            .add_enabled(has_file, egui::Button::new("Zoom Out"))
            .clicked()
        {
            action = Some(ToolbarAction::ZoomOut);
        }

        if ui
            .add_enabled(has_file, egui::Button::new("Zoom to Fit"))
            .clicked()
        {
            action = Some(ToolbarAction::ZoomToFit);
        }

        ui.separator();

        if ui
            .add_enabled(has_selection, egui::Button::new("Delete"))
            .clicked()
        {
            action = Some(ToolbarAction::Delete);
        }

        if ui
            .add_enabled(has_selection, egui::Button::new("Crop"))
            .clicked()
        {
            action = Some(ToolbarAction::Crop);
        }
    });

    action
}
