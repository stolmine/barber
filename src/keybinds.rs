use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::ui::toolbar::ToolbarAction;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyCombo {
    #[serde(default)]
    pub command: bool,
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub alt: bool,
    pub key: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Keybinds {
    pub bindings: HashMap<String, KeyCombo>,
}

impl Default for Keybinds {
    fn default() -> Self {
        let mut b = HashMap::new();
        let k = |cmd: bool, shift: bool, alt: bool, key: &str| KeyCombo {
            command: cmd, shift, alt, key: key.to_string(),
        };
        b.insert("Play".into(), k(false, false, false, "Space"));
        b.insert("PlaySelection".into(), k(false, true, false, "Space"));
        b.insert("ToggleLoop".into(), k(false, false, false, "L"));
        b.insert("ToggleFollow".into(), k(false, false, false, "F"));
        b.insert("ToggleSnapZero".into(), k(false, false, false, "S"));
        b.insert("GapDelete".into(), k(false, false, false, "Backspace"));
        b.insert("RippleDelete".into(), k(false, true, false, "Backspace"));
        b.insert("Undo".into(), k(true, false, false, "Z"));
        b.insert("Redo".into(), k(true, true, false, "Z"));
        b.insert("Cut".into(), k(true, false, false, "X"));
        b.insert("Copy".into(), k(true, false, false, "C"));
        b.insert("Paste".into(), k(true, false, false, "V"));
        b.insert("Duplicate".into(), k(true, false, false, "D"));
        b.insert("OpenFile".into(), k(true, false, false, "O"));
        b.insert("Export".into(), k(true, false, false, "E"));
        b.insert("Stop".into(), k(false, false, false, "Escape"));
        b.insert("Reverse".into(), k(true, false, false, "R"));
        b.insert("ChangeSpeed".into(), k(true, true, false, "R"));
        b.insert("Normalize".into(), k(true, true, false, "N"));
        b.insert("RemoveDC".into(), k(true, true, false, "D"));
        b.insert("SelectAll".into(), k(true, false, false, "A"));
        b.insert("Crop".into(), k(true, false, false, "K"));
        b.insert("SetInPoint".into(), k(false, true, false, "I"));
        b.insert("SetOutPoint".into(), k(false, true, false, "O"));
        b.insert("GoToInPoint".into(), k(false, false, false, "I"));
        b.insert("GoToOutPoint".into(), k(false, false, false, "O"));
        b.insert("GoToStart".into(), k(false, false, false, "ArrowUp"));
        b.insert("GoToEnd".into(), k(false, false, false, "ArrowDown"));
        b.insert("NudgeLeft".into(), k(false, false, false, "ArrowLeft"));
        b.insert("NudgeRight".into(), k(false, false, false, "ArrowRight"));
        b.insert("VolumeUp".into(), k(true, false, false, "ArrowUp"));
        b.insert("VolumeDown".into(), k(true, false, false, "ArrowDown"));
        b.insert("Quit".into(), k(true, false, false, "Q"));
        b.insert("ZoomIn".into(), k(true, false, false, "Equals"));
        b.insert("ZoomOut".into(), k(true, false, false, "Minus"));
        b.insert("ZoomToFit".into(), k(false, false, false, "Enter"));
        b.insert("FadeInLinear".into(), k(true, false, false, "F"));
        b.insert("FadeOutLinear".into(), k(true, true, false, "F"));
        b.insert("VerticalZoomIn".into(), k(true, true, false, "Plus"));
        b.insert("VerticalZoomOut".into(), k(true, true, false, "Minus"));
        b.insert("VerticalZoomReset".into(), k(true, false, false, "Num0"));
        Self { bindings: b }
    }
}

impl Keybinds {
    pub fn load() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("barber");
        let path = config_dir.join("keybinds.toml");
        let defaults = Self::default();
        if let Ok(contents) = std::fs::read_to_string(&path) {
            if let Ok(mut kb) = toml::from_str::<Keybinds>(&contents) {
                for (name, combo) in &defaults.bindings {
                    kb.bindings.entry(name.clone()).or_insert_with(|| combo.clone());
                }
                return kb;
            }
        }
        if std::fs::create_dir_all(&config_dir).is_ok() {
            let _ = std::fs::write(&path, toml::to_string_pretty(&defaults).unwrap_or_default());
        }
        defaults
    }

    pub fn check_input(
        &self,
        ctx: &egui::Context,
        is_playing: bool,
        has_selection: bool,
        has_file: bool,
        can_undo: bool,
        can_redo: bool,
        has_clipboard: bool,
    ) -> Option<ToolbarAction> {
        let (evt_copy, evt_cut, evt_paste) = ctx.input(|i| {
            let mut copy = false;
            let mut cut = false;
            let mut paste = false;
            for event in &i.events {
                match event {
                    egui::Event::Copy => copy = true,
                    egui::Event::Cut => cut = true,
                    egui::Event::Paste(_) => paste = true,
                    _ => {}
                }
            }
            (copy, cut, paste)
        });
        if evt_cut && has_selection { return Some(ToolbarAction::Cut); }
        if evt_copy && has_selection { return Some(ToolbarAction::Copy); }
        if evt_paste && has_clipboard { return Some(ToolbarAction::Paste); }

        for (name, combo) in &self.bindings {
            let Some(key) = parse_key(&combo.key) else { continue };
            let pressed = ctx.input(|i| {
                i.key_pressed(key)
                    && i.modifiers.command == combo.command
                    && i.modifiers.shift == combo.shift
                    && i.modifiers.alt == combo.alt
            });
            if !pressed { continue; }
            let action = match name.as_str() {
                "Play" => Some(if is_playing { ToolbarAction::Pause } else { ToolbarAction::Play }),
                "PlaySelection" if has_file => Some(ToolbarAction::PlaySelection),
                "ToggleLoop" => Some(ToolbarAction::ToggleLoop),
                "ToggleFollow" => Some(ToolbarAction::ToggleFollow),
                "ToggleSnapZero" => Some(ToolbarAction::ToggleSnapZero),
                "GapDelete" if has_selection => Some(ToolbarAction::GapDelete),
                "RippleDelete" if has_selection => Some(ToolbarAction::RippleDelete),
                "Undo" if can_undo => Some(ToolbarAction::Undo),
                "Redo" if can_redo => Some(ToolbarAction::Redo),
                "Cut" if has_selection => Some(ToolbarAction::Cut),
                "Copy" if has_selection => Some(ToolbarAction::Copy),
                "Paste" if has_clipboard => Some(ToolbarAction::Paste),
                "Duplicate" if has_selection => Some(ToolbarAction::Duplicate),
                "OpenFile" => Some(ToolbarAction::OpenFile),
                "Export" if has_file => Some(ToolbarAction::Export),
                "Stop" => Some(ToolbarAction::Stop),
                "Reverse" if has_file => Some(ToolbarAction::Reverse),
                "ChangeSpeed" if has_file => Some(ToolbarAction::ChangeSpeed),
                "Normalize" if has_file => Some(ToolbarAction::Normalize),
                "RemoveDC" if has_file => Some(ToolbarAction::RemoveDC),
                "ToggleFade" if has_file => Some(ToolbarAction::ToggleFade),
                "SelectAll" if has_file => Some(ToolbarAction::SelectAll),
                "Crop" if has_selection => Some(ToolbarAction::Crop),
                "ZoomIn" if has_file => Some(ToolbarAction::ZoomIn),
                "ZoomOut" if has_file => Some(ToolbarAction::ZoomOut),
                "ZoomToFit" if has_file => Some(ToolbarAction::ZoomToFit),
                "SetInPoint" if has_file => Some(ToolbarAction::SetInPoint),
                "SetOutPoint" if has_file => Some(ToolbarAction::SetOutPoint),
                "GoToInPoint" if has_file => Some(ToolbarAction::GoToInPoint),
                "GoToOutPoint" if has_file => Some(ToolbarAction::GoToOutPoint),
                "GoToStart" if has_file => Some(ToolbarAction::GoToStart),
                "GoToEnd" if has_file => Some(ToolbarAction::GoToEnd),
                "NudgeLeft" if has_file => Some(ToolbarAction::NudgeLeft),
                "NudgeRight" if has_file => Some(ToolbarAction::NudgeRight),
                "VolumeUp" => Some(ToolbarAction::VolumeUp),
                "VolumeDown" => Some(ToolbarAction::VolumeDown),
                "Quit" => Some(ToolbarAction::Quit),
                "FadeInLinear" if has_selection => Some(ToolbarAction::FadeInLinear),
                "FadeInExponential" if has_selection => Some(ToolbarAction::FadeInExponential),
                "FadeInLogarithmic" if has_selection => Some(ToolbarAction::FadeInLogarithmic),
                "FadeInSCurve" if has_selection => Some(ToolbarAction::FadeInSCurve),
                "FadeOutLinear" if has_selection => Some(ToolbarAction::FadeOutLinear),
                "FadeOutExponential" if has_selection => Some(ToolbarAction::FadeOutExponential),
                "FadeOutLogarithmic" if has_selection => Some(ToolbarAction::FadeOutLogarithmic),
                "FadeOutSCurve" if has_selection => Some(ToolbarAction::FadeOutSCurve),
                "VerticalZoomIn" => Some(ToolbarAction::VerticalZoomIn),
                "VerticalZoomOut" => Some(ToolbarAction::VerticalZoomOut),
                "VerticalZoomReset" => Some(ToolbarAction::VerticalZoomReset),
                _ => None,
            };
            if action.is_some() { return action; }
        }
        None
    }

    pub fn format_shortcut(&self, action: &str) -> String {
        let Some(combo) = self.bindings.get(action) else { return String::new() };
        let mut parts = Vec::new();
        if combo.command { parts.push("\u{2318}"); }
        if combo.shift { parts.push("\u{21e7}"); }
        if combo.alt { parts.push("\u{2325}"); }
        parts.push(&combo.key);
        parts.join("")
    }
}

fn parse_key(name: &str) -> Option<egui::Key> {
    use egui::Key;
    match name {
        "A" => Some(Key::A), "B" => Some(Key::B), "C" => Some(Key::C),
        "D" => Some(Key::D), "E" => Some(Key::E), "F" => Some(Key::F),
        "G" => Some(Key::G), "H" => Some(Key::H), "I" => Some(Key::I),
        "J" => Some(Key::J), "K" => Some(Key::K), "L" => Some(Key::L),
        "M" => Some(Key::M), "N" => Some(Key::N), "O" => Some(Key::O),
        "P" => Some(Key::P), "Q" => Some(Key::Q), "R" => Some(Key::R),
        "S" => Some(Key::S), "T" => Some(Key::T), "U" => Some(Key::U),
        "V" => Some(Key::V), "W" => Some(Key::W), "X" => Some(Key::X),
        "Y" => Some(Key::Y), "Z" => Some(Key::Z),
        "Space" => Some(Key::Space),
        "Backspace" => Some(Key::Backspace),
        "Delete" => Some(Key::Delete),
        "Escape" => Some(Key::Escape),
        "Enter" => Some(Key::Enter),
        "ArrowUp" => Some(Key::ArrowUp),
        "ArrowDown" => Some(Key::ArrowDown),
        "ArrowLeft" => Some(Key::ArrowLeft),
        "ArrowRight" => Some(Key::ArrowRight),
        "Plus" => Some(Key::Plus),
        "Minus" => Some(Key::Minus),
        "Equals" => Some(Key::Equals),
        "Num0" => Some(Key::Num0),
        "F12" => Some(Key::F12),
        _ => None,
    }
}
