use crate::frame::{ChromeFrame, Frame, PaneFrame};
use crate::quad::{QuadTrait, TripleLayerQuadAllocatorTrait};
use crate::render_command::{HsbTransform as CmdHsbTransform, QuadMode, RenderCommand};
use crate::render_plan::{snapshot_layers, QuadRange, RenderPlan, RenderSection, ScissorRect};
use crate::renderstate::RenderState;
use mux::pane::PaneId;
use std::collections::HashMap;
use ::window::bitmaps::TextureRect;

pub fn execute_frame(
    frame: &Frame,
    render_state: &RenderState,
    pixel_dims: (f32, f32),
    prev_content_hashes: &HashMap<PaneId, u64>,
) -> anyhow::Result<RenderPlan> {
    let left_offset = pixel_dims.0 / 2.0;
    let top_offset = pixel_dims.1 / 2.0;
    let viewport_width = pixel_dims.0.max(0.0) as u32;
    let viewport_height = pixel_dims.1.max(0.0) as u32;
    let filled_box = render_state.util_sprites.filled_box.texture_coords();
    let mut render_plan = RenderPlan::new(viewport_width, viewport_height);

    let background_start = snapshot_layers(render_state);
    execute_commands(
        &frame.background,
        render_state,
        left_offset,
        top_offset,
        &filled_box,
    )?;
    let background_end = snapshot_layers(render_state);
    render_plan.sections.push(RenderSection {
        scissor: None,
        content_hash: 0,
        quad_range: QuadRange {
            start: background_start,
            end: background_end,
        },
        skippable: false,
    });

    for pane_frame in &frame.panes {
        let skippable = prev_content_hashes
            .get(&pane_frame.pane_id)
            .map_or(false, |previous| *previous == pane_frame.content_hash);
        let pane_start = snapshot_layers(render_state);
        execute_pane_frame(pane_frame, render_state, left_offset, top_offset, &filled_box)?;
        let pane_end = snapshot_layers(render_state);
        render_plan.sections.push(RenderSection {
            scissor: Some(ScissorRect::from_pane_bounds(
                &pane_frame.bounds,
                viewport_width,
                viewport_height,
            )),
            content_hash: pane_frame.content_hash,
            quad_range: QuadRange {
                start: pane_start,
                end: pane_end,
            },
            skippable,
        });
    }

    let chrome_start = snapshot_layers(render_state);
    execute_chrome_frame(
        &frame.chrome,
        render_state,
        left_offset,
        top_offset,
        &filled_box,
    )?;
    let chrome_end = snapshot_layers(render_state);
    render_plan.sections.push(RenderSection {
        scissor: None,
        content_hash: 0,
        quad_range: QuadRange {
            start: chrome_start,
            end: chrome_end,
        },
        skippable: false,
    });

    let pane_section_count = render_plan.pane_section_count();
    let skippable_pane_section_count = render_plan.skippable_pane_section_count();
    log::trace!(
        "render plan: {}/{} pane sections skippable",
        skippable_pane_section_count,
        pane_section_count
    );

    Ok(render_plan)
}

fn execute_pane_frame(
    pane_frame: &PaneFrame,
    render_state: &RenderState,
    left_offset: f32,
    top_offset: f32,
    filled_box: &TextureRect,
) -> anyhow::Result<()> {
    execute_commands(
        &pane_frame.commands,
        render_state,
        left_offset,
        top_offset,
        filled_box,
    )
}

fn execute_chrome_frame(
    chrome_frame: &ChromeFrame,
    render_state: &RenderState,
    left_offset: f32,
    top_offset: f32,
    filled_box: &TextureRect,
) -> anyhow::Result<()> {
    execute_commands(
        &chrome_frame.tab_bar,
        render_state,
        left_offset,
        top_offset,
        filled_box,
    )?;
    execute_commands(
        &chrome_frame.splits,
        render_state,
        left_offset,
        top_offset,
        filled_box,
    )?;
    execute_commands(
        &chrome_frame.borders,
        render_state,
        left_offset,
        top_offset,
        filled_box,
    )?;
    execute_commands(
        &chrome_frame.modal,
        render_state,
        left_offset,
        top_offset,
        filled_box,
    )
}

fn execute_commands(
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

fn to_config_hsb_transform(hsv: &Option<CmdHsbTransform>) -> Option<config::HsbTransform> {
    hsv.as_ref().map(|value| config::HsbTransform {
        hue: value.hue,
        saturation: value.saturation,
        brightness: value.brightness,
    })
}
