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
    GapDelete,
    RippleDelete,
    Crop,
    Cut,
    Copy,
    Paste,
    Undo,
    Redo,
    PlaySelection,
    ToggleLoop,
    ToggleFollow,
}

pub fn toolbar_ui(
    ui: &mut egui::Ui,
    is_playing: bool,
    has_selection: bool,
    has_file: bool,
    can_undo: bool,
    can_redo: bool,
    has_clipboard: bool,
    is_loop: bool,
    is_following: bool,
) -> Option<ToolbarAction> {
    let mut action = None;

    let shift_space_pressed = ui.input(|i| i.modifiers.shift && i.key_pressed(Key::Space));
    let space_pressed = ui.input(|i| !i.modifiers.shift && i.key_pressed(Key::Space));
    let loop_pressed = ui.input(|i| !i.modifiers.command && !i.modifiers.shift && i.key_pressed(Key::L));
    let follow_pressed = ui.input(|i| !i.modifiers.command && !i.modifiers.shift && i.key_pressed(Key::F));
    let gap_delete_pressed = ui.input(|i| {
        let no_shift = !i.modifiers.shift;
        no_shift && (i.key_pressed(Key::Backspace) || i.key_pressed(Key::Delete))
    });
    let ripple_delete_pressed = ui.input(|i| {
        let shift = i.modifiers.shift;
        shift && (i.key_pressed(Key::Backspace) || i.key_pressed(Key::Delete))
    });
    let undo_pressed = ui.input(|i| i.modifiers.command && !i.modifiers.shift && i.key_pressed(Key::Z));
    let redo_pressed = ui.input(|i| i.modifiers.command && i.modifiers.shift && i.key_pressed(Key::Z));
    let cut_pressed = ui.input(|i| i.modifiers.command && i.key_pressed(Key::X));
    let copy_pressed = ui.input(|i| i.modifiers.command && !i.modifiers.shift && i.key_pressed(Key::C));
    let paste_pressed = ui.input(|i| i.modifiers.command && i.key_pressed(Key::V));

    let play_pause_action = if is_playing {
        ToolbarAction::Pause
    } else {
        ToolbarAction::Play
    };

    if shift_space_pressed && has_selection {
        action = Some(ToolbarAction::PlaySelection);
    }

    if space_pressed {
        action = Some(play_pause_action);
    }

    if loop_pressed {
        action = Some(ToolbarAction::ToggleLoop);
    }
    if follow_pressed {
        action = Some(ToolbarAction::ToggleFollow);
    }

    if gap_delete_pressed && has_selection {
        action = Some(ToolbarAction::GapDelete);
    }

    if ripple_delete_pressed && has_selection {
        action = Some(ToolbarAction::RippleDelete);
    }

    if undo_pressed && can_undo {
        action = Some(ToolbarAction::Undo);
    }

    if redo_pressed && can_redo {
        action = Some(ToolbarAction::Redo);
    }

    if cut_pressed && has_selection {
        action = Some(ToolbarAction::Cut);
    }

    if copy_pressed && has_selection {
        action = Some(ToolbarAction::Copy);
    }

    if paste_pressed && has_clipboard {
        action = Some(ToolbarAction::Paste);
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
            action = Some(play_pause_action);
        }

        if ui
            .add_enabled(has_file, egui::Button::new("Stop"))
            .clicked()
        {
            action = Some(ToolbarAction::Stop);
        }

        if ui.add_enabled(has_file, egui::SelectableLabel::new(is_loop, "Loop")).clicked() {
            action = Some(ToolbarAction::ToggleLoop);
        }
        if ui.add_enabled(has_file, egui::SelectableLabel::new(is_following, "Follow")).clicked() {
            action = Some(ToolbarAction::ToggleFollow);
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
            .add_enabled(can_undo, egui::Button::new("Undo"))
            .clicked()
        {
            action = Some(ToolbarAction::Undo);
        }

        if ui
            .add_enabled(can_redo, egui::Button::new("Redo"))
            .clicked()
        {
            action = Some(ToolbarAction::Redo);
        }

        ui.separator();

        if ui
            .add_enabled(has_selection, egui::Button::new("Gap Del"))
            .clicked()
        {
            action = Some(ToolbarAction::GapDelete);
        }

        if ui
            .add_enabled(has_selection, egui::Button::new("Ripple Del"))
            .clicked()
        {
            action = Some(ToolbarAction::RippleDelete);
        }

        if ui
            .add_enabled(has_selection, egui::Button::new("Crop"))
            .clicked()
        {
            action = Some(ToolbarAction::Crop);
        }

        if ui.add_enabled(has_selection, egui::Button::new("Cut")).clicked() {
            action = Some(ToolbarAction::Cut);
        }

        if ui.add_enabled(has_selection, egui::Button::new("Copy")).clicked() {
            action = Some(ToolbarAction::Copy);
        }

        if ui.add_enabled(has_clipboard, egui::Button::new("Paste")).clicked() {
            action = Some(ToolbarAction::Paste);
        }
    });

    action
}
