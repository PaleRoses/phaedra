use phaedra_dynamic::{FromDynamic, ToDynamic};

#[derive(Debug, Clone, FromDynamic, ToDynamic)]
pub struct CacheConfig {
    #[dynamic(default = "default_shape_cache_size")]
    pub shape_cache_size: usize,
    #[dynamic(default = "default_line_state_cache_size")]
    pub line_state_cache_size: usize,
    #[dynamic(default = "default_line_quad_cache_size")]
    pub line_quad_cache_size: usize,
    #[dynamic(default = "default_line_to_ele_shape_cache_size")]
    pub line_to_ele_shape_cache_size: usize,
    #[dynamic(default = "default_glyph_cache_image_cache_size")]
    pub glyph_cache_image_cache_size: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            shape_cache_size: default_shape_cache_size(),
            line_state_cache_size: default_line_state_cache_size(),
            line_quad_cache_size: default_line_quad_cache_size(),
            line_to_ele_shape_cache_size: default_line_to_ele_shape_cache_size(),
            glyph_cache_image_cache_size: default_glyph_cache_image_cache_size(),
        }
    }
}

fn default_glyph_cache_image_cache_size() -> usize {
    256
}

fn default_shape_cache_size() -> usize {
    1024
}

fn default_line_state_cache_size() -> usize {
    1024
}

fn default_line_quad_cache_size() -> usize {
    1024
}

fn default_line_to_ele_shape_cache_size() -> usize {
    1024
}
