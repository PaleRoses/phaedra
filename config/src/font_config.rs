use crate::font::{
    AllowSquareGlyphOverflow, DisplayPixelGeometry, FontLocatorSelection, FontRasterizerSelection,
    FontShaperSelection, FreeTypeLoadFlags, FreeTypeLoadTarget, StyleRule, TextStyle,
};
use phaedra_dynamic::{FromDynamic, ToDynamic};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, FromDynamic, ToDynamic)]
pub struct FontConfig {
    #[dynamic(default = "default_font_size")]
    pub font_size: f64,
    #[dynamic(default)]
    pub font: TextStyle,
    #[dynamic(default)]
    pub font_rules: Vec<StyleRule>,
    #[dynamic(default)]
    pub font_dirs: Vec<PathBuf>,
    #[dynamic(default)]
    pub font_locator: FontLocatorSelection,
    #[dynamic(default)]
    pub font_rasterizer: FontRasterizerSelection,
    #[dynamic(default = "default_colr_rasterizer")]
    pub font_colr_rasterizer: FontRasterizerSelection,
    #[dynamic(default)]
    pub font_shaper: FontShaperSelection,
    #[dynamic(default)]
    pub display_pixel_geometry: DisplayPixelGeometry,
    #[dynamic(default)]
    pub freetype_load_target: FreeTypeLoadTarget,
    #[dynamic(default)]
    pub freetype_render_target: Option<FreeTypeLoadTarget>,
    #[dynamic(default)]
    pub freetype_load_flags: Option<FreeTypeLoadFlags>,
    pub freetype_interpreter_version: Option<u32>,
    #[dynamic(default)]
    pub freetype_pcf_long_family_names: bool,
    #[dynamic(default = "default_harfbuzz_features")]
    pub harfbuzz_features: Vec<String>,
    pub dpi: Option<f64>,
    #[dynamic(default)]
    pub dpi_by_screen: HashMap<String, f64>,
    #[dynamic(default)]
    pub allow_square_glyphs_to_overflow_width: AllowSquareGlyphOverflow,
    #[dynamic(default)]
    pub ignore_svg_fonts: bool,
    #[dynamic(default)]
    pub sort_fallback_fonts_by_coverage: bool,
    #[dynamic(default)]
    pub search_font_dirs_for_fallback: bool,
    #[dynamic(default)]
    pub use_cap_height_to_scale_fallback_fonts: bool,
    #[dynamic(default)]
    pub char_select_font: Option<TextStyle>,
    #[dynamic(default = "default_char_select_font_size")]
    pub char_select_font_size: f64,
    #[dynamic(default)]
    pub command_palette_font: Option<TextStyle>,
    #[dynamic(default = "default_command_palette_font_size")]
    pub command_palette_font_size: f64,
    #[dynamic(default)]
    pub pane_select_font: Option<TextStyle>,
    #[dynamic(default = "default_pane_select_font_size")]
    pub pane_select_font_size: f64,
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            font_size: default_font_size(),
            font: TextStyle::default(),
            font_rules: Vec::new(),
            font_dirs: Vec::new(),
            font_locator: FontLocatorSelection::default(),
            font_rasterizer: FontRasterizerSelection::default(),
            font_colr_rasterizer: default_colr_rasterizer(),
            font_shaper: FontShaperSelection::default(),
            display_pixel_geometry: DisplayPixelGeometry::default(),
            freetype_load_target: FreeTypeLoadTarget::default(),
            freetype_render_target: None,
            freetype_load_flags: None,
            freetype_interpreter_version: None,
            freetype_pcf_long_family_names: false,
            harfbuzz_features: default_harfbuzz_features(),
            dpi: None,
            dpi_by_screen: HashMap::new(),
            allow_square_glyphs_to_overflow_width: AllowSquareGlyphOverflow::default(),
            ignore_svg_fonts: false,
            sort_fallback_fonts_by_coverage: false,
            search_font_dirs_for_fallback: false,
            use_cap_height_to_scale_fallback_fonts: false,
            char_select_font: None,
            char_select_font_size: default_char_select_font_size(),
            command_palette_font: None,
            command_palette_font_size: default_command_palette_font_size(),
            pane_select_font: None,
            pane_select_font_size: default_pane_select_font_size(),
        }
    }
}

fn default_font_size() -> f64 {
    12.0
}

fn default_colr_rasterizer() -> FontRasterizerSelection {
    FontRasterizerSelection::Harfbuzz
}

fn default_harfbuzz_features() -> Vec<String> {
    ["kern", "liga", "clig"]
        .iter()
        .map(|&s| s.to_string())
        .collect()
}

fn default_char_select_font_size() -> f64 {
    18.0
}

fn default_command_palette_font_size() -> f64 {
    14.0
}

fn default_pane_select_font_size() -> f64 {
    36.0
}
