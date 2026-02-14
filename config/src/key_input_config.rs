use crate::config::ImePreeditRendering;
use crate::keys::{Key, KeyMapPreference, LeaderKey};
use phaedra_dynamic::{FromDynamic, ToDynamic};
use phaedra_input_types::{Modifiers, UIKeyCapRendering};
use std::collections::HashMap;

#[derive(Debug, Clone, FromDynamic, ToDynamic)]
pub struct KeyInputConfig {
    #[dynamic(default)]
    pub keys: Vec<Key>,
    #[dynamic(default)]
    pub key_tables: HashMap<String, Vec<Key>>,
    pub leader: Option<LeaderKey>,
    #[dynamic(default)]
    pub disable_default_key_bindings: bool,
    #[dynamic(default)]
    pub debug_key_events: bool,
    #[dynamic(default)]
    pub send_composed_key_when_left_alt_is_pressed: bool,
    #[dynamic(default = "default_true")]
    pub send_composed_key_when_right_alt_is_pressed: bool,
    #[dynamic(default = "default_macos_forward_mods")]
    pub macos_forward_to_ime_modifier_mask: Modifiers,
    #[dynamic(default)]
    pub treat_left_ctrlalt_as_altgr: bool,
    #[dynamic(default = "default_swap_backspace_and_delete")]
    pub swap_backspace_and_delete: bool,
    #[dynamic(default = "default_true")]
    pub use_ime: bool,
    #[dynamic(default)]
    pub xim_im_name: Option<String>,
    #[dynamic(default)]
    pub ime_preedit_rendering: ImePreeditRendering,
    #[dynamic(default = "default_true")]
    pub use_dead_keys: bool,
    #[dynamic(default)]
    pub enable_csi_u_key_encoding: bool,
    #[dynamic(default)]
    pub key_map_preference: KeyMapPreference,
    #[dynamic(default)]
    pub ui_key_cap_rendering: UIKeyCapRendering,
    #[dynamic(default = "default_num_alphabet")]
    pub launcher_alphabet: String,
}

impl Default for KeyInputConfig {
    fn default() -> Self {
        Self {
            keys: Vec::new(),
            key_tables: HashMap::new(),
            leader: None,
            disable_default_key_bindings: false,
            debug_key_events: false,
            send_composed_key_when_left_alt_is_pressed: false,
            send_composed_key_when_right_alt_is_pressed: default_true(),
            macos_forward_to_ime_modifier_mask: default_macos_forward_mods(),
            treat_left_ctrlalt_as_altgr: false,
            swap_backspace_and_delete: default_swap_backspace_and_delete(),
            use_ime: default_true(),
            xim_im_name: None,
            ime_preedit_rendering: ImePreeditRendering::default(),
            use_dead_keys: default_true(),
            enable_csi_u_key_encoding: false,
            key_map_preference: KeyMapPreference::default(),
            ui_key_cap_rendering: UIKeyCapRendering::default(),
            launcher_alphabet: default_num_alphabet(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_swap_backspace_and_delete() -> bool {
    false
}

fn default_macos_forward_mods() -> Modifiers {
    Modifiers::SHIFT
}

fn default_num_alphabet() -> String {
    "1234567890abcdefghilmnopqrstuvwxyz".to_string()
}
