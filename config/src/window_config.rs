use crate::background::SystemBackdrop;
use crate::color::{IntegratedTitleButtonColor, RgbaColor, WindowFrameConfig};
use crate::config::{WindowCloseConfirmation, WindowContentAlignment, WindowPadding};
use crate::default_win32_acrylic_accent_color;
use phaedra_dynamic::{FromDynamic, ToDynamic};
use phaedra_input_types::{
    IntegratedTitleButton, IntegratedTitleButtonAlignment, IntegratedTitleButtonStyle,
    WindowDecorations,
};

#[derive(Debug, Clone, FromDynamic, ToDynamic)]
pub struct WindowConfig {
    #[dynamic(default)]
    pub window_decorations: WindowDecorations,
    #[dynamic(default = "default_integrated_title_buttons")]
    pub integrated_title_buttons: Vec<IntegratedTitleButton>,
    #[dynamic(default)]
    pub integrated_title_button_alignment: IntegratedTitleButtonAlignment,
    #[dynamic(default)]
    pub integrated_title_button_style: IntegratedTitleButtonStyle,
    #[dynamic(default)]
    pub integrated_title_button_color: IntegratedTitleButtonColor,
    #[dynamic(default)]
    pub window_frame: WindowFrameConfig,
    #[dynamic(default)]
    pub window_padding: WindowPadding,
    #[dynamic(default)]
    pub window_content_alignment: WindowContentAlignment,
    #[dynamic(default)]
    pub window_close_confirmation: WindowCloseConfirmation,
    #[dynamic(default = "default_initial_rows", validate = "validate_row_or_col")]
    pub initial_rows: u16,
    #[dynamic(default = "default_initial_cols", validate = "validate_row_or_col")]
    pub initial_cols: u16,
    #[dynamic(default)]
    pub macos_window_background_blur: i64,
    #[dynamic(default)]
    pub native_macos_fullscreen_mode: bool,
    #[dynamic(default)]
    pub macos_fullscreen_extend_behind_notch: bool,
    #[dynamic(default)]
    pub adjust_window_size_when_changing_font_size: Option<bool>,
    #[dynamic(default = "default_tiling_desktop_environments")]
    pub tiling_desktop_environments: Vec<String>,
    #[dynamic(default)]
    pub use_resize_increments: bool,
    #[dynamic(default = "default_true")]
    pub unzoom_on_switch_pane: bool,
    #[dynamic(default = "default_true")]
    pub quit_when_all_windows_are_closed: bool,
    #[dynamic(default)]
    pub enable_zwlr_output_manager: bool,
    #[dynamic(default)]
    pub win32_system_backdrop: SystemBackdrop,
    #[dynamic(default = "default_win32_acrylic_accent_color")]
    pub win32_acrylic_accent_color: RgbaColor,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            window_decorations: WindowDecorations::default(),
            integrated_title_buttons: default_integrated_title_buttons(),
            integrated_title_button_alignment: IntegratedTitleButtonAlignment::default(),
            integrated_title_button_style: IntegratedTitleButtonStyle::default(),
            integrated_title_button_color: IntegratedTitleButtonColor::default(),
            window_frame: WindowFrameConfig::default(),
            window_padding: WindowPadding::default(),
            window_content_alignment: WindowContentAlignment::default(),
            window_close_confirmation: WindowCloseConfirmation::default(),
            initial_rows: default_initial_rows(),
            initial_cols: default_initial_cols(),
            macos_window_background_blur: 0,
            native_macos_fullscreen_mode: false,
            macos_fullscreen_extend_behind_notch: false,
            adjust_window_size_when_changing_font_size: None,
            tiling_desktop_environments: default_tiling_desktop_environments(),
            use_resize_increments: false,
            unzoom_on_switch_pane: default_true(),
            quit_when_all_windows_are_closed: default_true(),
            enable_zwlr_output_manager: false,
            win32_system_backdrop: SystemBackdrop::default(),
            win32_acrylic_accent_color: default_win32_acrylic_accent_color(),
        }
    }
}

fn default_integrated_title_buttons() -> Vec<IntegratedTitleButton> {
    use IntegratedTitleButton::*;
    vec![Hide, Maximize, Close]
}

fn default_initial_rows() -> u16 {
    24
}

fn default_initial_cols() -> u16 {
    80
}

fn default_tiling_desktop_environments() -> Vec<String> {
    [
        "X11 LG3D",
        "X11 Qtile",
        "X11 awesome",
        "X11 bspwm",
        "X11 dwm",
        "X11 i3",
        "X11 xmonad",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

fn default_true() -> bool {
    true
}

fn validate_row_or_col(value: &u16) -> Result<(), String> {
    if *value < 1 {
        Err("initial_cols and initial_rows must be non-zero".to_string())
    } else {
        Ok(())
    }
}
