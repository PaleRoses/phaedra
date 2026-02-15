use crate::termwindow::webgpu::{PostProcessUniform, ShaderUniform};
use crate::render_plan::quad_count_for_snapshot;
use config::observers::*;

const INDICES_PER_QUAD: usize = 6;

fn quad_range_for_section(
    range: &crate::render_plan::QuadRange,
    zindex: i8,
    sub_idx: usize,
) -> Option<(usize, usize)> {
    let start_quad = quad_count_for_snapshot(&range.start, zindex, sub_idx);
    let end_quad = quad_count_for_snapshot(&range.end, zindex, sub_idx);
    (end_quad > start_quad).then_some((start_quad, end_quad))
}

fn draw_layer_sections(
    render_pass: &mut wgpu::RenderPass<'_>,
    render_plan: &crate::render_plan::RenderPlan,
    zindex: i8,
    sub_idx: usize,
    fallback_index_count: usize,
    current_vertex_buffer: &wgpu::Buffer,
    previous_frame: Option<&crate::renderstate::FrameBuffers>,
) {
    let mut drew = false;
    let mut has_range = false;
    let mut sections_drawn = 0usize;
    let mut sections_skipped = 0usize;

    for (section_idx, section) in render_plan.sections.iter().enumerate() {
        let current_range = quad_range_for_section(&section.quad_range, zindex, sub_idx);
        if current_range.is_some() {
            has_range = true;
        }

        let mut use_previous_frame = false;
        let range = if section.skippable {
            if let Some(previous_frame) = previous_frame {
                if let Some(range) = previous_frame.section_quad_range(section_idx, zindex, sub_idx) {
                    if previous_frame.buffer(zindex, sub_idx).is_some() {
                        use_previous_frame = true;
                        sections_skipped += 1;
                        Some(range)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            current_range
        };

        let Some((start_quad, end_quad)) = range else {
            if section.skippable {
                sections_skipped += 1;
            }
            continue;
        };

        if let Some(scissor) = &section.scissor {
            if scissor.width == 0 || scissor.height == 0 {
                continue;
            }
            render_pass.set_scissor_rect(scissor.x, scissor.y, scissor.width, scissor.height);
        } else if render_plan.viewport_width > 0 && render_plan.viewport_height > 0 {
            render_pass.set_scissor_rect(0, 0, render_plan.viewport_width, render_plan.viewport_height);
        } else {
            continue;
        }

        if use_previous_frame {
            if let Some(previous_buffer) = previous_frame.and_then(|frame| frame.buffer(zindex, sub_idx))
            {
                render_pass.set_vertex_buffer(0, previous_buffer.slice(..));
            } else {
                continue;
            }
        }

        render_pass.draw_indexed(
            (start_quad * INDICES_PER_QUAD) as u32..(end_quad * INDICES_PER_QUAD) as u32,
            0,
            0..1,
        );
        sections_drawn += 1;
        drew = true;

        if use_previous_frame {
            render_pass.set_vertex_buffer(0, current_vertex_buffer.slice(..));
        }
    }

    if !drew && !has_range && fallback_index_count > 0 {
        render_pass.draw_indexed(0..fallback_index_count as u32, 0, 0..1);
        sections_drawn += 1;
    }

    metrics::histogram!("gui.draw.sections_drawn").record(sections_drawn as f64);
    metrics::histogram!("gui.draw.sections_skipped").record(sections_skipped as f64);
}

impl crate::TermWindow {
    pub fn call_draw(&mut self) -> anyhow::Result<()> {
        self.call_draw_webgpu()
    }

    fn call_draw_webgpu(&mut self) -> anyhow::Result<()> {
        use crate::termwindow::webgpu::WebGpuTexture;

        let webgpu = self.webgpu.as_mut().unwrap();
        let render_state = self.render_state.as_ref().unwrap();
        let render_plan = self.render_plan.as_ref();

        let has_postprocess = webgpu.has_postprocess();
        let width = self.dimensions.pixel_width as u32;
        let height = self.dimensions.pixel_height as u32;

        log::trace!("call_draw_webgpu: has_postprocess={}", has_postprocess);

        // Ensure intermediate texture exists if post-processing is enabled
        if has_postprocess {
            log::trace!("Creating intermediate texture {}x{}", width, height);
            webgpu.ensure_intermediate_texture(width, height);
        }

        let output = webgpu.surface.get_current_texture()?;
        let surface_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Choose render target: intermediate texture if post-processing, otherwise surface
        let intermediate_texture = webgpu.postprocess_intermediate_texture.borrow();
        let render_target_view = if has_postprocess {
            intermediate_texture
                .as_ref()
                .unwrap()
                .create_view(&wgpu::TextureViewDescriptor::default())
        } else {
            surface_view.clone()
        };
        drop(intermediate_texture);

        let mut encoder = webgpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        let tex = render_state.glyph_cache.borrow().atlas.texture();
        let tex = tex.downcast_ref::<WebGpuTexture>().unwrap();
        let texture_view = tex.create_view(&wgpu::TextureViewDescriptor::default());

        let texture_linear_bind_group =
            webgpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &webgpu.texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&webgpu.texture_linear_sampler),
                    },
                ],
                label: Some("linear bind group"),
            });

        let texture_nearest_bind_group =
            webgpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &webgpu.texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&webgpu.texture_nearest_sampler),
                    },
                ],
                label: Some("nearest bind group"),
            });

        let mut cleared = false;
        let mut next_frame_buffers = crate::renderstate::FrameBuffers::default();
        let foreground_text_hsb = self.config.color_config().foreground_text_hsb;
        let foreground_text_hsb = [
            foreground_text_hsb.hue,
            foreground_text_hsb.saturation,
            foreground_text_hsb.brightness,
        ];

        let milliseconds = self.created.elapsed().as_millis() as u32;
        let projection = euclid::Transform3D::<f32, f32, f32>::ortho(
            -(self.dimensions.pixel_width as f32) / 2.0,
            self.dimensions.pixel_width as f32 / 2.0,
            self.dimensions.pixel_height as f32 / 2.0,
            -(self.dimensions.pixel_height as f32) / 2.0,
            -1.0,
            1.0,
        )
        .to_arrays_transposed();

        // First pass: render terminal content to render target
        for layer in render_state.layers.borrow().iter() {
            for idx in 0..3 {
                let vb = &layer.vb.borrow()[idx];
                let (vertex_count, index_count) = vb.vertex_index_count();
                let uniforms;
                if vertex_count > 0 {
                    let vertex_buffer = {
                        let mut vertices = vb.current_vb_mut();
                        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("Render Pass"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &render_target_view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: if cleared {
                                        wgpu::LoadOp::Load
                                    } else {
                                        wgpu::LoadOp::Clear(wgpu::Color {
                                            r: 0.,
                                            g: 0.,
                                            b: 0.,
                                            a: 0.,
                                        })
                                    },
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            depth_stencil_attachment: None,
                            occlusion_query_set: None,
                            timestamp_writes: None,
                        });
                        cleared = true;

                        uniforms = webgpu.create_uniform(ShaderUniform {
                            foreground_text_hsb,
                            milliseconds,
                            projection,
                        });

                        render_pass.set_pipeline(&webgpu.render_pipeline);
                        render_pass.set_bind_group(0, &uniforms, &[]);
                        render_pass.set_bind_group(1, &texture_linear_bind_group, &[]);
                        render_pass.set_bind_group(2, &texture_nearest_bind_group, &[]);
                        let vertex_buffer = vertices.webgpu_mut().recreate();
                        vertex_buffer.unmap();
                        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                        render_pass
                            .set_index_buffer(vb.indices.webgpu().slice(..), wgpu::IndexFormat::Uint32);
                        if let Some(render_plan) = render_plan {
                            let previous_frame = render_state.prev_frame_buffers.borrow();
                            draw_layer_sections(
                                &mut render_pass,
                                render_plan,
                                layer.zindex(),
                                idx,
                                index_count,
                                &vertex_buffer,
                                previous_frame.as_ref(),
                            );
                        } else {
                            render_pass.draw_indexed(0..index_count as u32, 0, 0..1);
                        }
                        vertex_buffer
                    };

                    next_frame_buffers
                        .buffers
                        .push((layer.zindex(), idx, vertex_buffer));
                }

                vb.next_index();
            }
        }

        if let Some(render_plan) = render_plan {
            next_frame_buffers.section_ranges = render_plan
                .sections
                .iter()
                .map(|section| section.quad_range.clone())
                .collect();
        }
        *render_state.prev_frame_buffers.borrow_mut() = Some(next_frame_buffers);

        // Second pass: apply post-processing shader if enabled
        if has_postprocess {
            let postprocess_uniform = webgpu.create_postprocess_uniform(PostProcessUniform {
                resolution: [width as f32, height as f32],
                time: self.created.elapsed().as_secs_f32(),
                _padding: 0.0,
            });

            let pipeline = webgpu.postprocess_pipeline.borrow();
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("PostProcess Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(pipeline.as_ref().unwrap());
            render_pass.set_bind_group(0, &postprocess_uniform, &[]);
            // Draw a full-screen triangle (3 vertices, no vertex buffer needed)
            render_pass.draw(0..3, 0..1);
        }

        // submit will accept anything that implements IntoIter
        webgpu.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

}
