use egui::Color32;

pub mod hex_color {
    use egui::Color32;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(color: &Color32, s: S) -> Result<S::Ok, S::Error> {
        let (r, g, b, a) = (color.r(), color.g(), color.b(), color.a());
        let hex = if a == 255 {
            format!("#{:02X}{:02X}{:02X}", r, g, b)
        } else {
            format!("#{:02X}{:02X}{:02X}{:02X}", r, g, b, a)
        };
        s.serialize_str(&hex)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Color32, D::Error> {
        let s = String::deserialize(d)?;
        let hex = s.trim_start_matches('#');
        let parse = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).map_err(serde::de::Error::custom);
        match hex.len() {
            6 => Ok(Color32::from_rgba_unmultiplied(parse(0)?, parse(2)?, parse(4)?, 255)),
            8 => Ok(Color32::from_rgba_unmultiplied(parse(0)?, parse(2)?, parse(4)?, parse(6)?)),
            _ => Err(serde::de::Error::custom("expected #RRGGBB or #RRGGBBAA")),
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct WaveformTheme {
    #[serde(with = "hex_color")] pub background: Color32,
    #[serde(with = "hex_color")] pub center_line: Color32,
    #[serde(with = "hex_color")] pub channel_separator: Color32,
    #[serde(with = "hex_color")] pub waveform_fill: Color32,
    #[serde(with = "hex_color")] pub waveform_stroke: Color32,
    pub waveform_stroke_width: f32,
    #[serde(with = "hex_color")] pub selection_fill: Color32,
    #[serde(with = "hex_color")] pub selection_stroke: Color32,
    #[serde(with = "hex_color")] pub playhead: Color32,
    #[serde(with = "hex_color")] pub phantom_bg: Color32,
    #[serde(with = "hex_color")] pub phantom_wave: Color32,
    #[serde(with = "hex_color")] pub in_point: Color32,
    #[serde(with = "hex_color")] pub out_point: Color32,
    #[serde(with = "hex_color")] pub ruler_text: Color32,
    #[serde(with = "hex_color")] pub ruler_tick: Color32,
}

impl Default for WaveformTheme {
    fn default() -> Self {
        let wave = Color32::from_rgb(100, 180, 255);
        Self {
            background: Color32::from_rgb(20, 20, 24),
            center_line: Color32::from_rgb(50, 50, 60),
            channel_separator: Color32::from_rgb(70, 70, 80),
            waveform_fill: wave,
            waveform_stroke: wave,
            waveform_stroke_width: 1.0,
            selection_fill: Color32::from_rgba_unmultiplied(100, 180, 255, 40),
            selection_stroke: Color32::from_rgba_unmultiplied(100, 180, 255, 120),
            playhead: Color32::from_rgb(255, 220, 60),
            phantom_bg: Color32::from_rgba_unmultiplied(255, 220, 60, 50),
            phantom_wave: Color32::from_rgb(255, 120, 0),
            in_point: Color32::from_rgb(80, 220, 100),
            out_point: Color32::from_rgb(220, 80, 80),
            ruler_text: Color32::from_rgb(160, 160, 170),
            ruler_tick: Color32::from_rgb(80, 80, 90),
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct MinimapTheme {
    #[serde(with = "hex_color")] pub background: Color32,
    #[serde(with = "hex_color")] pub waveform: Color32,
    #[serde(with = "hex_color")] pub viewport_stroke: Color32,
    #[serde(with = "hex_color")] pub dim_overlay: Color32,
    #[serde(with = "hex_color")] pub playhead: Color32,
}

impl Default for MinimapTheme {
    fn default() -> Self {
        Self {
            background: Color32::from_rgb(16, 16, 20),
            waveform: Color32::from_rgb(70, 130, 190),
            viewport_stroke: Color32::from_gray(200),
            dim_overlay: Color32::from_rgba_premultiplied(0, 0, 0, 120),
            playhead: Color32::from_rgb(255, 220, 60),
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct MeterTheme {
    #[serde(with = "hex_color")] pub background: Color32,
    #[serde(with = "hex_color")] pub green: Color32,
    #[serde(with = "hex_color")] pub yellow: Color32,
    #[serde(with = "hex_color")] pub red: Color32,
    #[serde(with = "hex_color")] pub unity_notch: Color32,
    #[serde(with = "hex_color")] pub ruler_text: Color32,
    #[serde(with = "hex_color")] pub ruler_tick: Color32,
}

impl Default for MeterTheme {
    fn default() -> Self {
        Self {
            background: Color32::from_gray(30),
            green: Color32::from_rgb(80, 200, 80),
            yellow: Color32::from_rgb(255, 200, 0),
            red: Color32::from_rgb(255, 60, 60),
            unity_notch: Color32::from_gray(140),
            ruler_text: Color32::from_gray(130),
            ruler_tick: Color32::from_gray(80),
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct AppTheme {
    pub waveform: WaveformTheme,
    pub minimap: MinimapTheme,
    pub meter: MeterTheme,
    #[serde(with = "hex_color")] pub error_text: Color32,
}

impl Default for AppTheme {
    fn default() -> Self {
        Self {
            waveform: WaveformTheme::default(),
            minimap: MinimapTheme::default(),
            meter: MeterTheme::default(),
            error_text: Color32::RED,
        }
    }
}

impl AppTheme {
    pub fn load() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("barber");
        let path = config_dir.join("theme.toml");
        let defaults = Self::default();
        if let Ok(contents) = std::fs::read_to_string(&path) {
            if let Ok(theme) = toml::from_str::<AppTheme>(&contents) {
                return theme;
            }
        }
        if std::fs::create_dir_all(&config_dir).is_ok() {
            let _ = std::fs::write(&path, toml::to_string_pretty(&defaults).unwrap_or_default());
        }
        defaults
    }
}
