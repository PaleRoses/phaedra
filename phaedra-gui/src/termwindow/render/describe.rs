use crate::frame::{ChromeFrame, Frame, PaneFrame, PostProcessParams};
use crate::render_command::{HsbTransform as CmdHsbTransform, RectF, RenderCommand};
use crate::selection::SelectionRange;
use crate::termwindow::render::paint::AllowImage;
use crate::termwindow::render::{
    same_hyperlink, CursorProperties, LineCommandCacheValue, LineQuadCacheKey,
    LineToEleShapeCacheKey, RenderScreenLineParams, RenderScreenLineResult,
};
use crate::termwindow::{ScrollHit, UIItem, UIItemType};
use anyhow::Context;
use ::window::DeadKeyStatus;
use config::observers::*;
use config::{TermConfig, VisualBellTarget};
use mux::pane::{Pane, WithPaneLines};
use mux::renderable::{RenderableDimensions, StableCursorPosition};
use mux::tab::{PositionedPane, PositionedSplit, SplitDirection};
use ordered_float::NotNan;
use phaedra_dynamic::Value;
use phaedra_term::color::{ColorAttribute, ColorPalette};
use phaedra_term::{Line, StableRowIndex, TerminalConfiguration};
use std::sync::Arc;
use std::time::Instant;
use window::bitmaps::TextureRect;
use window::color::LinearRgba;

impl crate::TermWindow {
    pub fn describe_window_borders(&self) -> Vec<RenderCommand> {
        let border_dimensions = self.get_os_border();
        let mut commands = Vec::new();

        if border_dimensions.top.get() > 0
            || border_dimensions.bottom.get() > 0
            || border_dimensions.left.get() > 0
            || border_dimensions.right.get() > 0
        {
            let height = self.dimensions.pixel_height as f32;
            let width = self.dimensions.pixel_width as f32;

            let border_top = border_dimensions.top.get() as f32;
            if border_top > 0.0 {
                let rect: RectF = euclid::rect(0.0, 0.0, width, border_top);
                commands.push(RenderCommand::FillRect {
                    layer: 1,
                    zindex: 0,
                    rect,
                    color: self
                        .config
                        .window_config()
                        .window_frame
                        .border_top_color
                        .map(|c| c.to_linear())
                        .unwrap_or(border_dimensions.color),
                    hsv: None,
                });
            }

            let border_left = border_dimensions.left.get() as f32;
            if border_left > 0.0 {
                let rect: RectF = euclid::rect(0.0, 0.0, border_left, height);
                commands.push(RenderCommand::FillRect {
                    layer: 1,
                    zindex: 0,
                    rect,
                    color: self
                        .config
                        .window_config()
                        .window_frame
                        .border_left_color
                        .map(|c| c.to_linear())
                        .unwrap_or(border_dimensions.color),
                    hsv: None,
                });
            }

            let border_bottom = border_dimensions.bottom.get() as f32;
            if border_bottom > 0.0 {
                let rect: RectF = euclid::rect(0.0, height - border_bottom, width, height);
                commands.push(RenderCommand::FillRect {
                    layer: 1,
                    zindex: 0,
                    rect,
                    color: self
                        .config
                        .window_config()
                        .window_frame
                        .border_bottom_color
                        .map(|c| c.to_linear())
                        .unwrap_or(border_dimensions.color),
                    hsv: None,
                });
            }

            let border_right = border_dimensions.right.get() as f32;
            if border_right > 0.0 {
                let rect: RectF = euclid::rect(width - border_right, 0.0, border_right, height);
                commands.push(RenderCommand::FillRect {
                    layer: 1,
                    zindex: 0,
                    rect,
                    color: self
                        .config
                        .window_config()
                        .window_frame
                        .border_right_color
                        .map(|c| c.to_linear())
                        .unwrap_or(border_dimensions.color),
                    hsv: None,
                });
            }
        }

        commands
    }

    pub fn describe_split(
        &self,
        split: &PositionedSplit,
        pane: &Arc<dyn Pane>,
    ) -> (Vec<RenderCommand>, Vec<UIItem>) {
        let palette = pane.palette();
        let foreground = palette.split.to_linear();
        let cell_width = self.render_metrics.cell_size.width as f32;
        let cell_height = self.render_metrics.cell_size.height as f32;

        let border = self.get_os_border();
        let first_row_offset = if self.show_tab_bar && !self.config.tab_bar().tab_bar_at_bottom {
            self.tab_bar_pixel_height().unwrap_or(0.0)
        } else {
            0.0
        } + border.top.get() as f32;

        let (padding_left, padding_top) = self.padding_left_top();

        let pos_y = split.top as f32 * cell_height + first_row_offset + padding_top;
        let pos_x = split.left as f32 * cell_width + padding_left + border.left.get() as f32;

        let mut commands = Vec::with_capacity(1);
        let mut ui_items = Vec::with_capacity(1);

        if split.direction == SplitDirection::Horizontal {
            let rect: RectF = euclid::rect(
                pos_x + (cell_width / 2.0),
                pos_y - (cell_height / 2.0),
                self.render_metrics.underline_height as f32,
                (1.0 + split.size as f32) * cell_height,
            );
            commands.push(RenderCommand::FillRect {
                layer: 2,
                zindex: 0,
                rect,
                color: foreground,
                hsv: None,
            });
            ui_items.push(UIItem {
                x: border.left.get() as usize
                    + padding_left as usize
                    + (split.left * cell_width as usize),
                width: cell_width as usize,
                y: padding_top as usize
                    + first_row_offset as usize
                    + split.top * cell_height as usize,
                height: split.size * cell_height as usize,
                item_type: UIItemType::Split(split.clone()),
            });
        } else {
            let rect: RectF = euclid::rect(
                pos_x - (cell_width / 2.0),
                pos_y + (cell_height / 2.0),
                (1.0 + split.size as f32) * cell_width,
                self.render_metrics.underline_height as f32,
            );
            commands.push(RenderCommand::FillRect {
                layer: 2,
                zindex: 0,
                rect,
                color: foreground,
                hsv: None,
            });
            ui_items.push(UIItem {
                x: border.left.get() as usize
                    + padding_left as usize
                    + (split.left * cell_width as usize),
                width: split.size * cell_width as usize,
                y: padding_top as usize
                    + first_row_offset as usize
                    + split.top * cell_height as usize,
                height: cell_height as usize,
                item_type: UIItemType::Split(split.clone()),
            });
        }

        (commands, ui_items)
    }

    pub fn describe_window_background(
        &self,
        panes: &[PositionedPane],
    ) -> anyhow::Result<Vec<RenderCommand>> {
        let window_is_transparent = !self.window_background.is_empty();
        let mut paint_terminal_background = false;

        match (self.window_background.is_empty(), self.allow_images) {
            (false, AllowImage::Yes | AllowImage::Scale(_)) => {
                let bg_color = self
                    .palette
                    .as_ref()
                    .map(|palette| palette.background)
                    .unwrap_or_else(|| config::TermConfig::new().color_palette().background)
                    .to_linear();
                let top = panes
                    .iter()
                    .find(|p| p.is_active)
                    .map(|p| match self.get_viewport(p.pane.pane_id()) {
                        Some(top) => top,
                        None => p.pane.get_dimensions().physical_top,
                    })
                    .unwrap_or(0);

                let (commands, loaded_any) = self.describe_backgrounds(bg_color, top)?;
                if loaded_any {
                    return Ok(commands);
                }
                paint_terminal_background = true;
            }
            _ if window_is_transparent => {}
            _ => {
                paint_terminal_background = true;
            }
        }

        if !paint_terminal_background {
            return Ok(Vec::new());
        }

        let background = if panes.len() == 1 {
            panes[0].pane.palette().background
        } else {
            self.palette
                .as_ref()
                .map(|palette| palette.background)
                .unwrap_or_else(|| config::TermConfig::new().color_palette().background)
        }
        .to_linear()
        .mul_alpha(1.0);

        let rect: RectF = euclid::rect(
            0.0,
            0.0,
            self.dimensions.pixel_width as f32,
            self.dimensions.pixel_height as f32,
        );

        Ok(vec![RenderCommand::FillRect {
            layer: 0,
            zindex: 0,
            rect,
            color: background,
            hsv: None,
        }])
    }

    pub fn describe_pane(&self, pos: &PositionedPane) -> anyhow::Result<PaneFrame> {
        let config = &self.config;
        let palette = pos.pane.palette();
        let pane_id = pos.pane.pane_id();

        let (padding_left, padding_top) = self.padding_left_top();
        let tab_bar_height = if self.show_tab_bar {
            self.tab_bar_pixel_height().context("tab_bar_pixel_height")?
        } else {
            0.0
        };
        let (top_bar_height, bottom_bar_height) = if self.config.tab_bar().tab_bar_at_bottom {
            (0.0, tab_bar_height)
        } else {
            (tab_bar_height, 0.0)
        };

        let border = self.get_os_border();
        let top_pixel_y = top_bar_height + padding_top + border.top.get() as f32;

        let cursor = pos.pane.get_cursor_position();
        let current_viewport = self.get_viewport(pane_id);
        let dims = pos.pane.get_dimensions();

        let gl_state = self.render_state.as_ref().unwrap();
        let white_space = gl_state.util_sprites.white_space.texture_coords();
        let filled_box = gl_state.util_sprites.filled_box.texture_coords();

        let window_is_transparent = !self.window_background.is_empty();

        let default_bg = palette
            .resolve_bg(ColorAttribute::Default)
            .to_linear()
            .mul_alpha(if window_is_transparent {
                0.0
            } else {
                config.text().text_background_opacity
            });

        let cell_width = self.render_metrics.cell_size.width as f32;
        let cell_height = self.render_metrics.cell_size.height as f32;

        let background_rect = {
            let (x, width_delta) = if pos.left == 0 {
                (
                    0.0,
                    padding_left + border.left.get() as f32 + (cell_width / 2.0),
                )
            } else {
                (
                    padding_left + border.left.get() as f32 - (cell_width / 2.0)
                        + (pos.left as f32 * cell_width),
                    cell_width,
                )
            };

            let (y, height_delta) = if pos.top == 0 {
                ((top_pixel_y - padding_top), padding_top + (cell_height / 2.0))
            } else {
                (
                    top_pixel_y + (pos.top as f32 * cell_height) - (cell_height / 2.0),
                    cell_height,
                )
            };

            euclid::rect(
                x,
                y,
                if pos.left + pos.width >= self.terminal_size.cols as usize {
                    self.dimensions.pixel_width as f32 - x
                } else {
                    (pos.width as f32 * cell_width) + width_delta
                },
                if pos.top + pos.height >= self.terminal_size.rows as usize {
                    self.dimensions.pixel_height as f32 - y
                } else {
                    (pos.height as f32 * cell_height) + height_delta
                },
            )
        };

        let inactive_hsv = if pos.is_active {
            None
        } else {
            Some(CmdHsbTransform {
                hue: config.color_config().inactive_pane_hsb.hue,
                saturation: config.color_config().inactive_pane_hsb.saturation,
                brightness: config.color_config().inactive_pane_hsb.brightness,
            })
        };

        let mut commands = Vec::new();
        let mut ui_items = Vec::new();

        if self.window_background.is_empty() {
            commands.push(RenderCommand::FillRect {
                layer: 0,
                zindex: 0,
                rect: background_rect,
                color: palette.background.to_linear().mul_alpha(1.0),
                hsv: inactive_hsv.clone(),
            });
        }

        if let Some(intensity) = self.get_intensity_if_bell_target_ringing(
            &pos.pane,
            config,
            VisualBellTarget::BackgroundColor,
        ) {
            let LinearRgba(r, g, b, _) = config
                .color_config()
                .resolved_palette
                .visual_bell
                .as_deref()
                .unwrap_or(&palette.foreground)
                .to_linear();

            let background = if window_is_transparent {
                LinearRgba::with_components(r, g, b, intensity)
            } else {
                let (r1, g1, b1, a) = palette.background.to_linear().mul_alpha(1.0).tuple();
                LinearRgba::with_components(
                    r1 + (r - r1) * intensity,
                    g1 + (g - g1) * intensity,
                    b1 + (b - b1) * intensity,
                    a,
                )
            };

            commands.push(RenderCommand::FillRect {
                layer: 0,
                zindex: 0,
                rect: background_rect,
                color: background,
                hsv: inactive_hsv.clone(),
            });
        }

        if pos.is_active && self.show_scroll_bar {
            let thumb_y_offset = top_bar_height as usize + border.top.get();
            let min_height = self.min_scroll_bar_height();
            let info = ScrollHit::thumb(
                &*pos.pane,
                current_viewport,
                self.dimensions.pixel_height.saturating_sub(
                    thumb_y_offset + border.bottom.get() + bottom_bar_height as usize,
                ),
                min_height as usize,
            );

            let abs_thumb_top = thumb_y_offset + info.top;
            let thumb_size = info.height;
            let color = palette.scrollbar_thumb.to_linear();
            let padding = self.effective_right_padding(config) as f32;
            let thumb_x = self.dimensions.pixel_width - padding as usize - border.right.get();

            ui_items.push(UIItem {
                x: thumb_x,
                width: padding as usize,
                y: thumb_y_offset,
                height: info.top,
                item_type: UIItemType::AboveScrollThumb,
            });
            ui_items.push(UIItem {
                x: thumb_x,
                width: padding as usize,
                y: abs_thumb_top,
                height: thumb_size,
                item_type: UIItemType::ScrollThumb,
            });
            ui_items.push(UIItem {
                x: thumb_x,
                width: padding as usize,
                y: abs_thumb_top + thumb_size,
                height: self
                    .dimensions
                    .pixel_height
                    .saturating_sub(abs_thumb_top + thumb_size),
                item_type: UIItemType::BelowScrollThumb,
            });

            commands.push(RenderCommand::FillRect {
                layer: 2,
                zindex: 0,
                rect: euclid::rect(
                    thumb_x as f32,
                    abs_thumb_top as f32,
                    padding,
                    thumb_size as f32,
                ),
                color,
                hsv: None,
            });
        }

        let (selrange, rectangular) = {
            let sel = self.selection(pane_id);
            (sel.range.clone(), sel.rectangular)
        };

        let selection_fg = palette.selection_fg.to_linear();
        let selection_bg = palette.selection_bg.to_linear();
        let cursor_fg = palette.cursor_fg.to_linear();
        let cursor_bg = palette.cursor_bg.to_linear();
        let global_palette = self
            .palette
            .as_ref()
            .cloned()
            .unwrap_or_else(|| TermConfig::new().color_palette());
        let cursor_is_default_color =
            palette.cursor_fg == global_palette.cursor_fg && palette.cursor_bg == global_palette.cursor_bg;
        let cursor_border_color = palette.cursor_border.to_linear();
        let foreground = palette.foreground.to_linear();

        let stable_range = match current_viewport {
            Some(top) => top..top + dims.viewport_rows as StableRowIndex,
            None => dims.physical_top..dims.physical_top + dims.viewport_rows as StableRowIndex,
        };
        pos.pane.apply_hyperlinks(
            stable_range.clone(),
            &self.config.terminal_features().hyperlink_rules,
        );

        struct LineDescriber<'a> {
            term_window: &'a crate::TermWindow,
            selrange: Option<SelectionRange>,
            rectangular: bool,
            dims: RenderableDimensions,
            top_pixel_y: f32,
            left_pixel_x: f32,
            pos: &'a PositionedPane,
            cursor: &'a StableCursorPosition,
            palette: &'a ColorPalette,
            default_bg: LinearRgba,
            cursor_border_color: LinearRgba,
            selection_fg: LinearRgba,
            selection_bg: LinearRgba,
            cursor_fg: LinearRgba,
            cursor_bg: LinearRgba,
            foreground: LinearRgba,
            cursor_is_default_color: bool,
            white_space: TextureRect,
            filled_box: TextureRect,
            window_is_transparent: bool,
            commands: Vec<RenderCommand>,
            error: Option<anyhow::Error>,
        }

        impl<'a> LineDescriber<'a> {
            fn describe_line(
                &mut self,
                stable_top: StableRowIndex,
                line_idx: usize,
                line: &Line,
            ) -> anyhow::Result<()> {
                let stable_row = stable_top + line_idx as StableRowIndex;
                let selrange = self
                    .selrange
                    .map_or(0..0, |sel| sel.cols_for_row(stable_row, self.rectangular));
                let selection = selrange.start..selrange.end.min(self.dims.cols);

                let (cursor, composing, password_input) = if self.cursor.y == stable_row {
                    (
                        Some(CursorProperties {
                            position: StableCursorPosition {
                                y: 0,
                                ..*self.cursor
                            },
                            dead_key_or_leader: self.term_window.dead_key_status
                                != DeadKeyStatus::None
                                || self.term_window.leader_is_active(),
                            cursor_fg: self.cursor_fg,
                            cursor_bg: self.cursor_bg,
                            cursor_border_color: self.cursor_border_color,
                            cursor_is_default_color: self.cursor_is_default_color,
                        }),
                        match (self.pos.is_active, &self.term_window.dead_key_status) {
                            (true, DeadKeyStatus::Composing(composing)) => Some(composing.to_string()),
                            _ => None,
                        },
                        if self.term_window.config.terminal_features().detect_password_input {
                            match self.pos.pane.get_metadata() {
                                Value::Object(obj) => {
                                    match obj.get(&Value::String("password_input".to_string())) {
                                        Some(Value::Bool(b)) => *b,
                                        _ => false,
                                    }
                                }
                                _ => false,
                            }
                        } else {
                            false
                        },
                    )
                } else {
                    (None, None, false)
                };

                let shape_hash = self.term_window.shape_hash_for_line(line);
                let quad_key = LineQuadCacheKey {
                    pane_id: self.pos.pane.pane_id(),
                    pane_width: self.pos.width,
                    pane_left: self.pos.left,
                    password_input,
                    pane_is_active: self.pos.is_active,
                    config_generation: self.term_window.config.generation(),
                    shape_generation: self.term_window.shape_generation,
                    quad_generation: self.term_window.quad_generation,
                    composing: composing.clone(),
                    selection: selection.clone(),
                    cursor,
                    shape_hash,
                    top_pixel_y: NotNan::new(self.top_pixel_y).unwrap()
                        + (line_idx + self.pos.top) as f32
                            * self.term_window.render_metrics.cell_size.height as f32,
                    left_pixel_x: NotNan::new(self.left_pixel_x).unwrap(),
                    phys_line_idx: line_idx,
                    reverse_video: self.dims.reverse_video,
                };

                let cached_commands = {
                    let mut cache = self.term_window.line_command_cache.borrow_mut();
                    cache.get(&quad_key).and_then(|cached| {
                        let expired = cached.expires.map(|i| Instant::now() >= i).unwrap_or(false);
                        let hover_changed = if cached.invalidate_on_hover_change {
                            !same_hyperlink(
                                cached.current_highlight.as_ref(),
                                self.term_window.current_highlight.as_ref(),
                            )
                        } else {
                            false
                        };
                        if expired || hover_changed {
                            None
                        } else {
                            self.term_window.update_next_frame_time(cached.expires);
                            Some(cached.commands.clone())
                        }
                    })
                };

                if let Some(mut commands) = cached_commands {
                    self.commands.append(&mut commands);
                    return Ok(());
                }

                let next_due = self.term_window.has_animation.borrow_mut().take();
                let shape_key = LineToEleShapeCacheKey {
                    shape_hash,
                    shape_generation: quad_key.shape_generation,
                    composing: if self.cursor.y == stable_row && self.pos.is_active {
                        if let DeadKeyStatus::Composing(composing) = &self.term_window.dead_key_status {
                            Some((self.cursor.x, composing.to_string()))
                        } else {
                            None
                        }
                    } else {
                        None
                    },
                };

                let (mut line_commands, line_result): (
                    Vec<RenderCommand>,
                    RenderScreenLineResult,
                ) = self
                    .term_window
                    .describe_screen_line(RenderScreenLineParams {
                        top_pixel_y: *quad_key.top_pixel_y,
                        left_pixel_x: self.left_pixel_x,
                        pixel_width: self.dims.cols as f32
                            * self.term_window.render_metrics.cell_size.width as f32,
                        stable_line_idx: Some(stable_row),
                        line,
                        selection,
                        cursor: self.cursor,
                        palette: self.palette,
                        dims: &self.dims,
                        config: &self.term_window.config,
                        pane: Some(&self.pos.pane),
                        white_space: self.white_space,
                        filled_box: self.filled_box,
                        cursor_border_color: self.cursor_border_color,
                        foreground: self.foreground,
                        is_active: self.pos.is_active,
                        selection_fg: self.selection_fg,
                        selection_bg: self.selection_bg,
                        cursor_fg: self.cursor_fg,
                        cursor_bg: self.cursor_bg,
                        cursor_is_default_color: self.cursor_is_default_color,
                        window_is_transparent: self.window_is_transparent,
                        default_bg: self.default_bg,
                        font: None,
                        style: None,
                        use_pixel_positioning: self
                            .term_window
                            .config
                            .text()
                            .experimental_pixel_positioning,
                        render_metrics: self.term_window.render_metrics,
                        shape_key: Some(shape_key),
                        password_input,
                    })
                    .context("describe_screen_line")?;

                let expires = self.term_window.has_animation.borrow().as_ref().cloned();
                self.term_window.update_next_frame_time(next_due);

                self.term_window
                    .line_command_cache
                    .borrow_mut()
                    .put(
                        quad_key,
                        LineCommandCacheValue {
                            expires,
                            commands: line_commands.clone(),
                            invalidate_on_hover_change: line_result.invalidate_on_hover_change,
                            current_highlight: if line_result.invalidate_on_hover_change {
                                self.term_window.current_highlight.clone()
                            } else {
                                None
                            },
                        },
                    );

                self.commands.append(&mut line_commands);
                Ok(())
            }
        }

        impl<'a> WithPaneLines for LineDescriber<'a> {
            fn with_lines_mut(&mut self, stable_top: StableRowIndex, lines: &mut [&mut Line]) {
                for (line_idx, line) in lines.iter().enumerate() {
                    if let Err(err) = self.describe_line(stable_top, line_idx, &**line) {
                        self.error.replace(err);
                        return;
                    }
                }
            }
        }

        let left_pixel_x = padding_left
            + border.left.get() as f32
            + (pos.left as f32 * self.render_metrics.cell_size.width as f32);
        let mut line_describer = LineDescriber {
            term_window: self,
            selrange,
            rectangular,
            dims,
            top_pixel_y,
            left_pixel_x,
            pos,
            cursor: &cursor,
            palette: &palette,
            default_bg,
            cursor_border_color,
            selection_fg,
            selection_bg,
            cursor_fg,
            cursor_bg,
            foreground,
            cursor_is_default_color,
            white_space,
            filled_box,
            window_is_transparent,
            commands: Vec::new(),
            error: None,
        };

        pos.pane.with_lines_mut(stable_range, &mut line_describer);
        if let Some(error) = line_describer.error.take() {
            return Err(error).context("error while calling with_lines_mut");
        }

        commands.append(&mut line_describer.commands);
        // DIAGNOSTIC: clip_to_rect disabled to isolate rendering bug
        // let commands: Vec<RenderCommand> = commands
        //     .into_iter()
        //     .map(|cmd| cmd.clip_to_rect(&background_rect))
        //     .filter(|cmd| !matches!(cmd, RenderCommand::Nop))
        //     .collect();
        let content_hash = RenderCommand::content_hash(&commands);

        Ok(PaneFrame {
            pane_id,
            is_active: pos.is_active,
            bounds: background_rect,
            content_hash,
            commands,
            ui_items,
        })
    }

    pub fn describe_tab_bar(&self) -> anyhow::Result<(Vec<RenderCommand>, Vec<UIItem>)> {
        if self.config.tab_bar().use_fancy_tab_bar {
            if let Some(computed) = self.fancy_tab_bar.as_ref() {
                let ui_items = computed.ui_items();
                let commands = self.describe_element(computed, None)?;
                return Ok((commands, ui_items));
            }

            let palette = self
                .palette
                .as_ref()
                .cloned()
                .unwrap_or_else(|| TermConfig::new().color_palette());
            let computed = self.build_fancy_tab_bar(&palette)?;
            let ui_items = computed.ui_items();
            let commands = self.describe_element(&computed, None)?;
            return Ok((commands, ui_items));
        }

        let border = self.get_os_border();
        let palette = self
            .palette
            .as_ref()
            .cloned()
            .unwrap_or_else(|| TermConfig::new().color_palette());
        let tab_bar_height = self.tab_bar_pixel_height()?;
        let tab_bar_y = if self.config.tab_bar().tab_bar_at_bottom {
            ((self.dimensions.pixel_height as f32) - (tab_bar_height + border.bottom.get() as f32))
                .max(0.0)
        } else {
            border.top.get() as f32
        };

        let ui_items = self.tab_bar.compute_ui_items(
            tab_bar_y as usize,
            self.render_metrics.cell_size.height as usize,
            self.render_metrics.cell_size.width as usize,
        );

        let window_is_transparent = !self.window_background.is_empty();
        let gl_state = self.render_state.as_ref().unwrap();
        let white_space = gl_state.util_sprites.white_space.texture_coords();
        let filled_box = gl_state.util_sprites.filled_box.texture_coords();
        let default_bg = palette
            .resolve_bg(ColorAttribute::Default)
            .to_linear()
            .mul_alpha(if window_is_transparent {
                0.0
            } else {
                self.config.text().text_background_opacity
            });
        let cursor = StableCursorPosition::default();

        let (commands, _result): (Vec<RenderCommand>, RenderScreenLineResult) =
            self.describe_screen_line(RenderScreenLineParams {
                top_pixel_y: tab_bar_y,
                left_pixel_x: 0.0,
                pixel_width: self.dimensions.pixel_width as f32,
                stable_line_idx: None,
                line: self.tab_bar.line(),
                selection: 0..0,
                cursor: &cursor,
                palette: &palette,
                dims: &RenderableDimensions {
                    cols: self.dimensions.pixel_width / self.render_metrics.cell_size.width as usize,
                    physical_top: 0,
                    scrollback_rows: 0,
                    scrollback_top: 0,
                    viewport_rows: 1,
                    dpi: self.terminal_size.dpi,
                    pixel_height: self.render_metrics.cell_size.height as usize,
                    pixel_width: self.terminal_size.pixel_width,
                    reverse_video: false,
                },
                config: &self.config,
                cursor_border_color: LinearRgba::default(),
                foreground: palette.foreground.to_linear(),
                pane: None,
                is_active: true,
                selection_fg: LinearRgba::default(),
                selection_bg: LinearRgba::default(),
                cursor_fg: LinearRgba::default(),
                cursor_bg: LinearRgba::default(),
                cursor_is_default_color: true,
                white_space,
                filled_box,
                window_is_transparent,
                default_bg,
                style: None,
                font: None,
                use_pixel_positioning: self.config.text().experimental_pixel_positioning,
                render_metrics: self.render_metrics,
                shape_key: None,
                password_input: false,
            })?;

        Ok((commands, ui_items))
    }

    pub fn describe_modal(&self) -> anyhow::Result<(Vec<RenderCommand>, Vec<UIItem>)> {
        let mut commands = Vec::new();
        let mut ui_items = Vec::new();

        if let Some(modal) = self.get_modal() {
            for computed in modal.computed_element(self)?.iter() {
                let mut element_ui_items = computed.ui_items();
                let mut element_commands = self.describe_element(computed, None)?;
                commands.append(&mut element_commands);
                ui_items.append(&mut element_ui_items);
            }
        }

        Ok((commands, ui_items))
    }

    pub fn describe_frame(&self) -> anyhow::Result<Frame> {
        let panes = self.get_panes_to_render();
        let background = self.describe_window_background(&panes)?;

        let mut pane_frames = Vec::with_capacity(panes.len());
        let mut ui_items = Vec::new();

        for pos in &panes {
            let pane_frame = self.describe_pane(pos)?;
            ui_items.extend(pane_frame.ui_items.iter().cloned());
            pane_frames.push(pane_frame);
        }

        let mut split_commands = Vec::new();
        let mut split_ui_items = Vec::new();

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
                let (mut commands, mut items) = self.describe_split(split, &pane);
                split_commands.append(&mut commands);
                split_ui_items.append(&mut items);
            }
        }

        let (tab_bar, tab_bar_ui_items) = if self.show_tab_bar {
            self.describe_tab_bar()?
        } else {
            (Vec::new(), Vec::new())
        };

        let borders = self.describe_window_borders();
        let (modal, modal_ui_items) = self.describe_modal()?;

        ui_items.extend(split_ui_items.iter().cloned());
        ui_items.extend(tab_bar_ui_items.iter().cloned());
        ui_items.extend(modal_ui_items.iter().cloned());

        let chrome = ChromeFrame {
            tab_bar,
            tab_bar_ui_items,
            splits: split_commands,
            split_ui_items,
            borders,
            modal,
            modal_ui_items,
        };
        let postprocess: Option<PostProcessParams> = None;

        Ok(Frame {
            background,
            panes: pane_frames,
            chrome,
            postprocess,
            ui_items,
        })
    }
}
