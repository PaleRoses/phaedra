use crate::render_command::RenderCommand;
use crate::render_plan::ExecutionStats;
use crate::termwindow::UIItem;
use mux::pane::PaneId;
use std::sync::Arc;

#[derive(Debug, Default)]
pub struct PostProcessParams {
    pub resolution: [f32; 2],
    pub time: f32,
}

#[derive(Debug, Clone)]
pub struct PaneFrame {
    pub pane_id: PaneId,
    pub is_active: bool,
    pub bounds: phaedra_render_command::RectF,
    pub command_hash: u64,
    pub cache_key: u64,
    pub commands: Arc<[RenderCommand]>,
    pub ui_items: Vec<UIItem>,
    pub last_execution_stats: Option<ExecutionStats>,
    pub skip_streak: u32,
}

#[derive(Debug, Default)]
pub struct ChromeFrame {
    pub tab_bar: Vec<RenderCommand>,
    pub tab_bar_ui_items: Vec<UIItem>,
    pub splits: Vec<RenderCommand>,
    pub split_ui_items: Vec<UIItem>,
    pub borders: Vec<RenderCommand>,
    pub modal: Vec<RenderCommand>,
    pub modal_ui_items: Vec<UIItem>,
}

#[derive(Debug, Default)]
pub struct Frame {
    pub background: Vec<RenderCommand>,
    pub panes: Vec<PaneFrame>,
    pub chrome: ChromeFrame,
    pub postprocess: Option<PostProcessParams>,
    pub ui_items: Vec<UIItem>,
}
