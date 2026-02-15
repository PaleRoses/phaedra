use crate::quad::{QuadTrait, TripleLayerQuadAllocatorTrait};
use crate::render_command::{HsbTransform as CmdHsbTransform, QuadMode, RenderCommand};
use crate::render_plan::ExecutionStats;
use crate::renderstate::RenderState;
use std::collections::HashSet;
use ::window::bitmaps::TextureRect;

pub struct ExecutionHistory {
    pub quads_emitted: usize,
    pub fills_emitted: usize,
    pub draws_emitted: usize,
    pub overdraw_positions: usize,
    position_set: HashSet<[u32; 4]>,
}

impl ExecutionHistory {
    pub fn new() -> Self {
        Self {
            quads_emitted: 0,
            fills_emitted: 0,
            draws_emitted: 0,
            overdraw_positions: 0,
            position_set: HashSet::new(),
        }
    }

    pub fn stats(&self) -> ExecutionStats {
        ExecutionStats {
            quads_emitted: self.quads_emitted,
            fills_emitted: self.fills_emitted,
            draws_emitted: self.draws_emitted,
            overdraw_positions: self.overdraw_positions,
        }
    }

    fn mark_position(&mut self, position: [u32; 4]) {
        if self.position_set.contains(&position) {
            self.overdraw_positions += 1;
        }
        self.position_set.insert(position);
    }
}

pub fn execute_commands(
    commands: &[RenderCommand],
    render_state: &RenderState,
    left_offset: f32,
    top_offset: f32,
    filled_box: &TextureRect,
) -> anyhow::Result<()> {
    for cmd in commands {
        execute_command(cmd, render_state, left_offset, top_offset, filled_box)?;
    }
    Ok(())
}

pub fn execute_commands_with_history(
    commands: &[RenderCommand],
    render_state: &RenderState,
    left_offset: f32,
    top_offset: f32,
    filled_box: &TextureRect,
) -> anyhow::Result<ExecutionHistory> {
    let mut history = ExecutionHistory::new();
    execute_commands_with_history_mut(
        commands,
        render_state,
        left_offset,
        top_offset,
        filled_box,
        &mut history,
    )?;
    Ok(history)
}

fn execute_commands_with_history_mut(
    commands: &[RenderCommand],
    render_state: &RenderState,
    left_offset: f32,
    top_offset: f32,
    filled_box: &TextureRect,
    history: &mut ExecutionHistory,
) -> anyhow::Result<()> {
    for cmd in commands {
        execute_command_with_history(
            cmd,
            render_state,
            left_offset,
            top_offset,
            filled_box,
            history,
        )?;
    }
    Ok(())
}

fn execute_command(
    cmd: &RenderCommand,
    render_state: &RenderState,
    left_offset: f32,
    top_offset: f32,
    filled_box: &TextureRect,
) -> anyhow::Result<()> {
    match cmd {
        RenderCommand::Clear { .. }
        | RenderCommand::SetClipRect(_)
        | RenderCommand::BeginPostProcess
        | RenderCommand::Nop => Ok(()),
        RenderCommand::Batch(commands) => execute_commands(
            commands,
            render_state,
            left_offset,
            top_offset,
            filled_box,
        ),
        RenderCommand::FillRect {
            layer,
            zindex,
            rect,
            color,
            hsv,
        } => {
            let render_layer = render_state.layer_for_zindex(*zindex)?;
            let mut layers = render_layer.quad_allocator();
            let mut quad = layers.allocate(*layer)?;

            quad.set_position(
                rect.min_x() - left_offset,
                rect.min_y() - top_offset,
                rect.max_x() - left_offset,
                rect.max_y() - top_offset,
            );
            quad.set_texture_discrete(
                filled_box.min_x(),
                filled_box.max_x(),
                filled_box.min_y(),
                filled_box.max_y(),
            );
            quad.set_is_background();
            quad.set_fg_color(color.clone());
            quad.set_hsv(to_config_hsb_transform(hsv));

            Ok(())
        }
        RenderCommand::DrawQuad {
            layer,
            zindex,
            position,
            texture,
            fg_color,
            alt_color,
            hsv,
            mode,
        } => {
            let render_layer = render_state.layer_for_zindex(*zindex)?;
            let mut layers = render_layer.quad_allocator();
            let mut quad = layers.allocate(*layer)?;

            quad.set_position(
                position.min_x() - left_offset,
                position.min_y() - top_offset,
                position.max_x() - left_offset,
                position.max_y() - top_offset,
            );
            quad.set_texture_discrete(texture.left, texture.right, texture.top, texture.bottom);
            quad.set_fg_color(fg_color.clone());

            if let Some((alt, mix)) = alt_color {
                quad.set_alt_color_and_mix_value(alt.clone(), *mix);
            }

            quad.set_hsv(to_config_hsb_transform(hsv));

            match mode {
                QuadMode::Glyph => quad.set_has_color(false),
                QuadMode::ColorEmoji => quad.set_has_color(true),
                QuadMode::BackgroundImage => quad.set_is_background_image(),
                QuadMode::SolidColor => quad.set_is_background(),
                QuadMode::GrayScale => quad.set_grayscale(),
            }

            Ok(())
        }
    }
}

fn execute_command_with_history(
    cmd: &RenderCommand,
    render_state: &RenderState,
    left_offset: f32,
    top_offset: f32,
    filled_box: &TextureRect,
    history: &mut ExecutionHistory,
) -> anyhow::Result<()> {
    match cmd {
        RenderCommand::Batch(commands) => execute_commands_with_history_mut(
            commands,
            render_state,
            left_offset,
            top_offset,
            filled_box,
            history,
        ),
        RenderCommand::FillRect { rect, .. } => {
            let min_x = rect.min_x() - left_offset;
            let min_y = rect.min_y() - top_offset;
            let max_x = rect.max_x() - left_offset;
            let max_y = rect.max_y() - top_offset;
            history.mark_position(position_fingerprint(min_x, min_y, max_x, max_y));
            execute_command(cmd, render_state, left_offset, top_offset, filled_box)?;
            history.fills_emitted += 1;
            history.quads_emitted += 1;
            Ok(())
        }
        RenderCommand::DrawQuad { position, .. } => {
            let min_x = position.min_x() - left_offset;
            let min_y = position.min_y() - top_offset;
            let max_x = position.max_x() - left_offset;
            let max_y = position.max_y() - top_offset;
            history.mark_position(position_fingerprint(min_x, min_y, max_x, max_y));
            execute_command(cmd, render_state, left_offset, top_offset, filled_box)?;
            history.draws_emitted += 1;
            history.quads_emitted += 1;
            Ok(())
        }
        _ => execute_command(cmd, render_state, left_offset, top_offset, filled_box),
    }
}

fn position_fingerprint(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> [u32; 4] {
    [min_x.to_bits(), min_y.to_bits(), max_x.to_bits(), max_y.to_bits()]
}

fn to_config_hsb_transform(hsv: &Option<CmdHsbTransform>) -> Option<config::HsbTransform> {
    hsv.as_ref().map(|value| config::HsbTransform {
        hue: value.hue,
        saturation: value.saturation,
        brightness: value.brightness,
    })
}
