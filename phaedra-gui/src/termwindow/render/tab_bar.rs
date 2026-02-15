use config::observers::*;
use crate::utilsprites::RenderMetrics;
use config::ConfigHandle;

impl crate::TermWindow {
    pub fn tab_bar_pixel_height_impl(
        config: &ConfigHandle,
        fontconfig: &phaedra_font::FontConfiguration,
        render_metrics: &RenderMetrics,
    ) -> anyhow::Result<f32> {
        if config.tab_bar().use_fancy_tab_bar {
            let font = fontconfig.title_font()?;
            Ok((font.metrics().cell_height.get() as f32 * 1.75).ceil())
        } else {
            Ok(render_metrics.cell_size.height as f32)
        }
    }

    pub fn tab_bar_pixel_height(&self) -> anyhow::Result<f32> {
        Self::tab_bar_pixel_height_impl(&self.config, &self.fonts, &self.render_metrics)
    }
}
