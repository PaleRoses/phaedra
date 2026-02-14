use crate::units::Dimension;
use phaedra_dynamic::{FromDynamic, ToDynamic};

#[derive(Debug, Clone, FromDynamic, ToDynamic)]
pub struct ScrollConfig {
    #[dynamic(
        default = "default_scrollback_lines",
        validate = "validate_scrollback_lines"
    )]
    pub scrollback_lines: usize,
    #[dynamic(default)]
    pub enable_scroll_bar: bool,
    #[dynamic(try_from = "crate::units::PixelUnit", default = "default_half_cell")]
    pub min_scroll_bar_height: Dimension,
    #[dynamic(default = "default_true")]
    pub scroll_to_bottom_on_input: bool,
    #[dynamic(default = "default_alternate_buffer_wheel_scroll_speed")]
    pub alternate_buffer_wheel_scroll_speed: u8,
}

impl Default for ScrollConfig {
    fn default() -> Self {
        Self {
            scrollback_lines: default_scrollback_lines(),
            enable_scroll_bar: false,
            min_scroll_bar_height: default_half_cell(),
            scroll_to_bottom_on_input: default_true(),
            alternate_buffer_wheel_scroll_speed: default_alternate_buffer_wheel_scroll_speed(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_scrollback_lines() -> usize {
    3500
}

const MAX_SCROLLBACK_LINES: usize = 999_999_999;
fn validate_scrollback_lines(value: &usize) -> Result<(), String> {
    if *value > MAX_SCROLLBACK_LINES {
        return Err(format!(
            "Illegal value {value} for scrollback_lines; it must be <= {MAX_SCROLLBACK_LINES}!"
        ));
    }
    Ok(())
}

const fn default_half_cell() -> Dimension {
    Dimension::Cells(0.5)
}

fn default_alternate_buffer_wheel_scroll_speed() -> u8 {
    3
}
