use crate::frame::{Frame, PaneFrame};
use crate::render_command::{HsbTransform, QuadMode, RectF, RenderCommand, TextureCoords};
use std::sync::Arc;
use window::color::LinearRgba;

pub trait Lens<S, A> {
    fn view<'a>(&self, source: &'a S) -> &'a A;
    fn over<F: FnOnce(A) -> A>(&self, source: S, f: F) -> S;
}

pub trait Prism<S, A> {
    fn preview(&self, source: &S) -> Option<A>;
    fn review(&self, value: A) -> S;
}

pub trait Traversal<S, A> {
    fn fold<B, F: FnMut(B, &A) -> B>(&self, source: &S, init: B, f: F) -> B;
    fn traverse<F: FnMut(&A) -> A>(&self, source: S, f: F) -> S;
}

pub struct Compose<Outer, Inner>(pub Outer, pub Inner);

#[derive(Debug, Clone)]
pub struct FillRectFields {
    pub layer: usize,
    pub zindex: i8,
    pub rect: RectF,
    pub color: LinearRgba,
    pub hsv: Option<HsbTransform>,
}

#[derive(Debug, Clone)]
pub struct DrawQuadFields {
    pub layer: usize,
    pub zindex: i8,
    pub position: RectF,
    pub texture: TextureCoords,
    pub fg_color: LinearRgba,
    pub alt_color: Option<(LinearRgba, f32)>,
    pub hsv: Option<HsbTransform>,
    pub mode: QuadMode,
}

pub struct PaneBounds;

impl Lens<PaneFrame, RectF> for PaneBounds {
    fn view<'a>(&self, source: &'a PaneFrame) -> &'a RectF {
        &source.bounds
    }

    fn over<F: FnOnce(RectF) -> RectF>(&self, source: PaneFrame, f: F) -> PaneFrame {
        let PaneFrame {
            pane_id,
            is_active,
            bounds,
            command_hash,
            cache_key,
            commands,
            ui_items,
            last_execution_stats,
            skip_streak,
        } = source;
        PaneFrame {
            pane_id,
            is_active,
            bounds: f(bounds),
            command_hash,
            cache_key,
            commands,
            ui_items,
            last_execution_stats,
            skip_streak,
        }
    }
}

pub struct PaneCommands;

impl Lens<PaneFrame, Arc<[RenderCommand]>> for PaneCommands {
    fn view<'a>(&self, source: &'a PaneFrame) -> &'a Arc<[RenderCommand]> {
        &source.commands
    }

    fn over<F: FnOnce(Arc<[RenderCommand]>) -> Arc<[RenderCommand]>>(
        &self,
        source: PaneFrame,
        f: F,
    ) -> PaneFrame {
        let PaneFrame {
            pane_id,
            is_active,
            bounds,
            command_hash,
            cache_key,
            commands,
            ui_items,
            last_execution_stats,
            skip_streak,
        } = source;
        PaneFrame {
            pane_id,
            is_active,
            bounds,
            command_hash,
            cache_key,
            commands: f(commands),
            ui_items,
            last_execution_stats,
            skip_streak,
        }
    }
}

pub struct AsFillRect;

impl Prism<RenderCommand, FillRectFields> for AsFillRect {
    fn preview(&self, source: &RenderCommand) -> Option<FillRectFields> {
        match source {
            RenderCommand::FillRect {
                layer,
                zindex,
                rect,
                color,
                hsv,
            } => Some(FillRectFields {
                layer: *layer,
                zindex: *zindex,
                rect: *rect,
                color: *color,
                hsv: hsv.clone(),
            }),
            _ => None,
        }
    }

    fn review(&self, value: FillRectFields) -> RenderCommand {
        RenderCommand::FillRect {
            layer: value.layer,
            zindex: value.zindex,
            rect: value.rect,
            color: value.color,
            hsv: value.hsv,
        }
    }
}

impl Traversal<RenderCommand, FillRectFields> for AsFillRect {
    fn fold<B, F: FnMut(B, &FillRectFields) -> B>(&self, source: &RenderCommand, init: B, mut f: F) -> B {
        match self.preview(source) {
            Some(fields) => f(init, &fields),
            None => init,
        }
    }

    fn traverse<F: FnMut(&FillRectFields) -> FillRectFields>(
        &self,
        source: RenderCommand,
        mut f: F,
    ) -> RenderCommand {
        match self.preview(&source) {
            Some(fields) => self.review(f(&fields)),
            None => source,
        }
    }
}

pub struct AsDrawQuad;

impl Prism<RenderCommand, DrawQuadFields> for AsDrawQuad {
    fn preview(&self, source: &RenderCommand) -> Option<DrawQuadFields> {
        match source {
            RenderCommand::DrawQuad {
                layer,
                zindex,
                position,
                texture,
                fg_color,
                alt_color,
                hsv,
                mode,
            } => Some(DrawQuadFields {
                layer: *layer,
                zindex: *zindex,
                position: *position,
                texture: texture.clone(),
                fg_color: *fg_color,
                alt_color: *alt_color,
                hsv: hsv.clone(),
                mode: mode.clone(),
            }),
            _ => None,
        }
    }

    fn review(&self, value: DrawQuadFields) -> RenderCommand {
        RenderCommand::DrawQuad {
            layer: value.layer,
            zindex: value.zindex,
            position: value.position,
            texture: value.texture,
            fg_color: value.fg_color,
            alt_color: value.alt_color,
            hsv: value.hsv,
            mode: value.mode,
        }
    }
}

impl Traversal<RenderCommand, DrawQuadFields> for AsDrawQuad {
    fn fold<B, F: FnMut(B, &DrawQuadFields) -> B>(&self, source: &RenderCommand, init: B, mut f: F) -> B {
        match self.preview(source) {
            Some(fields) => f(init, &fields),
            None => init,
        }
    }

    fn traverse<F: FnMut(&DrawQuadFields) -> DrawQuadFields>(
        &self,
        source: RenderCommand,
        mut f: F,
    ) -> RenderCommand {
        match self.preview(&source) {
            Some(fields) => self.review(f(&fields)),
            None => source,
        }
    }
}

pub struct DeepCommands;

fn fold_deep_command<B, F>(command: &RenderCommand, init: B, f: &mut F) -> B
where
    F: FnMut(B, &RenderCommand) -> B,
{
    match command {
        RenderCommand::Batch(commands) => commands
            .iter()
            .fold(init, |acc, nested| fold_deep_command(nested, acc, f)),
        _ => f(init, command),
    }
}

fn traverse_deep_command<F>(command: RenderCommand, f: &mut F) -> RenderCommand
where
    F: FnMut(&RenderCommand) -> RenderCommand,
{
    match command {
        RenderCommand::Batch(commands) => {
            RenderCommand::Batch(commands.into_iter().map(|nested| traverse_deep_command(nested, f)).collect())
        }
        other => f(&other),
    }
}

impl Traversal<Arc<[RenderCommand]>, RenderCommand> for DeepCommands {
    fn fold<B, F: FnMut(B, &RenderCommand) -> B>(
        &self,
        source: &Arc<[RenderCommand]>,
        init: B,
        mut f: F,
    ) -> B {
        source
            .iter()
            .fold(init, |acc, command| fold_deep_command(command, acc, &mut f))
    }

    fn traverse<F: FnMut(&RenderCommand) -> RenderCommand>(
        &self,
        source: Arc<[RenderCommand]>,
        mut f: F,
    ) -> Arc<[RenderCommand]> {
        let transformed: Vec<RenderCommand> = source
            .iter()
            .cloned()
            .map(|command| traverse_deep_command(command, &mut f))
            .collect();
        Arc::from(transformed.into_boxed_slice())
    }
}

impl Traversal<Vec<RenderCommand>, RenderCommand> for DeepCommands {
    fn fold<B, F: FnMut(B, &RenderCommand) -> B>(&self, source: &Vec<RenderCommand>, init: B, mut f: F) -> B {
        source
            .iter()
            .fold(init, |acc, command| fold_deep_command(command, acc, &mut f))
    }

    fn traverse<F: FnMut(&RenderCommand) -> RenderCommand>(
        &self,
        source: Vec<RenderCommand>,
        mut f: F,
    ) -> Vec<RenderCommand> {
        source
            .into_iter()
            .map(|command| traverse_deep_command(command, &mut f))
            .collect()
    }
}

pub struct AllPanes;

impl Traversal<Frame, PaneFrame> for AllPanes {
    fn fold<B, F: FnMut(B, &PaneFrame) -> B>(&self, source: &Frame, init: B, mut f: F) -> B {
        source.panes.iter().fold(init, |acc, pane| f(acc, pane))
    }

    fn traverse<F: FnMut(&PaneFrame) -> PaneFrame>(&self, source: Frame, mut f: F) -> Frame {
        let Frame {
            background,
            panes,
            chrome,
            postprocess,
            ui_items,
        } = source;
        Frame {
            background,
            panes: panes.into_iter().map(|pane| f(&pane)).collect(),
            chrome,
            postprocess,
            ui_items,
        }
    }
}

impl Traversal<Frame, Arc<[RenderCommand]>> for Compose<AllPanes, PaneCommands> {
    fn fold<B, F: FnMut(B, &Arc<[RenderCommand]>) -> B>(&self, source: &Frame, init: B, mut f: F) -> B {
        self.0
            .fold(source, init, |acc, pane| f(acc, self.1.view(pane)))
    }

    fn traverse<F: FnMut(&Arc<[RenderCommand]>) -> Arc<[RenderCommand]>>(
        &self,
        source: Frame,
        mut f: F,
    ) -> Frame {
        self.0
            .traverse(source, |pane| self.1.over(pane.clone(), |commands| f(&commands)))
    }
}

impl<Outer> Traversal<Frame, RenderCommand> for Compose<Outer, DeepCommands>
where
    Outer: Traversal<Frame, Arc<[RenderCommand]>>,
{
    fn fold<B, F: FnMut(B, &RenderCommand) -> B>(&self, source: &Frame, init: B, mut f: F) -> B {
        self.0
            .fold(source, init, |acc, commands| self.1.fold(commands, acc, |inner, command| f(inner, command)))
    }

    fn traverse<F: FnMut(&RenderCommand) -> RenderCommand>(
        &self,
        source: Frame,
        mut f: F,
    ) -> Frame {
        self.0
            .traverse(source, |commands| self.1.traverse(commands.clone(), |command| f(command)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> RectF {
        euclid::rect(x, y, w, h)
    }

    fn draw_quad(x: f32) -> RenderCommand {
        RenderCommand::DrawQuad {
            layer: 0,
            zindex: 0,
            position: rect(x, 0.0, 1.0, 1.0),
            texture: TextureCoords {
                left: 0.0,
                top: 0.0,
                right: 1.0,
                bottom: 1.0,
            },
            fg_color: LinearRgba::with_components(1.0, 1.0, 1.0, 1.0),
            alt_color: None,
            hsv: None,
            mode: QuadMode::Glyph,
        }
    }

    fn fill_rect() -> RenderCommand {
        RenderCommand::FillRect {
            layer: 1,
            zindex: 2,
            rect: rect(1.0, 2.0, 3.0, 4.0),
            color: LinearRgba::with_components(0.2, 0.3, 0.4, 1.0),
            hsv: Some(HsbTransform {
                hue: 10.0,
                saturation: 0.8,
                brightness: 0.9,
            }),
        }
    }

    fn pane(id: usize, commands: Vec<RenderCommand>) -> PaneFrame {
        PaneFrame {
            pane_id: id,
            is_active: true,
            bounds: rect(0.0, 0.0, 10.0, 10.0),
            command_hash: 0,
            cache_key: 0,
            commands: Arc::from(commands.into_boxed_slice()),
            ui_items: Vec::new(),
            last_execution_stats: None,
            skip_streak: 0,
        }
    }

    fn frame_with_panes(panes: Vec<PaneFrame>) -> Frame {
        Frame {
            background: Vec::new(),
            panes,
            chrome: Default::default(),
            postprocess: None,
            ui_items: Vec::new(),
        }
    }

    #[test]
    fn lens_view_over_roundtrip() {
        let optic = PaneBounds;
        let original = pane(7, Vec::new());
        let viewed = *optic.view(&original);
        let shifted = optic.over(original.clone(), |bounds| {
            rect(
                bounds.origin.x + 2.0,
                bounds.origin.y + 3.0,
                bounds.size.width,
                bounds.size.height,
            )
        });
        assert_eq!(optic.view(&shifted).origin.x, viewed.origin.x + 2.0);
        assert_eq!(optic.view(&shifted).origin.y, viewed.origin.y + 3.0);
        let roundtrip = optic.over(shifted, |_| viewed);
        assert_eq!(*optic.view(&roundtrip), viewed);
        assert_eq!(roundtrip.pane_id, original.pane_id);
    }

    #[test]
    fn prism_preview_review_roundtrip() {
        let prism = AsFillRect;
        let command = fill_rect();
        let fields = prism.preview(&command).expect("expected FillRect fields");
        let rebuilt = prism.review(fields.clone());
        match rebuilt {
            RenderCommand::FillRect {
                layer,
                zindex,
                rect,
                color,
                hsv,
            } => {
                assert_eq!(layer, fields.layer);
                assert_eq!(zindex, fields.zindex);
                assert_eq!(rect, fields.rect);
                assert_eq!(color, fields.color);
                let rebuilt_hsv = hsv.map(|v| (v.hue, v.saturation, v.brightness));
                let fields_hsv = fields.hsv.as_ref().map(|v| (v.hue, v.saturation, v.brightness));
                assert_eq!(rebuilt_hsv, fields_hsv);
            }
            _ => panic!("expected FillRect command"),
        }
    }

    #[test]
    fn traversal_fold_counts_draw_quads_in_frame() {
        let pane_a = pane(
            1,
            vec![
                draw_quad(1.0),
                RenderCommand::Batch(vec![draw_quad(2.0), RenderCommand::Batch(vec![draw_quad(3.0)])]),
                fill_rect(),
            ],
        );
        let pane_b = pane(2, vec![fill_rect()]);
        let frame = frame_with_panes(vec![pane_a, pane_b]);
        let all_panes = AllPanes;
        let pane_commands = PaneCommands;
        let deep = DeepCommands;
        let count = all_panes.fold(&frame, 0usize, |acc, pane| {
            deep.fold(pane_commands.view(pane), acc, |inner, command| {
                if matches!(command, RenderCommand::DrawQuad { .. }) {
                    inner + 1
                } else {
                    inner
                }
            })
        });
        assert_eq!(count, 3);
    }

    #[test]
    fn composed_all_panes_pane_commands_and_deep_commands() {
        let pane_a = pane(
            1,
            vec![
                draw_quad(1.0),
                RenderCommand::Batch(vec![draw_quad(4.0), fill_rect()]),
            ],
        );
        let pane_b = pane(2, vec![RenderCommand::Batch(vec![draw_quad(7.0)])]);
        let frame = frame_with_panes(vec![pane_a, pane_b]);
        let optic = Compose(Compose(AllPanes, PaneCommands), DeepCommands);
        let transformed = optic.traverse(frame, |command| match command {
            RenderCommand::DrawQuad {
                layer,
                zindex,
                position,
                texture,
                fg_color,
                alt_color,
                hsv,
                mode,
            } => RenderCommand::DrawQuad {
                layer: *layer,
                zindex: *zindex,
                position: rect(
                    position.origin.x + 10.0,
                    position.origin.y,
                    position.size.width,
                    position.size.height,
                ),
                texture: texture.clone(),
                fg_color: *fg_color,
                alt_color: *alt_color,
                hsv: hsv.clone(),
                mode: mode.clone(),
            },
            other => other.clone(),
        });
        let sum_x = optic.fold(&transformed, 0.0f32, |acc, command| match command {
            RenderCommand::DrawQuad { position, .. } => acc + position.origin.x,
            _ => acc,
        });
        assert_eq!(sum_x, 42.0);
    }
}
