use crate::frontend::FrontEndSelection;
use crate::{GpuInfo, WebGpuPowerPreference};
use phaedra_dynamic::{FromDynamic, ToDynamic};
use std::path::PathBuf;

#[derive(Debug, Clone, FromDynamic, ToDynamic)]
pub struct GpuConfig {
    #[dynamic(default)]
    pub front_end: FrontEndSelection,
    #[dynamic(default)]
    pub webgpu_power_preference: WebGpuPowerPreference,
    #[dynamic(default)]
    pub webgpu_force_fallback_adapter: bool,
    #[dynamic(default)]
    pub webgpu_preferred_adapter: Option<GpuInfo>,
    #[dynamic(default)]
    pub webgpu_shader: Option<PathBuf>,
    #[dynamic(default = "default_webgpu_shader_fps")]
    pub webgpu_shader_fps: u8,
    #[dynamic(default = "default_max_fps")]
    pub max_fps: u64,
}

impl Default for GpuConfig {
    fn default() -> Self {
        Self {
            front_end: FrontEndSelection::default(),
            webgpu_power_preference: WebGpuPowerPreference::default(),
            webgpu_force_fallback_adapter: false,
            webgpu_preferred_adapter: None,
            webgpu_shader: None,
            webgpu_shader_fps: default_webgpu_shader_fps(),
            max_fps: default_max_fps(),
        }
    }
}

fn default_webgpu_shader_fps() -> u8 {
    0
}

fn default_max_fps() -> u64 {
    60
}
