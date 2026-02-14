use crate::config::{ExitBehavior, ExitBehaviorMessaging};
use crate::keyassignment::SpawnCommand;
use phaedra_dynamic::{FromDynamic, ToDynamic};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, FromDynamic, ToDynamic)]
pub struct LaunchConfig {
    pub default_prog: Option<Vec<String>>,
    #[dynamic(default = "default_gui_startup_args")]
    pub default_gui_startup_args: Vec<String>,
    pub default_cwd: Option<PathBuf>,
    #[dynamic(default)]
    pub launch_menu: Vec<SpawnCommand>,
    #[dynamic(default)]
    pub exit_behavior: ExitBehavior,
    #[dynamic(default)]
    pub exit_behavior_messaging: ExitBehaviorMessaging,
    #[dynamic(default = "default_clean_exits")]
    pub clean_exit_codes: Vec<u32>,
    #[dynamic(default)]
    pub set_environment_variables: HashMap<String, String>,
    #[dynamic(default)]
    pub prefer_to_spawn_tabs: bool,
    #[dynamic(default = "default_term")]
    pub term: String,
    #[dynamic(default)]
    pub default_workspace: Option<String>,
    pub command_palette_rows: Option<usize>,
    #[dynamic(default = "default_stateless_process_list")]
    pub skip_close_confirmation_for_processes_named: Vec<String>,
}

impl Default for LaunchConfig {
    fn default() -> Self {
        Self {
            default_prog: None,
            default_gui_startup_args: default_gui_startup_args(),
            default_cwd: None,
            launch_menu: Vec::new(),
            exit_behavior: ExitBehavior::default(),
            exit_behavior_messaging: ExitBehaviorMessaging::default(),
            clean_exit_codes: default_clean_exits(),
            set_environment_variables: HashMap::new(),
            prefer_to_spawn_tabs: false,
            term: default_term(),
            default_workspace: None,
            command_palette_rows: None,
            skip_close_confirmation_for_processes_named: default_stateless_process_list(),
        }
    }
}

fn default_gui_startup_args() -> Vec<String> {
    vec!["start".to_string()]
}

fn default_clean_exits() -> Vec<u32> {
    vec![]
}

fn default_term() -> String {
    "xterm-256color".into()
}

fn default_stateless_process_list() -> Vec<String> {
    [
        "bash",
        "sh",
        "zsh",
        "fish",
        "tmux",
        "nu",
        "nu.exe",
        "cmd.exe",
        "pwsh.exe",
        "powershell.exe",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}
