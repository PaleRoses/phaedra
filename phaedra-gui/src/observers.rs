use crate::termwindow::TermWindow;
use config::ConfigHandle;
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
