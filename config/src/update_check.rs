use phaedra_dynamic::{FromDynamic, ToDynamic};

#[derive(Debug, Clone, FromDynamic, ToDynamic)]
pub struct UpdateConfig {
    #[dynamic(default = "default_check_for_updates")]
    pub check_for_updates: bool,
    #[dynamic(default = "default_update_interval")]
    pub check_for_updates_interval_seconds: u64,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            check_for_updates: default_check_for_updates(),
            check_for_updates_interval_seconds: default_update_interval(),
        }
    }
}

fn default_check_for_updates() -> bool {
    cfg!(not(feature = "distro-defaults"))
}

fn default_update_interval() -> u64 {
    86_400
}
