use crate::termwindow::TermWindowNotif;
use crate::execute_render::{execute_commands, execute_commands_with_history};
use crate::render_plan::{
    quad_count_for_snapshot, snapshot_layers, CofreeContext, QuadRange, RenderPlan, RenderSection,
    ScissorRect, SectionOutcome,
};
use config::observers::*;
use mux::pane::TerminalView;
use ::window::bitmaps::atlas::OutOfTextureSpace;
use ::window::WindowOps;
use smol::Timer;
use std::time::{Duration, Instant};
use phaedra_font::ClearShapeCache;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllowImage {
    Yes,
    Scale(usize),
    No,
}

fn frame_has_buffers_for_range(
    frame: &crate::renderstate::FrameBuffers,
    quad_range: &QuadRange,
) -> bool {
    quad_range.end.iter().all(|end_snapshot| {
        let start_quad =
            quad_count_for_snapshot(&quad_range.start, end_snapshot.zindex, end_snapshot.sub_idx);
        end_snapshot.quad_count <= start_quad
            || frame
                .buffer(end_snapshot.zindex, end_snapshot.sub_idx)
                .is_some()
    })
}

fn advance_quad_counts_for_range(
    render_state: &crate::renderstate::RenderState,
    quad_range: &QuadRange,
) -> anyhow::Result<()> {
    for end_snapshot in &quad_range.end {
        let start_quad =
            quad_count_for_snapshot(&quad_range.start, end_snapshot.zindex, end_snapshot.sub_idx);
        if end_snapshot.quad_count <= start_quad {
            continue;
        }
        let quad_delta = end_snapshot.quad_count - start_quad;
        let render_layer = render_state.layer_for_zindex(end_snapshot.zindex)?;
        render_layer.vb.borrow()[end_snapshot.sub_idx].advance_quad_count(quad_delta);
    }
    Ok(())
}

impl crate::TermWindow {
    pub fn paint_impl(&mut self) {
        self.num_frames += 1;
        // If nothing on screen needs animating, then we can avoid
        // invalidating as frequently
        *self.has_animation.borrow_mut() = None;
        // Start with the assumption that we should allow images to render
        self.allow_images = AllowImage::Yes;

        let start = Instant::now();

        {
            let diff = start.duration_since(self.last_fps_check_time);
            if diff > Duration::from_secs(1) {
                let seconds = diff.as_secs_f32();
                self.fps = self.num_frames as f32 / seconds;
                self.num_frames = 0;
                self.last_fps_check_time = start;
            }
        }

        'pass: for pass in 0.. {
            match self.paint_pass() {
                Ok(_) => match self.render_state.as_mut().unwrap().allocated_more_quads() {
                    Ok(allocated) => {
                        if !allocated {
                            break 'pass;
                        }
                        self.invalidate_fancy_tab_bar();
                        self.invalidate_modal();
                    }
                    Err(err) => {
                        log::error!("{:#}", err);
                        break 'pass;
                    }
                },
                Err(err) => {
                    if let Some(&OutOfTextureSpace {
                        size: Some(size),
                        current_size,
                    }) = err.root_cause().downcast_ref::<OutOfTextureSpace>()
                    {
                        let result = if pass == 0 {
                            // Let's try clearing out the atlas and trying again
                            // self.clear_texture_atlas()
                            log::trace!("recreate_texture_atlas");
                            self.recreate_texture_atlas(Some(current_size))
                        } else {
                            log::trace!("grow texture atlas to {}", size);
                            self.recreate_texture_atlas(Some(size))
                        };
                        self.invalidate_fancy_tab_bar();
                        self.invalidate_modal();

                        if let Err(err) = result {
                            self.allow_images = match self.allow_images {
                                AllowImage::Yes => AllowImage::Scale(2),
                                AllowImage::Scale(2) => AllowImage::Scale(4),
                                AllowImage::Scale(4) => AllowImage::Scale(8),
                                AllowImage::Scale(8) => AllowImage::No,
                                AllowImage::No | _ => {
                                    log::error!(
                                        "Failed to {} texture: {}",
                                        if pass == 0 { "clear" } else { "resize" },
                                        err
                                    );
                                    break 'pass;
                                }
                            };

                            log::info!(
                                "Not enough texture space ({:#}); \
                                     will retry render with {:?}",
                                err,
                                self.allow_images,
                            );
                        }
                    } else if err.root_cause().downcast_ref::<ClearShapeCache>().is_some() {
                        self.invalidate_fancy_tab_bar();
                        self.invalidate_modal();
                        self.shape_generation += 1;
                        self.shape_cache.borrow_mut().clear();
                        self.line_to_ele_shape_cache.borrow_mut().clear();
                    } else {
                        log::error!("paint_pass failed: {:#}", err);
                        break 'pass;
                    }
                }
            }
        }
        log::debug!("paint_impl before call_draw elapsed={:?}", start.elapsed());

        self.call_draw().ok();
        self.last_frame_duration = start.elapsed();
        log::debug!(
            "paint_impl elapsed={:?}, fps={}",
            self.last_frame_duration,
            self.fps
        );
        metrics::histogram!("gui.paint.impl").record(self.last_frame_duration);
        metrics::histogram!("gui.paint.impl.rate").record(1.);

        // Schedule continuous rendering for animated shaders
        if let Some(ref webgpu) = self.webgpu {
            if webgpu.has_postprocess() {
                let fps = self.config.gpu().webgpu_shader_fps;
                if fps > 0 {
                    let frame_interval = Duration::from_millis(1000 / fps as u64);
                    let next_frame = Instant::now() + frame_interval;
                    let mut has_anim = self.has_animation.borrow_mut();
                    match *has_anim {
                        None => {
                            *has_anim = Some(next_frame);
                        }
                        Some(existing) if next_frame < existing => {
                            *has_anim = Some(next_frame);
                        }
                        _ => {}
                    }
                }
            }
        }

        // If self.has_animation is some, then the last render detected
        // image attachments with multiple frames, so we also need to
        // invalidate the viewport when the next frame is due
        if self.focused.is_some() {
            if let Some(next_due) = *self.has_animation.borrow() {
                let prior = self.scheduled_animation.borrow_mut().take();
                match prior {
                    Some(prior) if prior <= next_due => {
                        // Already due before that time
                    }
                    _ => {
                        self.scheduled_animation.borrow_mut().replace(next_due);
                        let window = self.window.clone().take().unwrap();
                        promise::spawn::spawn(async move {
                            Timer::at(next_due).await;
                            let win = window.clone();
                            window.notify(TermWindowNotif::Apply(Box::new(move |tw| {
                                tw.scheduled_animation.borrow_mut().take();
                                win.invalidate();
                            })));
                        })
                        .detach();
                    }
                }
            }
        }
    }

    pub fn paint_pass(&mut self) -> anyhow::Result<()> {
        {
            let gl_state = self.render_state.as_ref().unwrap();
            for layer in gl_state.layers.borrow().iter() {
                layer.clear_quad_allocation();
            }
        }
        self.ui_items.clear();
        self.render_plan = None;

        let panes = self.get_panes_to_render();
        let focused = self.focused.is_some();
        for pos in &panes {
            if pos.is_active {
                self.update_text_cursor(pos);
                if focused {
                    pos.pane.advise_focus();
                    mux::Mux::get().record_focus_for_current_identity(pos.pane.pane_id());
                }
            }
        }

        let render_state = self.render_state.as_ref().unwrap();
        let pixel_dims = (
            self.dimensions.pixel_width as f32,
            self.dimensions.pixel_height as f32,
        );
        let left_offset = pixel_dims.0 / 2.0;
        let top_offset = pixel_dims.1 / 2.0;
        let viewport_width = pixel_dims.0.max(0.0) as u32;
        let viewport_height = pixel_dims.1.max(0.0) as u32;
        let filled_box = render_state.util_sprites.filled_box.texture_coords();
        let mut plan = RenderPlan::new(viewport_width, viewport_height);
        let mut ui_items = Vec::new();

        let background = self.describe_window_background(&panes)?;
        let background_start = snapshot_layers(render_state);
        execute_commands(
            &background,
            render_state,
            left_offset,
            top_offset,
            &filled_box,
        )?;
        let background_end = snapshot_layers(render_state);
        plan.sections.push(RenderSection {
            scissor: None,
            content_hash: 0,
            quad_range: QuadRange {
                start: background_start,
                end: background_end,
            },
            skippable: false,
            stats: None,
        });

        let mut new_pane_frames = std::collections::HashMap::with_capacity(panes.len());
        let previous_frame = render_state.prev_frame_buffers.borrow();
        let mut pane_skip_chain_valid = true;
        let mut cofree = CofreeContext::new();

        for pos in &panes {
            let pane_id = pos.pane.pane_id();
            let snapshot = pos.pane.snapshot_for_render(self.get_viewport(pane_id));
            let terminal_hash = snapshot.content_hash();
            let cache_key = self.pane_describe_cache_key(pane_id, pos, terminal_hash);
            let prior = self.prev_pane_frames.get(&pane_id);
            let prior_skip_streak = prior.map_or(0, |frame| frame.skip_streak);

            let (mut pane_frame, candidate_skippable) = match prior {
                Some(cached) if cached.cache_key == cache_key => {
                    let mut frame = cached.clone();
                    frame.skip_streak = prior_skip_streak.saturating_add(1);
                    log::trace!(
                        "pane {pane_id}: chrono skip (seed unchanged, skip_streak={}, intra_skip_streak={})",
                        frame.skip_streak,
                        cofree.skip_streak
                    );
                    (frame, true)
                }
                _ => {
                    log::trace!(
                        "pane {pane_id}: chrono describe (prior_skip_streak={}, intra_skip_streak={})",
                        prior_skip_streak,
                        cofree.skip_streak
                    );
                    (self.describe_pane_with_snapshot(pos, snapshot, cache_key)?, false)
                }
            };

            let prior_quad_range = if candidate_skippable && pane_skip_chain_valid {
                previous_frame
                    .as_ref()
                    .and_then(|frame| frame.section_ranges.get(plan.sections.len()).cloned())
                    .filter(|range| {
                        previous_frame
                            .as_ref()
                            .map(|frame| frame_has_buffers_for_range(frame, range))
                            .unwrap_or(false)
                    })
            } else {
                None
            };

            let pane_start = snapshot_layers(render_state);
            let outcome = if let Some(prior_quad_range) = prior_quad_range.as_ref() {
                advance_quad_counts_for_range(render_state, prior_quad_range)?;
                SectionOutcome::Skipped
            } else {
                let history = execute_commands_with_history(
                    &pane_frame.commands,
                    render_state,
                    left_offset,
                    top_offset,
                    &filled_box,
                )?;
                let stats = history.stats();
                pane_frame.last_execution_stats = Some(stats);
                pane_frame.skip_streak = 0;
                SectionOutcome::Executed { stats }
            };
            let pane_end = snapshot_layers(render_state);
            let skippable = prior_quad_range.is_some();
            if !skippable {
                pane_skip_chain_valid = false;
            }
            cofree.advance(outcome);

            plan.sections.push(RenderSection {
                scissor: Some(ScissorRect::from_pane_bounds(
                    &pane_frame.bounds,
                    viewport_width,
                    viewport_height,
                )),
                content_hash: pane_frame.command_hash,
                quad_range: QuadRange {
                    start: pane_start,
                    end: pane_end,
                },
                skippable,
                stats: pane_frame.last_execution_stats,
            });

            ui_items.extend(pane_frame.ui_items.iter().cloned());
            new_pane_frames.insert(pane_id, pane_frame);
        }

        let chrono_skip_streak_max = cofree
            .prior_outcomes
            .iter()
            .scan(0usize, |streak, outcome| {
                match outcome {
                    SectionOutcome::Skipped => *streak += 1,
                    SectionOutcome::Executed { .. } => *streak = 0,
                }
                Some(*streak)
            })
            .max()
            .unwrap_or(0);
        metrics::histogram!("gui.chrono.skip_streak_max").record(chrono_skip_streak_max as f64);
        metrics::histogram!("gui.chrono.total_quads").record(cofree.total_quads_emitted as f64);
        metrics::histogram!("gui.chrono.skip_rate").record(cofree.skip_rate());

        let chrome_start = snapshot_layers(render_state);

        if self.show_tab_bar {
            let (tab_bar, tab_bar_ui_items) = self.describe_tab_bar()?;
            execute_commands(
                &tab_bar,
                render_state,
                left_offset,
                top_offset,
                &filled_box,
            )?;
            ui_items.extend(tab_bar_ui_items);
        }

        if let Some(pane) = self.get_active_pane_or_overlay() {
            let splits = {
                let mux = mux::Mux::get();
                match mux.get_active_tab_for_window(self.mux_window_id) {
                    Some(tab) => {
                        let tab_id = tab.tab_id();
                        if self.tab_state(tab_id).overlay.is_some() {
                            vec![]
                        } else {
                            tab.iter_splits()
                        }
                    }
                    None => vec![],
                }
            };

            for split in &splits {
                let (commands, items) = self.describe_split(split, &pane);
                execute_commands(
                    &commands,
                    render_state,
                    left_offset,
                    top_offset,
                    &filled_box,
                )?;
                ui_items.extend(items);
            }
        }

        let borders = self.describe_window_borders();
        execute_commands(
            &borders,
            render_state,
            left_offset,
            top_offset,
            &filled_box,
        )?;

        let (modal, modal_ui_items) = self.describe_modal()?;
        execute_commands(
            &modal,
            render_state,
            left_offset,
            top_offset,
            &filled_box,
        )?;
        ui_items.extend(modal_ui_items);

        let chrome_end = snapshot_layers(render_state);
        plan.sections.push(RenderSection {
            scissor: None,
            content_hash: 0,
            quad_range: QuadRange {
                start: chrome_start,
                end: chrome_end,
            },
            skippable: false,
            stats: None,
        });

        let pane_section_count = plan.pane_section_count();
        let skippable_pane_section_count = plan.skippable_pane_section_count();
        log::trace!(
            "chrono render plan: {}/{} pane sections skippable",
            skippable_pane_section_count,
            pane_section_count
        );
        metrics::histogram!("gui.paint.pane_skip_rate").record(if pane_section_count > 0 {
            skippable_pane_section_count as f64 / pane_section_count as f64
        } else {
            0.0
        });

        self.render_plan = Some(plan);
        self.prev_pane_frames = new_pane_frames;
        self.ui_items = ui_items;

        Ok(())
    }
}
