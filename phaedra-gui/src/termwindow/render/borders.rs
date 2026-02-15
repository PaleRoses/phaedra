use config::observers::*;
use crate::utilsprites::RenderMetrics;
use config::{ConfigHandle, DimensionContext};
use ::window::ULength;

impl crate::TermWindow {
    pub fn get_os_border_impl(
        os_parameters: &Option<window::parameters::Parameters>,
        config: &ConfigHandle,
        dimensions: &crate::Dimensions,
        render_metrics: &RenderMetrics,
    ) -> window::parameters::Border {
        let mut border = os_parameters
            .as_ref()
            .and_then(|p| p.border_dimensions.clone())
            .unwrap_or_default();

        border.left += ULength::new(
            config
                .window_config().window_frame
                .border_left_width
                .evaluate_as_pixels(DimensionContext {
                    dpi: dimensions.dpi as f32,
                    pixel_max: dimensions.pixel_width as f32,
                    pixel_cell: render_metrics.cell_size.width as f32,
                })
                .ceil() as usize,
        );
        border.right += ULength::new(
            config
                .window_config().window_frame
                .border_right_width
                .evaluate_as_pixels(DimensionContext {
                    dpi: dimensions.dpi as f32,
                    pixel_max: dimensions.pixel_width as f32,
                    pixel_cell: render_metrics.cell_size.width as f32,
                })
                .ceil() as usize,
        );
        border.top += ULength::new(
            config
                .window_config().window_frame
                .border_top_height
                .evaluate_as_pixels(DimensionContext {
                    dpi: dimensions.dpi as f32,
                    pixel_max: dimensions.pixel_height as f32,
                    pixel_cell: render_metrics.cell_size.height as f32,
                })
                .ceil() as usize,
        );
        border.bottom += ULength::new(
            config
                .window_config().window_frame
                .border_bottom_height
                .evaluate_as_pixels(DimensionContext {
                    dpi: dimensions.dpi as f32,
                    pixel_max: dimensions.pixel_height as f32,
                    pixel_cell: render_metrics.cell_size.height as f32,
                })
                .ceil() as usize,
        );

        border
    }

    pub fn get_os_border(&self) -> window::parameters::Border {
        Self::get_os_border_impl(
            &self.os_parameters,
            &self.config,
            &self.dimensions,
            &self.render_metrics,
        )
    }
}
