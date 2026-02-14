use crate::color::TabBarStyle;
use phaedra_dynamic::{FromDynamic, ToDynamic};

#[derive(Debug, Clone, FromDynamic, ToDynamic)]
pub struct TabBarConfig {
    #[dynamic(default)]
    pub tab_bar_style: TabBarStyle,
    #[dynamic(default = "default_true")]
    pub enable_tab_bar: bool,
    #[dynamic(default = "default_true")]
    pub use_fancy_tab_bar: bool,
    #[dynamic(default)]
    pub tab_bar_at_bottom: bool,
    #[dynamic(default = "default_true")]
    pub mouse_wheel_scrolls_tabs: bool,
    #[dynamic(default = "default_true")]
    pub show_tab_index_in_tab_bar: bool,
    #[dynamic(default = "default_true")]
    pub show_tabs_in_tab_bar: bool,
    #[dynamic(default = "default_true")]
    pub show_new_tab_button_in_tab_bar: bool,
    #[dynamic(default = "default_true")]
    pub show_close_tab_button_in_tabs: bool,
    #[dynamic(default)]
    pub tab_and_split_indices_are_zero_based: bool,
    #[dynamic(default = "default_tab_max_width")]
    pub tab_max_width: usize,
    #[dynamic(default)]
    pub hide_tab_bar_if_only_one_tab: bool,
    #[dynamic(default)]
    pub switch_to_last_active_tab_when_closing_tab: bool,
}

impl Default for TabBarConfig {
    fn default() -> Self {
        Self {
            tab_bar_style: TabBarStyle::default(),
            enable_tab_bar: default_true(),
            use_fancy_tab_bar: default_true(),
            tab_bar_at_bottom: false,
            mouse_wheel_scrolls_tabs: default_true(),
            show_tab_index_in_tab_bar: default_true(),
            show_tabs_in_tab_bar: default_true(),
            show_new_tab_button_in_tab_bar: default_true(),
            show_close_tab_button_in_tabs: default_true(),
            tab_and_split_indices_are_zero_based: false,
            tab_max_width: default_tab_max_width(),
            hide_tab_bar_if_only_one_tab: false,
            switch_to_last_active_tab_when_closing_tab: false,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_tab_max_width() -> usize {
    16
}
