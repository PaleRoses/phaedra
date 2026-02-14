use phaedra_dynamic::{FromDynamic, ToDynamic};

#[derive(Debug, Clone, FromDynamic, ToDynamic)]
pub struct RuntimeConfig {
    #[dynamic(default)]
    pub log_unknown_escape_sequences: bool,
    #[dynamic(default)]
    pub periodic_stat_logging: u64,
    #[dynamic(default = "default_true")]
    pub automatically_reload_config: bool,
    #[dynamic(default = "default_status_update_interval")]
    pub status_update_interval: u64,
    #[dynamic(default = "default_anim_fps")]
    pub animation_fps: u8,
    #[dynamic(default = "default_ulimit_nofile")]
    pub ulimit_nofile: u64,
    #[dynamic(default = "default_ulimit_nproc")]
    pub ulimit_nproc: u64,
    #[dynamic(default = "default_one")]
    pub palette_max_key_assigments_for_action: usize,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            log_unknown_escape_sequences: false,
            periodic_stat_logging: 0,
            automatically_reload_config: default_true(),
            status_update_interval: default_status_update_interval(),
            animation_fps: default_anim_fps(),
            ulimit_nofile: default_ulimit_nofile(),
            ulimit_nproc: default_ulimit_nproc(),
            palette_max_key_assigments_for_action: default_one(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_one() -> usize {
    1
}

fn default_ulimit_nofile() -> u64 {
    2048
}

fn default_ulimit_nproc() -> u64 {
    2048
}

fn default_anim_fps() -> u8 {
    10
}

fn default_status_update_interval() -> u64 {
    1_000
}
