use crate::bell::EasingFunction;
use crate::cell::CellWidth;
use crate::config::NewlineCanon;
use crate::default_one_point_oh;
use crate::default_one_point_oh_f64;
use crate::units::Dimension;
use phaedra_bidi::ParagraphDirectionHint;
use phaedra_dynamic::{FromDynamic, ToDynamic};

#[derive(Debug, Clone, FromDynamic, ToDynamic)]
pub struct TextConfig {
    #[dynamic(
        default = "default_one_point_oh_f64",
        validate = "validate_line_height"
    )]
    pub line_height: f64,
    #[dynamic(default = "default_one_point_oh_f64")]
    pub cell_width: f64,
    #[dynamic(try_from = "crate::units::OptPixelUnit", default)]
    pub underline_thickness: Option<Dimension>,
    #[dynamic(try_from = "crate::units::OptPixelUnit", default)]
    pub underline_position: Option<Dimension>,
    #[dynamic(try_from = "crate::units::OptPixelUnit", default)]
    pub strikethrough_position: Option<Dimension>,
    #[dynamic(default = "default_true")]
    pub custom_block_glyphs: bool,
    #[dynamic(default = "default_true")]
    pub anti_alias_custom_block_glyphs: bool,
    #[dynamic(default = "default_one_point_oh")]
    pub text_background_opacity: f32,
    #[dynamic(default)]
    pub text_min_contrast_ratio: Option<f32>,
    #[dynamic(default = "default_text_blink_rate")]
    pub text_blink_rate: u64,
    #[dynamic(default = "linear_ease")]
    pub text_blink_ease_in: EasingFunction,
    #[dynamic(default = "linear_ease")]
    pub text_blink_ease_out: EasingFunction,
    #[dynamic(default = "default_text_blink_rate_rapid")]
    pub text_blink_rate_rapid: u64,
    #[dynamic(default = "linear_ease")]
    pub text_blink_rapid_ease_in: EasingFunction,
    #[dynamic(default = "linear_ease")]
    pub text_blink_rapid_ease_out: EasingFunction,
    #[dynamic(default)]
    pub normalize_output_to_unicode_nfc: bool,
    #[dynamic(default)]
    pub bidi_enabled: bool,
    #[dynamic(default)]
    pub bidi_direction: ParagraphDirectionHint,
    #[dynamic(default)]
    pub experimental_pixel_positioning: bool,
    #[dynamic(default)]
    pub use_box_model_render: bool,
    #[dynamic(default = "default_true")]
    pub warn_about_missing_glyphs: bool,
    #[dynamic(default)]
    pub canonicalize_pasted_newlines: Option<NewlineCanon>,
    #[dynamic(default = "default_unicode_version")]
    pub unicode_version: u8,
    #[dynamic(default)]
    pub treat_east_asian_ambiguous_width_as_wide: bool,
    #[dynamic(default)]
    pub cell_widths: Option<Vec<CellWidth>>,
}

impl Default for TextConfig {
    fn default() -> Self {
        Self {
            line_height: default_one_point_oh_f64(),
            cell_width: default_one_point_oh_f64(),
            underline_thickness: None,
            underline_position: None,
            strikethrough_position: None,
            custom_block_glyphs: default_true(),
            anti_alias_custom_block_glyphs: default_true(),
            text_background_opacity: default_one_point_oh(),
            text_min_contrast_ratio: None,
            text_blink_rate: default_text_blink_rate(),
            text_blink_ease_in: linear_ease(),
            text_blink_ease_out: linear_ease(),
            text_blink_rate_rapid: default_text_blink_rate_rapid(),
            text_blink_rapid_ease_in: linear_ease(),
            text_blink_rapid_ease_out: linear_ease(),
            normalize_output_to_unicode_nfc: false,
            bidi_enabled: false,
            bidi_direction: ParagraphDirectionHint::default(),
            experimental_pixel_positioning: false,
            use_box_model_render: false,
            warn_about_missing_glyphs: default_true(),
            canonicalize_pasted_newlines: None,
            unicode_version: default_unicode_version(),
            treat_east_asian_ambiguous_width_as_wide: false,
            cell_widths: None,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_text_blink_rate() -> u64 {
    500
}

fn default_text_blink_rate_rapid() -> u64 {
    250
}

const fn linear_ease() -> EasingFunction {
    EasingFunction::Linear
}

fn default_unicode_version() -> u8 {
    9
}

fn validate_line_height(value: &f64) -> Result<(), String> {
    if *value <= 0.0 {
        Err(format!(
            "Illegal value {value} for line_height; it must be positive and greater than zero!"
        ))
    } else {
        Ok(())
    }
}
