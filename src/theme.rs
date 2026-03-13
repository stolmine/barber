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

impl WaveformTheme {
    fn dark() -> Self {
        let wave = Color32::from_rgb(0x83, 0xC0, 0x92); // everforest aqua
        Self {
            background: Color32::from_rgb(0x27, 0x2E, 0x33),       // bg0
            center_line: Color32::from_rgb(0x49, 0x51, 0x56),      // bg4
            channel_separator: Color32::from_rgb(0x41, 0x4B, 0x50), // bg3
            waveform_fill: wave,
            waveform_stroke: wave,
            waveform_stroke_width: 1.0,
            selection_fill: Color32::from_rgba_unmultiplied(0x7F, 0xBB, 0xB3, 0x30),    // blue
            selection_stroke: Color32::from_rgba_unmultiplied(0x7F, 0xBB, 0xB3, 0x90),  // blue
            playhead: Color32::from_rgb(0xE6, 0x98, 0x75),         // orange
            phantom_bg: Color32::from_rgba_unmultiplied(0xE6, 0x98, 0x75, 0x38),
            phantom_wave: Color32::from_rgb(0xE6, 0x7E, 0x80),     // red
            in_point: Color32::from_rgb(0xA7, 0xC0, 0x80),         // green
            out_point: Color32::from_rgb(0xE6, 0x7E, 0x80),        // red
            ruler_text: Color32::from_rgb(0x9D, 0xA9, 0xA0),       // grey2
            ruler_tick: Color32::from_rgb(0x7A, 0x84, 0x78),       // grey0
        }
    }

    fn light() -> Self {
        let wave = Color32::from_rgb(0x35, 0xA7, 0x7C); // everforest aqua
        Self {
            background: Color32::from_rgb(0xFF, 0xFB, 0xEF),       // bg0
            center_line: Color32::from_rgb(0xE8, 0xE5, 0xD5),      // bg4
            channel_separator: Color32::from_rgb(0xED, 0xEA, 0xDA), // bg3
            waveform_fill: wave,
            waveform_stroke: wave,
            waveform_stroke_width: 1.0,
            selection_fill: Color32::from_rgba_unmultiplied(0x3A, 0x94, 0xC5, 0x30),    // blue
            selection_stroke: Color32::from_rgba_unmultiplied(0x3A, 0x94, 0xC5, 0x90),  // blue
            playhead: Color32::from_rgb(0xF5, 0x7D, 0x26),         // orange
            phantom_bg: Color32::from_rgba_unmultiplied(0xF5, 0x7D, 0x26, 0x38),
            phantom_wave: Color32::from_rgb(0xF8, 0x55, 0x52),     // red
            in_point: Color32::from_rgb(0x8D, 0xA1, 0x01),         // green
            out_point: Color32::from_rgb(0xF8, 0x55, 0x52),        // red
            ruler_text: Color32::from_rgb(0x82, 0x91, 0x81),       // grey2
            ruler_tick: Color32::from_rgb(0xA6, 0xB0, 0xA0),       // grey0
        }
    }
}

impl Default for WaveformTheme {
    fn default() -> Self {
        Self::dark()
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

impl MinimapTheme {
    fn dark() -> Self {
        Self {
            background: Color32::from_rgb(0x1E, 0x23, 0x26),       // bg_dim
            waveform: Color32::from_rgb(0x83, 0xC0, 0x92),         // aqua
            viewport_stroke: Color32::from_rgb(0xD3, 0xC6, 0xAA),  // fg
            dim_overlay: Color32::from_rgba_unmultiplied(0x00, 0x00, 0x00, 0x78),
            playhead: Color32::from_rgb(0xE6, 0x98, 0x75),         // orange
        }
    }

    fn light() -> Self {
        Self {
            background: Color32::from_rgb(0xF2, 0xEF, 0xDF),       // bg_dim
            waveform: Color32::from_rgb(0x35, 0xA7, 0x7C),         // aqua
            viewport_stroke: Color32::from_rgb(0x5C, 0x6A, 0x72),  // fg
            dim_overlay: Color32::from_rgba_unmultiplied(0xFF, 0xFB, 0xEF, 0x78), // bg0
            playhead: Color32::from_rgb(0xF5, 0x7D, 0x26),         // orange
        }
    }
}

impl Default for MinimapTheme {
    fn default() -> Self {
        Self::dark()
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

impl MeterTheme {
    fn dark() -> Self {
        Self {
            background: Color32::from_rgb(0x2E, 0x38, 0x3C),       // bg1
            green: Color32::from_rgb(0xA7, 0xC0, 0x80),            // green
            yellow: Color32::from_rgb(0xDB, 0xBC, 0x7F),           // yellow
            red: Color32::from_rgb(0xE6, 0x7E, 0x80),              // red
            unity_notch: Color32::from_rgb(0x85, 0x92, 0x89),      // grey1
            ruler_text: Color32::from_rgb(0x9D, 0xA9, 0xA0),       // grey2
            ruler_tick: Color32::from_rgb(0x7A, 0x84, 0x78),       // grey0
        }
    }

    fn light() -> Self {
        Self {
            background: Color32::from_rgb(0xF8, 0xF5, 0xE4),       // bg1
            green: Color32::from_rgb(0x8D, 0xA1, 0x01),            // green
            yellow: Color32::from_rgb(0xDF, 0xA0, 0x00),           // yellow
            red: Color32::from_rgb(0xF8, 0x55, 0x52),              // red
            unity_notch: Color32::from_rgb(0x93, 0x9F, 0x91),      // grey1
            ruler_text: Color32::from_rgb(0x82, 0x91, 0x81),       // grey2
            ruler_tick: Color32::from_rgb(0xA6, 0xB0, 0xA0),       // grey0
        }
    }
}

impl Default for MeterTheme {
    fn default() -> Self {
        Self::dark()
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct ThemeVariant {
    pub waveform: WaveformTheme,
    pub minimap: MinimapTheme,
    pub meter: MeterTheme,
    #[serde(with = "hex_color")] pub error_text: Color32,
}

impl ThemeVariant {
    fn dark() -> Self {
        Self {
            waveform: WaveformTheme::dark(),
            minimap: MinimapTheme::dark(),
            meter: MeterTheme::dark(),
            error_text: Color32::from_rgb(0xE6, 0x7E, 0x80), // everforest red
        }
    }

    fn light() -> Self {
        Self {
            waveform: WaveformTheme::light(),
            minimap: MinimapTheme::light(),
            meter: MeterTheme::light(),
            error_text: Color32::from_rgb(0xF8, 0x55, 0x52), // everforest red
        }
    }
}

impl Default for ThemeVariant {
    fn default() -> Self {
        Self::dark()
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct AppTheme {
    pub dark: ThemeVariant,
    pub light: ThemeVariant,
}

impl Default for AppTheme {
    fn default() -> Self {
        Self {
            dark: ThemeVariant::dark(),
            light: ThemeVariant::light(),
        }
    }
}

impl AppTheme {
    pub fn active(&self, dark_mode: bool) -> &ThemeVariant {
        if dark_mode { &self.dark } else { &self.light }
    }

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
            if let Ok(variant) = toml::from_str::<ThemeVariant>(&contents) {
                return Self { dark: variant, light: ThemeVariant::light() };
            }
        }
        if std::fs::create_dir_all(&config_dir).is_ok() {
            let _ = std::fs::write(&path, toml::to_string_pretty(&defaults).unwrap_or_default());
        }
        defaults
    }
}
