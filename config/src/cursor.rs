use crate::bell::EasingFunction;
use crate::config::DefaultCursorStyle;
use crate::units::Dimension;
use phaedra_dynamic::{FromDynamic, ToDynamic};

#[derive(Debug, Clone, FromDynamic, ToDynamic)]
pub struct CursorConfig {
    #[dynamic(try_from = "crate::units::OptPixelUnit", default)]
    pub cursor_thickness: Option<Dimension>,
    #[dynamic(default = "default_cursor_blink_rate")]
    pub cursor_blink_rate: u64,
    #[dynamic(default = "linear_ease")]
    pub cursor_blink_ease_in: EasingFunction,
    #[dynamic(default = "linear_ease")]
    pub cursor_blink_ease_out: EasingFunction,
    #[dynamic(default)]
    pub default_cursor_style: DefaultCursorStyle,
    #[dynamic(default)]
    pub force_reverse_video_cursor: bool,
    #[dynamic(default = "default_reverse_video_cursor_min_contrast")]
    pub reverse_video_cursor_min_contrast: f32,
    #[dynamic(default)]
    pub xcursor_theme: Option<String>,
    #[dynamic(default)]
    pub xcursor_size: Option<u32>,
}

impl Default for CursorConfig {
    fn default() -> Self {
        Self {
            cursor_thickness: None,
            cursor_blink_rate: default_cursor_blink_rate(),
            cursor_blink_ease_in: linear_ease(),
            cursor_blink_ease_out: linear_ease(),
            default_cursor_style: DefaultCursorStyle::default(),
            force_reverse_video_cursor: false,
            reverse_video_cursor_min_contrast: default_reverse_video_cursor_min_contrast(),
            xcursor_theme: None,
            xcursor_size: None,
        }
    }
}

fn default_cursor_blink_rate() -> u64 {
    800
}

const fn linear_ease() -> EasingFunction {
    EasingFunction::Linear
}

const fn default_reverse_video_cursor_min_contrast() -> f32 {
    2.5
}
