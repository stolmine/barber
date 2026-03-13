use crate::keybinds::Keybinds;
use crate::ui::toolbar::ToolbarAction;

pub fn menu_bar_ui(
    ui: &mut egui::Ui,
    keybinds: &Keybinds,
    has_file: bool,
    has_selection: bool,
    undo_label: Option<&str>,
    redo_label: Option<&str>,
    has_clipboard: bool,
) -> Option<ToolbarAction> {
    let mut action = None;

    egui::menu::bar(ui, |ui| {
        ui.menu_button("File", |ui| {
            menu_item(ui, keybinds, "OpenFile", "Open...", true, &mut action);
            menu_item(ui, keybinds, "Export", "Export...", has_file, &mut action);
            ui.separator();
            menu_item(ui, keybinds, "Quit", "Quit", true, &mut action);
        });

        ui.menu_button("Edit", |ui| {
            let undo_text = undo_label.map(|l| format!("Undo {}", l)).unwrap_or_else(|| "Undo".to_string());
            menu_item(ui, keybinds, "Undo", &undo_text, undo_label.is_some(), &mut action);
            let redo_text = redo_label.map(|l| format!("Redo {}", l)).unwrap_or_else(|| "Redo".to_string());
            menu_item(ui, keybinds, "Redo", &redo_text, redo_label.is_some(), &mut action);
            ui.separator();
            menu_item(ui, keybinds, "SelectAll", "Select All", has_file, &mut action);
            menu_item(ui, keybinds, "Cut", "Cut", has_selection, &mut action);
            menu_item(ui, keybinds, "Copy", "Copy", has_selection, &mut action);
            menu_item(ui, keybinds, "Paste", "Paste", has_clipboard, &mut action);
            menu_item(ui, keybinds, "Duplicate", "Duplicate", has_selection, &mut action);
            ui.separator();
            menu_item(ui, keybinds, "GapDelete", "Gap Delete", has_selection, &mut action);
            menu_item(ui, keybinds, "RippleDelete", "Ripple Delete", has_selection, &mut action);
            menu_item(ui, keybinds, "Crop", "Crop", has_selection, &mut action);
            ui.separator();
            menu_item(ui, keybinds, "Reverse", "Reverse", has_file, &mut action);
            menu_item(ui, keybinds, "Normalize", "Normalize", has_file, &mut action);
            menu_item(ui, keybinds, "RemoveDC", "Remove DC Offset", has_file, &mut action);
            menu_item(ui, keybinds, "ChangeSpeed", "Change Speed...", has_file, &mut action);
            menu_item(ui, keybinds, "ToggleFade", "Toggle Fades", has_file, &mut action);
            ui.separator();
            ui.add_enabled_ui(has_selection, |ui| {
                ui.menu_button("Fade In", |ui| {
                    menu_item(ui, keybinds, "FadeInLinear", "Linear", has_selection, &mut action);
                    menu_item(ui, keybinds, "FadeInExponential", "Exponential", has_selection, &mut action);
                    menu_item(ui, keybinds, "FadeInLogarithmic", "Logarithmic", has_selection, &mut action);
                    menu_item(ui, keybinds, "FadeInSCurve", "S-Curve", has_selection, &mut action);
                });
                ui.menu_button("Fade Out", |ui| {
                    menu_item(ui, keybinds, "FadeOutLinear", "Linear", has_selection, &mut action);
                    menu_item(ui, keybinds, "FadeOutExponential", "Exponential", has_selection, &mut action);
                    menu_item(ui, keybinds, "FadeOutLogarithmic", "Logarithmic", has_selection, &mut action);
                    menu_item(ui, keybinds, "FadeOutSCurve", "S-Curve", has_selection, &mut action);
                });
            });
        });

        ui.menu_button("Transport", |ui| {
            menu_item(ui, keybinds, "PlaySelection", "Play Selection", has_selection, &mut action);
            ui.separator();
            menu_item(ui, keybinds, "SetInPoint", "Set In Point", has_file, &mut action);
            menu_item(ui, keybinds, "SetOutPoint", "Set Out Point", has_file, &mut action);
            menu_item(ui, keybinds, "GoToInPoint", "Go to In", has_file, &mut action);
            menu_item(ui, keybinds, "GoToOutPoint", "Go to Out", has_file, &mut action);
        });

        ui.menu_button("View", |ui| {
            menu_item(ui, keybinds, "ZoomIn", "Zoom In", has_file, &mut action);
            menu_item(ui, keybinds, "ZoomOut", "Zoom Out", has_file, &mut action);
            menu_item(ui, keybinds, "ZoomToFit", "Zoom to Fit", has_file, &mut action);
            ui.separator();
            menu_item(ui, keybinds, "VerticalZoomIn", "Vertical Zoom In", has_file, &mut action);
            menu_item(ui, keybinds, "VerticalZoomOut", "Vertical Zoom Out", has_file, &mut action);
            menu_item(ui, keybinds, "VerticalZoomReset", "Vertical Zoom Reset", has_file, &mut action);
        });
    });

    action
}

fn menu_item(
    ui: &mut egui::Ui,
    keybinds: &Keybinds,
    action_name: &str,
    label: &str,
    enabled: bool,
    action: &mut Option<ToolbarAction>,
) {
    let shortcut = keybinds.format_shortcut(action_name);
    let button = egui::Button::new(label).shortcut_text(shortcut);
    if ui.add_enabled(enabled, button).clicked() {
        *action = match action_name {
            "OpenFile" => Some(ToolbarAction::OpenFile),
            "Export" => Some(ToolbarAction::Export),
            "Undo" => Some(ToolbarAction::Undo),
            "Redo" => Some(ToolbarAction::Redo),
            "Cut" => Some(ToolbarAction::Cut),
            "Copy" => Some(ToolbarAction::Copy),
            "Paste" => Some(ToolbarAction::Paste),
            "Duplicate" => Some(ToolbarAction::Duplicate),
            "GapDelete" => Some(ToolbarAction::GapDelete),
            "RippleDelete" => Some(ToolbarAction::RippleDelete),
            "Crop" => Some(ToolbarAction::Crop),
            "Reverse" => Some(ToolbarAction::Reverse),
            "Normalize" => Some(ToolbarAction::Normalize),
            "RemoveDC" => Some(ToolbarAction::RemoveDC),
            "ChangeSpeed" => Some(ToolbarAction::ChangeSpeed),
            "ToggleFade" => Some(ToolbarAction::ToggleFade),
            "SelectAll" => Some(ToolbarAction::SelectAll),
            "Quit" => Some(ToolbarAction::Quit),
            "PlaySelection" => Some(ToolbarAction::PlaySelection),
            "ZoomIn" => Some(ToolbarAction::ZoomIn),
            "ZoomOut" => Some(ToolbarAction::ZoomOut),
            "ZoomToFit" => Some(ToolbarAction::ZoomToFit),
            "VerticalZoomIn" => Some(ToolbarAction::VerticalZoomIn),
            "VerticalZoomOut" => Some(ToolbarAction::VerticalZoomOut),
            "VerticalZoomReset" => Some(ToolbarAction::VerticalZoomReset),
            "SetInPoint" => Some(ToolbarAction::SetInPoint),
            "SetOutPoint" => Some(ToolbarAction::SetOutPoint),
            "GoToInPoint" => Some(ToolbarAction::GoToInPoint),
            "GoToOutPoint" => Some(ToolbarAction::GoToOutPoint),
            "FadeInLinear" => Some(ToolbarAction::FadeInLinear),
            "FadeInExponential" => Some(ToolbarAction::FadeInExponential),
            "FadeInLogarithmic" => Some(ToolbarAction::FadeInLogarithmic),
            "FadeInSCurve" => Some(ToolbarAction::FadeInSCurve),
            "FadeOutLinear" => Some(ToolbarAction::FadeOutLinear),
            "FadeOutExponential" => Some(ToolbarAction::FadeOutExponential),
            "FadeOutLogarithmic" => Some(ToolbarAction::FadeOutLogarithmic),
            "FadeOutSCurve" => Some(ToolbarAction::FadeOutSCurve),
            _ => None,
        };
        ui.close_menu();
    }
}
