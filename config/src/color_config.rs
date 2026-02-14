use crate::background::BackgroundLayer;
use crate::color::{HsbTransform, Palette, RgbaColor, SrgbaTuple};
use crate::config::BoldBrightening;
use phaedra_dynamic::{FromDynamic, ToDynamic};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, FromDynamic, ToDynamic)]
pub struct ColorConfig {
    #[dynamic(default)]
    pub color_scheme_dirs: Vec<PathBuf>,
    #[dynamic(default)]
    pub bold_brightens_ansi_colors: BoldBrightening,
    pub colors: Option<Palette>,
    #[dynamic(default)]
    pub resolved_palette: Palette,
    pub color_scheme: Option<String>,
    #[dynamic(default)]
    pub color_schemes: HashMap<String, Palette>,
    #[dynamic(default)]
    pub foreground_text_hsb: HsbTransform,
    #[dynamic(default = "default_inactive_pane_hsb")]
    pub inactive_pane_hsb: HsbTransform,
    #[dynamic(default)]
    pub background: Vec<BackgroundLayer>,
    #[dynamic(default = "default_char_select_fg_color")]
    pub char_select_fg_color: RgbaColor,
    #[dynamic(default = "default_char_select_bg_color")]
    pub char_select_bg_color: RgbaColor,
    #[dynamic(default = "default_command_palette_fg_color")]
    pub command_palette_fg_color: RgbaColor,
    #[dynamic(default = "default_command_palette_bg_color")]
    pub command_palette_bg_color: RgbaColor,
    #[dynamic(default = "default_pane_select_fg_color")]
    pub pane_select_fg_color: RgbaColor,
    #[dynamic(default = "default_pane_select_bg_color")]
    pub pane_select_bg_color: RgbaColor,
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self {
            color_scheme_dirs: Vec::new(),
            bold_brightens_ansi_colors: BoldBrightening::default(),
            colors: None,
            resolved_palette: Palette::default(),
            color_scheme: None,
            color_schemes: HashMap::new(),
            foreground_text_hsb: HsbTransform::default(),
            inactive_pane_hsb: default_inactive_pane_hsb(),
            background: Vec::new(),
            char_select_fg_color: default_char_select_fg_color(),
            char_select_bg_color: default_char_select_bg_color(),
            command_palette_fg_color: default_command_palette_fg_color(),
            command_palette_bg_color: default_command_palette_bg_color(),
            pane_select_fg_color: default_pane_select_fg_color(),
            pane_select_bg_color: default_pane_select_bg_color(),
        }
    }
}

fn default_inactive_pane_hsb() -> HsbTransform {
    HsbTransform {
        brightness: 0.8,
        saturation: 0.9,
        hue: 1.0,
    }
}

fn default_char_select_fg_color() -> RgbaColor {
    SrgbaTuple(0.75, 0.75, 0.75, 1.0).into()
}

fn default_char_select_bg_color() -> RgbaColor {
    (0x33, 0x33, 0x33).into()
}

fn default_command_palette_fg_color() -> RgbaColor {
    SrgbaTuple(0.75, 0.75, 0.75, 1.0).into()
}

fn default_command_palette_bg_color() -> RgbaColor {
    (0x33, 0x33, 0x33).into()
}

fn default_pane_select_fg_color() -> RgbaColor {
    SrgbaTuple(0.75, 0.75, 0.75, 1.0).into()
}

fn default_pane_select_bg_color() -> RgbaColor {
    SrgbaTuple(0., 0., 0., 0.5).into()
}
