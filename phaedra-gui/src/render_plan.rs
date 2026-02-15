use phaedra_render_command::RectF;

#[derive(Debug, Clone)]
pub struct ScissorRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl ScissorRect {
    pub fn from_pane_bounds(bounds: &RectF, viewport_width: u32, viewport_height: u32) -> Self {
        let x = bounds.origin.x.max(0.0) as u32;
        let y = bounds.origin.y.max(0.0) as u32;
        let right = (bounds.origin.x + bounds.size.width).min(viewport_width as f32) as u32;
        let bottom = (bounds.origin.y + bounds.size.height).min(viewport_height as f32) as u32;
        Self {
            x,
            y,
            width: right.saturating_sub(x),
            height: bottom.saturating_sub(y),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LayerQuadSnapshot {
    pub zindex: i8,
    pub sub_idx: usize,
    pub quad_count: usize,
}

#[derive(Debug, Clone)]
pub struct QuadRange {
    pub start: Vec<LayerQuadSnapshot>,
    pub end: Vec<LayerQuadSnapshot>,
}

#[derive(Debug, Clone, Copy)]
pub struct ExecutionStats {
    pub quads_emitted: usize,
    pub fills_emitted: usize,
    pub draws_emitted: usize,
    pub overdraw_positions: usize,
}

#[derive(Debug, Clone)]
pub enum SectionOutcome {
    Skipped,
    Executed { stats: ExecutionStats },
}

#[derive(Debug, Clone)]
pub struct CofreeContext {
    pub total_quads_emitted: usize,
    pub sections_processed: usize,
    pub sections_skipped: usize,
    pub skip_streak: usize,
    pub prior_outcomes: Vec<SectionOutcome>,
}

impl CofreeContext {
    pub fn new() -> Self {
        Self {
            total_quads_emitted: 0,
            sections_processed: 0,
            sections_skipped: 0,
            skip_streak: 0,
            prior_outcomes: Vec::new(),
        }
    }

    pub fn advance(&mut self, outcome: SectionOutcome) {
        self.sections_processed += 1;
        match &outcome {
            SectionOutcome::Skipped => {
                self.sections_skipped += 1;
                self.skip_streak += 1;
            }
            SectionOutcome::Executed { stats } => {
                self.total_quads_emitted += stats.quads_emitted;
                self.skip_streak = 0;
            }
        }
        self.prior_outcomes.push(outcome);
    }

    pub fn skip_rate(&self) -> f64 {
        if self.sections_processed == 0 {
            0.0
        } else {
            self.sections_skipped as f64 / self.sections_processed as f64
        }
    }
}

#[derive(Debug, Clone)]
pub struct RenderSection {
    pub scissor: Option<ScissorRect>,
    pub content_hash: u64,
    pub quad_range: QuadRange,
    pub skippable: bool,
    pub stats: Option<ExecutionStats>,
}

#[derive(Debug)]
pub struct RenderPlan {
    pub sections: Vec<RenderSection>,
    pub viewport_width: u32,
    pub viewport_height: u32,
}

impl RenderPlan {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            sections: Vec::new(),
            viewport_width: width,
            viewport_height: height,
        }
    }

    pub fn pane_section_count(&self) -> usize {
        self.sections.iter().filter(|section| section.scissor.is_some()).count()
    }

    pub fn skippable_pane_section_count(&self) -> usize {
        self.sections
            .iter()
            .filter(|section| section.scissor.is_some() && section.skippable)
            .count()
    }
}

pub fn quad_count_for_snapshot(
    snapshots: &[LayerQuadSnapshot],
    zindex: i8,
    sub_idx: usize,
) -> usize {
    snapshots
        .iter()
        .find(|snapshot| snapshot.zindex == zindex && snapshot.sub_idx == sub_idx)
        .map(|snapshot| snapshot.quad_count)
        .unwrap_or(0)
}

pub fn snapshot_layers(render_state: &crate::renderstate::RenderState) -> Vec<LayerQuadSnapshot> {
    let layers = render_state.layers.borrow();
    let mut snaps = Vec::new();
    for layer in layers.iter() {
        for sub_idx in 0..3 {
            snaps.push(LayerQuadSnapshot {
                zindex: layer.zindex(),
                sub_idx,
                quad_count: layer.vb.borrow()[sub_idx].current_quad_count(),
            });
        }
    }
    snaps
}
