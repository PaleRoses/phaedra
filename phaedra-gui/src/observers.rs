use crate::termwindow::render::paint::AllowImage;
use crate::termwindow::TermWindow;
use config::ConfigHandle;
use mux::pane::PaneId;
use mux::tab::PositionedPane;
use phaedra_term::StableRowIndex;
use window::Dimensions;

pub trait WindowObserver {
    fn dimensions(&self) -> Dimensions;
    fn is_focused(&self) -> bool;
}

pub trait ConfigObserver {
    fn config(&self) -> &ConfigHandle;
}

pub trait RenderMetricsObserver {
    fn render_metrics(&self) -> &crate::utilsprites::RenderMetrics;
}

pub trait WindowGeometryObserver {
    fn pixel_dimensions(&self) -> (f32, f32);
    fn padding(&self) -> (f32, f32, f32, f32);
}

pub trait PaneLayoutObserver {
    fn get_panes_to_render(&self) -> Vec<PositionedPane>;
    fn get_viewport(&self, pane_id: PaneId) -> Option<StableRowIndex>;
    fn is_zoomed(&self) -> bool;
}

pub trait TransientRenderObserver {
    fn allow_images(&self) -> AllowImage;
    fn shape_generation(&self) -> usize;
    fn created_elapsed_ms(&self) -> u32;
}

pub trait FrameObserver:
    WindowObserver
    + ConfigObserver
    + RenderMetricsObserver
    + WindowGeometryObserver
    + PaneLayoutObserver
    + TransientRenderObserver
{
}

impl WindowObserver for TermWindow {
    fn dimensions(&self) -> Dimensions {
        self.dimensions
    }

    fn is_focused(&self) -> bool {
        self.focused.is_some()
    }
}

impl ConfigObserver for TermWindow {
    fn config(&self) -> &ConfigHandle {
        &self.config
    }
}

impl RenderMetricsObserver for TermWindow {
    fn render_metrics(&self) -> &crate::utilsprites::RenderMetrics {
        &self.render_metrics
    }
}

impl FrameObserver for TermWindow {}
