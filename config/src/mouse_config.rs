use crate::config::DroppedFileQuoting;
use crate::keys::Mouse;
use phaedra_dynamic::{FromDynamic, ToDynamic};
use phaedra_input_types::Modifiers;

#[derive(Debug, Clone, FromDynamic, ToDynamic)]
pub struct MouseConfig {
    #[dynamic(default)]
    pub mouse_bindings: Vec<Mouse>,
    #[dynamic(default)]
    pub disable_default_mouse_bindings: bool,
    #[dynamic(default = "default_bypass_mouse_reporting_modifiers")]
    pub bypass_mouse_reporting_modifiers: Modifiers,
    #[dynamic(default = "default_word_boundary")]
    pub selection_word_boundary: String,
    #[dynamic(default)]
    pub quick_select_patterns: Vec<String>,
    #[dynamic(default = "default_alphabet")]
    pub quick_select_alphabet: String,
    #[dynamic(default)]
    pub quick_select_remove_styling: bool,
    #[dynamic(default)]
    pub disable_default_quick_select_patterns: bool,
    #[dynamic(default = "default_true")]
    pub hide_mouse_cursor_when_typing: bool,
    #[dynamic(default)]
    pub swallow_mouse_click_on_pane_focus: bool,
    #[dynamic(default = "default_swallow_mouse_click_on_window_focus")]
    pub swallow_mouse_click_on_window_focus: bool,
    #[dynamic(default)]
    pub pane_focus_follows_mouse: bool,
    #[dynamic(default)]
    pub quote_dropped_files: DroppedFileQuoting,
}

impl Default for MouseConfig {
    fn default() -> Self {
        Self {
            mouse_bindings: vec![],
            disable_default_mouse_bindings: false,
            bypass_mouse_reporting_modifiers: default_bypass_mouse_reporting_modifiers(),
            selection_word_boundary: default_word_boundary(),
            quick_select_patterns: vec![],
            quick_select_alphabet: default_alphabet(),
            quick_select_remove_styling: false,
            disable_default_quick_select_patterns: false,
            hide_mouse_cursor_when_typing: default_true(),
            swallow_mouse_click_on_pane_focus: false,
            swallow_mouse_click_on_window_focus: default_swallow_mouse_click_on_window_focus(),
            pane_focus_follows_mouse: false,
            quote_dropped_files: DroppedFileQuoting::default(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_bypass_mouse_reporting_modifiers() -> Modifiers {
    Modifiers::SHIFT
}

fn default_alphabet() -> String {
    "asdfqwerzxcvjklmiuopghtybn".to_string()
}

fn default_word_boundary() -> String {
    " \t\n{[}]()\"'`".to_string()
}

fn default_swallow_mouse_click_on_window_focus() -> bool {
    cfg!(target_os = "macos")
}
