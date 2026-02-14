use crate::daemon::DaemonOptions;
use phaedra_dynamic::{FromDynamic, ToDynamic};

#[derive(Debug, Clone, FromDynamic, ToDynamic)]
pub struct MuxConfig {
    #[dynamic(default = "default_ratelimit_line_prefetches_per_second")]
    pub ratelimit_mux_line_prefetches_per_second: u32,
    #[dynamic(default = "default_mux_output_parser_buffer_size")]
    pub mux_output_parser_buffer_size: usize,
    #[dynamic(default = "default_mux_output_parser_coalesce_delay_ms")]
    pub mux_output_parser_coalesce_delay_ms: u64,
    #[dynamic(default)]
    pub daemon_options: DaemonOptions,
}

impl Default for MuxConfig {
    fn default() -> Self {
        Self {
            ratelimit_mux_line_prefetches_per_second: default_ratelimit_line_prefetches_per_second(),
            mux_output_parser_buffer_size: default_mux_output_parser_buffer_size(),
            mux_output_parser_coalesce_delay_ms: default_mux_output_parser_coalesce_delay_ms(),
            daemon_options: DaemonOptions::default(),
        }
    }
}

fn default_mux_output_parser_coalesce_delay_ms() -> u64 {
    3
}

fn default_mux_output_parser_buffer_size() -> usize {
    128 * 1024
}

fn default_ratelimit_line_prefetches_per_second() -> u32 {
    50
}
