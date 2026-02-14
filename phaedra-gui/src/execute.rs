use crate::frontend::front_end;
use crate::input_effect::InputEffect;
use crate::overlay::{
    confirm_quit_program, start_overlay, CopyModeParams, CopyOverlay, QuickSelectOverlay,
};
use crate::spawn::SpawnWhere;
use crate::termwindow::keyevent::KeyTableArgs;
use crate::termwindow::TermWindow;
use anyhow::anyhow;
use config::observers::{KeyInputObserver, WindowConfigObserver};
use config::keyassignment::{
    KeyAssignment, LauncherActionArgs, PaneDirection, RotationDirection, SpawnCommand, SplitSize,
};
use config::WindowCloseConfirmation;
use config::window::WindowLevel;
use mux::pane::{Pane, Pattern as MuxPattern};
use mux::tab::{SplitDirection, SplitRequest, SplitSize as MuxSplitSize};
use mux::Mux;
use std::rc::Rc;
use std::sync::Arc;
use termwiz::escape::{Action, Esc, EscCode};
use window::Connection;
use window::{ConnectionOps, WindowOps};

pub fn describe_effect(effect: &InputEffect) -> &'static str {
    match effect {
        InputEffect::ActivateKeyTable { .. } => "activate_key_table",
        InputEffect::PopKeyTable => "pop_key_table",
        InputEffect::ClearKeyTableStack => "clear_key_table_stack",
        InputEffect::ActivateLeader { .. } => "activate_leader",
        InputEffect::SpawnTab { .. } => "spawn_tab",
        InputEffect::SpawnWindow => "spawn_window",
        InputEffect::SpawnCommandInNewTab { .. } => "spawn_command_in_new_tab",
        InputEffect::SpawnCommandInNewWindow { .. } => "spawn_command_in_new_window",
        InputEffect::SplitPane { .. } => "split_pane",
        InputEffect::ToggleFullScreen => "toggle_full_screen",
        InputEffect::ToggleAlwaysOnTop => "toggle_always_on_top",
        InputEffect::ToggleAlwaysOnBottom => "toggle_always_on_bottom",
        InputEffect::SetWindowLevel(_) => "set_window_level",
        InputEffect::HideWindow => "hide_window",
        InputEffect::ShowWindow => "show_window",
        InputEffect::StartWindowDrag => "start_window_drag",
        InputEffect::AdjustFontSize { .. } => "adjust_font_size",
        InputEffect::ResetFontSize => "reset_font_size",
        InputEffect::ResetFontAndWindowSize => "reset_font_and_window_size",
        InputEffect::ActivateTab { .. } => "activate_tab",
        InputEffect::ActivateTabRelative { .. } => "activate_tab_relative",
        InputEffect::ActivateLastTab => "activate_last_tab",
        InputEffect::MoveTab { .. } => "move_tab",
        InputEffect::MoveTabRelative { .. } => "move_tab_relative",
        InputEffect::CloseTab { .. } => "close_tab",
        InputEffect::ActivatePaneByIndex { .. } => "activate_pane_by_index",
        InputEffect::ActivatePaneDirection { .. } => "activate_pane_direction",
        InputEffect::AdjustPaneSize { .. } => "adjust_pane_size",
        InputEffect::TogglePaneZoom => "toggle_pane_zoom",
        InputEffect::SetPaneZoom { .. } => "set_pane_zoom",
        InputEffect::ClosePane { .. } => "close_pane",
        InputEffect::RotatePanes { .. } => "rotate_panes",
        InputEffect::ActivateWindow { .. } => "activate_window",
        InputEffect::ActivateWindowRelative { .. } => "activate_window_relative",
        InputEffect::CopySelection { .. } => "copy_selection",
        InputEffect::CopyText { .. } => "copy_text",
        InputEffect::Paste { .. } => "paste",
        InputEffect::CompleteSelection { .. } => "complete_selection",
        InputEffect::CompleteSelectionOrOpenLink { .. } => "complete_selection_or_open_link",
        InputEffect::ScrollByPage { .. } => "scroll_by_page",
        InputEffect::ScrollByLine { .. } => "scroll_by_line",
        InputEffect::ScrollByWheelDelta => "scroll_by_wheel_delta",
        InputEffect::ScrollToPrompt { .. } => "scroll_to_prompt",
        InputEffect::ScrollToTop => "scroll_to_top",
        InputEffect::ScrollToBottom => "scroll_to_bottom",
        InputEffect::SelectAtMouseCursor { .. } => "select_at_mouse_cursor",
        InputEffect::ExtendSelectionToMouse { .. } => "extend_selection_to_mouse",
        InputEffect::OpenLinkAtMouseCursor => "open_link_at_mouse_cursor",
        InputEffect::ClearSelection => "clear_selection",
        InputEffect::SendString { .. } => "send_string",
        InputEffect::SendKey { .. } => "send_key",
        InputEffect::SendToPane { .. } => "send_to_pane",
        InputEffect::ResetTerminal => "reset_terminal",
        InputEffect::ClearScrollback { .. } => "clear_scrollback",
        InputEffect::CopyMode { .. } => "copy_mode",
        InputEffect::ShowCopyMode => "show_copy_mode",
        InputEffect::ShowSearch { .. } => "show_search",
        InputEffect::ShowQuickSelect { .. } => "show_quick_select",
        InputEffect::ShowTabNavigator => "show_tab_navigator",
        InputEffect::ShowDebugOverlay => "show_debug_overlay",
        InputEffect::ShowLauncher { .. } => "show_launcher",
        InputEffect::ShowPaneSelect { .. } => "show_pane_select",
        InputEffect::ShowCharSelect { .. } => "show_char_select",
        InputEffect::ShowCommandPalette => "show_command_palette",
        InputEffect::ShowPromptInput { .. } => "show_prompt_input",
        InputEffect::ShowInputSelector { .. } => "show_input_selector",
        InputEffect::ShowConfirmation { .. } => "show_confirmation",
        InputEffect::SwitchToWorkspace { .. } => "switch_to_workspace",
        InputEffect::SwitchWorkspaceRelative { .. } => "switch_workspace_relative",
        InputEffect::DetachDomain { .. } => "detach_domain",
        InputEffect::AttachDomain { .. } => "attach_domain",
        InputEffect::QuitApplication => "quit_application",
        InputEffect::HideApplication => "hide_application",
        InputEffect::ReloadConfiguration => "reload_configuration",
        InputEffect::OpenUri { .. } => "open_uri",
        InputEffect::EmitEvent { .. } => "emit_event",
        InputEffect::Invalidate => "invalidate",
        InputEffect::UpdateTitle => "update_title",
        InputEffect::Multiple(_) => "multiple",
        InputEffect::Nop => "nop",
    }
}

impl TermWindow {
    pub fn execute_effects(
        &mut self,
        effects: Vec<InputEffect>,
        pane: &Arc<dyn Pane>,
    ) -> anyhow::Result<()> {
        for effect in effects {
            self.execute_one(effect, pane)?;
        }
        Ok(())
    }

    fn execute_one(&mut self, effect: InputEffect, pane: &Arc<dyn Pane>) -> anyhow::Result<()> {
        match effect {
            InputEffect::ActivateKeyTable {
                name,
                timeout_milliseconds,
                replace_current,
                one_shot,
                until_unknown,
                prevent_fallback,
            } => {
                self.activate_key_table_effect(
                    &name,
                    timeout_milliseconds,
                    replace_current,
                    one_shot,
                    until_unknown,
                    prevent_fallback,
                )?;
            }
            InputEffect::PopKeyTable => {
                self.pop_key_table_effect();
            }
            InputEffect::ClearKeyTableStack => {
                self.clear_key_table_stack_effect();
            }
            InputEffect::ActivateLeader { timeout_ms } => {
                self.activate_leader_effect(timeout_ms);
            }
            InputEffect::SpawnTab { domain } => {
                self.spawn_tab(&domain);
            }
            InputEffect::SpawnWindow => {
                self.spawn_command(&SpawnCommand::default(), SpawnWhere::NewWindow);
            }
            InputEffect::SpawnCommandInNewTab { command } => {
                self.spawn_command(&command, SpawnWhere::NewTab);
            }
            InputEffect::SpawnCommandInNewWindow { command } => {
                self.spawn_command(&command, SpawnWhere::NewWindow);
            }
            InputEffect::SplitPane { split } => {
                log::trace!("SplitPane {:?}", split);
                let direction = match split.direction {
                    PaneDirection::Down | PaneDirection::Up => SplitDirection::Vertical,
                    PaneDirection::Left | PaneDirection::Right => SplitDirection::Horizontal,
                    PaneDirection::Next | PaneDirection::Prev => {
                        log::error!("Invalid direction {:?} for SplitPane", split.direction);
                        return Ok(());
                    }
                };
                let target_is_second = match split.direction {
                    PaneDirection::Down | PaneDirection::Right => true,
                    PaneDirection::Up | PaneDirection::Left => false,
                    PaneDirection::Next | PaneDirection::Prev => unreachable!(),
                };
                let size = match split.size {
                    SplitSize::Percent(n) => MuxSplitSize::Percent(n),
                    SplitSize::Cells(n) => MuxSplitSize::Cells(n),
                };
                self.spawn_command(
                    &split.command,
                    SpawnWhere::SplitPane(SplitRequest {
                        direction,
                        target_is_second,
                        size,
                        top_level: split.top_level,
                    }),
                );
            }
            InputEffect::ToggleFullScreen => {
                if let Some(window) = self.window.as_ref() {
                    window.toggle_fullscreen();
                }
            }
            InputEffect::ToggleAlwaysOnTop => {
                if let Some(window) = self.window.as_ref() {
                    match self.window_state.as_window_level() {
                        WindowLevel::AlwaysOnTop => {
                            window.set_window_level(WindowLevel::Normal);
                        }
                        WindowLevel::AlwaysOnBottom | WindowLevel::Normal => {
                            window.set_window_level(WindowLevel::AlwaysOnTop);
                        }
                    }
                }
            }
            InputEffect::ToggleAlwaysOnBottom => {
                if let Some(window) = self.window.as_ref() {
                    match self.window_state.as_window_level() {
                        WindowLevel::AlwaysOnBottom => {
                            window.set_window_level(WindowLevel::Normal);
                        }
                        WindowLevel::AlwaysOnTop | WindowLevel::Normal => {
                            window.set_window_level(WindowLevel::AlwaysOnBottom);
                        }
                    }
                }
            }
            InputEffect::SetWindowLevel(level) => {
                if let Some(window) = self.window.as_ref() {
                    window.set_window_level(level);
                }
            }
            InputEffect::HideWindow => {
                if let Some(window) = self.window.as_ref() {
                    window.hide();
                }
            }
            InputEffect::ShowWindow => {
                if let Some(window) = self.window.as_ref() {
                    window.show();
                }
            }
            InputEffect::StartWindowDrag => {
                self.start_window_drag_effect();
            }
            InputEffect::AdjustFontSize { delta } => {
                if delta > 0.0 {
                    self.increase_font_size();
                } else if delta < 0.0 {
                    self.decrease_font_size();
                }
            }
            InputEffect::ResetFontSize => {
                self.reset_font_size();
            }
            InputEffect::ResetFontAndWindowSize => {
                if let Some(window) = self.window.clone() {
                    self.reset_font_and_window_size(&window)?;
                }
            }
            InputEffect::ActivateTab { index } => {
                self.activate_tab(index)?;
            }
            InputEffect::ActivateTabRelative { delta, wrap } => {
                self.activate_tab_relative(delta, wrap)?;
            }
            InputEffect::ActivateLastTab => {
                self.activate_last_tab()?;
            }
            InputEffect::MoveTab { index } => {
                self.move_tab(index)?;
            }
            InputEffect::MoveTabRelative { delta } => {
                self.move_tab_relative(delta)?;
            }
            InputEffect::CloseTab { confirm } => {
                self.close_current_tab(confirm);
            }
            InputEffect::ActivatePaneByIndex { index } => {
                let mux = Mux::get();
                let tab = match mux.get_active_tab_for_window(self.mux_window_id) {
                    Some(tab) => tab,
                    None => return Ok(()),
                };
                if self.tab_state(tab.tab_id()).overlay.is_none() {
                    let panes = tab.iter_panes();
                    if panes.iter().any(|p| p.index == index) {
                        tab.set_active_idx(index);
                    }
                }
            }
            InputEffect::ActivatePaneDirection { direction } => {
                let mux = Mux::get();
                let tab = match mux.get_active_tab_for_window(self.mux_window_id) {
                    Some(tab) => tab,
                    None => return Ok(()),
                };
                if self.tab_state(tab.tab_id()).overlay.is_none() {
                    tab.activate_pane_direction(direction);
                }
            }
            InputEffect::AdjustPaneSize { direction, amount } => {
                let mux = Mux::get();
                let tab = match mux.get_active_tab_for_window(self.mux_window_id) {
                    Some(tab) => tab,
                    None => return Ok(()),
                };
                if self.tab_state(tab.tab_id()).overlay.is_none() {
                    tab.adjust_pane_size(direction, amount);
                }
            }
            InputEffect::TogglePaneZoom => {
                let mux = Mux::get();
                if let Some(tab) = mux.get_active_tab_for_window(self.mux_window_id) {
                    tab.toggle_zoom();
                }
            }
            InputEffect::SetPaneZoom { zoomed } => {
                let mux = Mux::get();
                if let Some(tab) = mux.get_active_tab_for_window(self.mux_window_id) {
                    tab.set_zoomed(zoomed);
                }
            }
            InputEffect::ClosePane { confirm } => {
                self.close_current_pane(confirm);
            }
            InputEffect::RotatePanes { direction } => {
                let mux = Mux::get();
                let tab = match mux.get_active_tab_for_window(self.mux_window_id) {
                    Some(tab) => tab,
                    None => return Ok(()),
                };
                match direction {
                    RotationDirection::Clockwise => tab.rotate_clockwise(),
                    RotationDirection::CounterClockwise => tab.rotate_counter_clockwise(),
                }
            }
            InputEffect::ActivateWindow { index } => {
                self.activate_window(index)?;
            }
            InputEffect::ActivateWindowRelative { delta, wrap } => {
                self.activate_window_relative(delta, wrap)?;
            }
            InputEffect::CopySelection { destination } => {
                let text = self.selection_text(pane);
                self.copy_to_clipboard(destination, text);
            }
            InputEffect::CopyText { text, destination } => {
                self.copy_to_clipboard(destination, text);
            }
            InputEffect::Paste { source } => {
                self.paste_from_clipboard(pane, source);
            }
            InputEffect::CompleteSelection { destination } => {
                let text = self.selection_text(pane);
                if !text.is_empty() {
                    self.copy_to_clipboard(destination, text);
                    if let Some(window) = self.window.as_ref() {
                        window.invalidate();
                    }
                }
            }
            InputEffect::CompleteSelectionOrOpenLink { destination } => {
                let text = self.selection_text(pane);
                if !text.is_empty() {
                    self.copy_to_clipboard(destination, text);
                    if let Some(window) = self.window.as_ref() {
                        window.invalidate();
                    }
                } else {
                    self.do_open_link_at_mouse_cursor(pane);
                }
            }
            InputEffect::ScrollByPage { pages } => {
                self.scroll_by_page(pages, pane)?;
            }
            InputEffect::ScrollByLine { lines } => {
                self.scroll_by_line(lines, pane)?;
            }
            InputEffect::ScrollByWheelDelta => {
                self.scroll_by_current_event_wheel_delta(pane)?;
            }
            InputEffect::ScrollToPrompt { direction } => {
                self.scroll_to_prompt(direction, pane)?;
            }
            InputEffect::ScrollToTop => {
                self.scroll_to_top(pane);
            }
            InputEffect::ScrollToBottom => {
                self.scroll_to_bottom(pane);
            }
            InputEffect::SelectAtMouseCursor { mode } => {
                self.select_text_at_mouse_cursor(mode, pane);
            }
            InputEffect::ExtendSelectionToMouse { mode } => {
                self.extend_selection_at_mouse_cursor(mode, pane);
            }
            InputEffect::OpenLinkAtMouseCursor => {
                self.do_open_link_at_mouse_cursor(pane);
            }
            InputEffect::ClearSelection => {
                self.clear_selection(pane);
            }
            InputEffect::SendString { text } => {
                pane.writer().write_all(text.as_bytes())?;
            }
            InputEffect::SendKey { key } => {
                use crate::termwindow::keyevent::Key;
                let mods = key.mods;
                if let Key::Code(key) = self.win_key_code_to_termwiz_key_code(
                    &key.key.resolve(self.config.key_input().key_map_preference),
                ) {
                    pane.key_down(key, mods)?;
                }
            }
            InputEffect::SendToPane { pane_id, data } => {
                let mux = Mux::get();
                if let Some(target) = mux.get_pane(pane_id) {
                    target.writer().write_all(&data)?;
                } else {
                    log::warn!("SendToPane: pane {} not found", pane_id);
                }
            }
            InputEffect::ResetTerminal => {
                pane.perform_actions(vec![Action::Esc(Esc::Code(EscCode::FullReset))]);
            }
            InputEffect::ClearScrollback { mode } => {
                pane.erase_scrollback(mode);
                if let Some(window) = self.window.as_ref() {
                    window.invalidate();
                }
            }
            InputEffect::CopyMode { assignment } => {
                let _ = pane.perform_assignment(&KeyAssignment::CopyMode(assignment));
            }
            InputEffect::ShowCopyMode => {
                if let Some(active) = self.get_active_pane_or_overlay() {
                    let mut replace_current = false;
                    if let Some(existing) = active.downcast_ref::<CopyOverlay>() {
                        let mut params = existing.get_params();
                        params.editing_search = false;
                        existing.apply_params(params);
                        replace_current = true;
                    } else {
                        let copy = CopyOverlay::with_pane(
                            self,
                            &active,
                            CopyModeParams {
                                pattern: MuxPattern::default(),
                                editing_search: false,
                            },
                        )?;
                        self.assign_overlay_for_pane(active.pane_id(), copy);
                    }
                    self.pane_state(active.pane_id())
                        .overlay
                        .as_mut()
                        .map(|overlay| {
                            overlay.key_table_state.activate(KeyTableArgs {
                                name: "copy_mode",
                                timeout_milliseconds: None,
                                replace_current,
                                one_shot: false,
                                until_unknown: false,
                                prevent_fallback: false,
                            });
                        });
                }
            }
            InputEffect::ShowSearch { pattern } => {
                if let Some(active) = self.get_active_pane_or_overlay() {
                    let mut replace_current = false;
                    if let Some(existing) = active.downcast_ref::<CopyOverlay>() {
                        let mut params = existing.get_params();
                        params.editing_search = true;
                        if !pattern.is_empty() {
                            params.pattern = self.resolve_search_pattern(pattern.clone(), &active);
                        }
                        existing.apply_params(params);
                        replace_current = true;
                    } else {
                        let search = CopyOverlay::with_pane(
                            self,
                            &active,
                            CopyModeParams {
                                pattern: self.resolve_search_pattern(pattern, &active),
                                editing_search: true,
                            },
                        )?;
                        self.assign_overlay_for_pane(active.pane_id(), search);
                    }
                    self.pane_state(active.pane_id())
                        .overlay
                        .as_mut()
                        .map(|overlay| {
                            overlay.key_table_state.activate(KeyTableArgs {
                                name: "search_mode",
                                timeout_milliseconds: None,
                                replace_current,
                                one_shot: false,
                                until_unknown: false,
                                prevent_fallback: false,
                            });
                        });
                }
            }
            InputEffect::ShowQuickSelect { args } => {
                if let Some(active) = self.get_active_pane_no_overlay() {
                    let args = args.unwrap_or_default();
                    let overlay = QuickSelectOverlay::with_pane(self, &active, &args);
                    self.assign_overlay_for_pane(active.pane_id(), overlay);
                }
            }
            InputEffect::ShowTabNavigator => {
                self.show_tab_navigator();
            }
            InputEffect::ShowDebugOverlay => {
                self.show_debug_overlay();
            }
            InputEffect::ShowLauncher { args } => {
                if let Some(args) = args {
                    let title = args.title.unwrap_or_else(|| "Launcher".to_string());
                    let args = LauncherActionArgs {
                        title: Some(title),
                        flags: args.flags,
                        help_text: args.help_text,
                        fuzzy_help_text: args.fuzzy_help_text,
                        alphabet: args.alphabet,
                    };
                    self.show_launcher_impl(args, 0);
                } else {
                    self.show_launcher();
                }
            }
            InputEffect::ShowPaneSelect { args } => {
                let modal = crate::termwindow::paneselect::PaneSelector::new(self, &args);
                self.set_modal(Rc::new(modal));
            }
            InputEffect::ShowCharSelect { args } => {
                let modal = crate::termwindow::charselect::CharSelector::new(self, &args);
                self.set_modal(Rc::new(modal));
            }
            InputEffect::ShowCommandPalette => {
                let modal = crate::termwindow::palette::CommandPalette::new(self);
                self.set_modal(Rc::new(modal));
            }
            InputEffect::ShowPromptInput { args } => {
                self.show_prompt_input_line(&args);
            }
            InputEffect::ShowInputSelector { args } => {
                self.show_input_selector(&args);
            }
            InputEffect::ShowConfirmation { args } => {
                self.show_confirmation(&args);
            }
            InputEffect::SwitchToWorkspace { name, spawn } => {
                self.switch_to_workspace_effect(name, spawn);
            }
            InputEffect::SwitchWorkspaceRelative { delta } => {
                let mux = Mux::get();
                let workspace = mux.active_workspace();
                let workspaces = mux.iter_workspaces();
                if workspaces.is_empty() {
                    return Ok(());
                }
                let idx = workspaces.iter().position(|w| *w == workspace).unwrap_or(0);
                let new_idx = idx as isize + delta;
                let new_idx = if new_idx < 0 {
                    workspaces.len() as isize + new_idx
                } else {
                    new_idx
                };
                let new_idx = new_idx as usize % workspaces.len();
                if let Some(name) = workspaces.get(new_idx) {
                    front_end().switch_workspace(name);
                }
            }
            InputEffect::DetachDomain { domain } => {
                let domain = Mux::get().resolve_spawn_tab_domain(Some(pane.pane_id()), &domain)?;
                domain.detach()?;
            }
            InputEffect::AttachDomain { name } => {
                let window = self.mux_window_id;
                let dpi = self.dimensions.dpi as u32;
                promise::spawn::spawn(async move {
                    let mux = Mux::get();
                    let domain = mux
                        .get_domain_by_name(&name)
                        .ok_or_else(|| anyhow!("{} is not a valid domain name", name))?;
                    domain.attach(Some(window)).await?;
                    let have_panes_in_domain = mux
                        .iter_panes()
                        .iter()
                        .any(|pane| pane.domain_id() == domain.domain_id());
                    if !have_panes_in_domain {
                        let config = config::configuration();
                        let _tab = domain
                            .spawn(
                                config.initial_size(
                                    dpi,
                                    Some(crate::cell_pixel_dims(&config, dpi as f64)?),
                                ),
                                None,
                                None,
                                window,
                            )
                            .await?;
                    }
                    Result::<(), anyhow::Error>::Ok(())
                })
                .detach();
            }
            InputEffect::QuitApplication => {
                let mux = Mux::get();
                log::info!("QuitApplication over here (window)");
                match self.config.window_config().window_close_confirmation {
                    WindowCloseConfirmation::NeverPrompt => {
                        let con = Connection::get().expect("call on gui thread");
                        con.terminate_message_loop();
                    }
                    WindowCloseConfirmation::AlwaysPrompt => {
                        let tab = mux
                            .get_active_tab_for_window(self.mux_window_id)
                            .ok_or_else(|| anyhow!("no active tab!?"))?;
                        let window = self
                            .window
                            .clone()
                            .ok_or_else(|| anyhow!("window is not available"))?;
                        let (overlay, future) =
                            start_overlay(self, &tab, move |tab_id, term| {
                                confirm_quit_program(term, window, tab_id)
                            });
                        self.assign_overlay(tab.tab_id(), overlay);
                        promise::spawn::spawn(future).detach();
                    }
                }
            }
            InputEffect::HideApplication => {
                let con = Connection::get().expect("call on gui thread");
                con.hide_application();
            }
            InputEffect::ReloadConfiguration => {
                config::reload();
            }
            InputEffect::OpenUri { uri } => {
                phaedra_open_url::open_url(&uri);
            }
            InputEffect::EmitEvent { name } => {
                self.emit_window_event(&name, None);
            }
            InputEffect::Invalidate => {
                if let Some(window) = self.window.as_ref() {
                    window.invalidate();
                }
            }
            InputEffect::UpdateTitle => {
                self.update_title();
            }
            InputEffect::Multiple(effects) => {
                self.execute_effects(effects, pane)?;
            }
            InputEffect::Nop => {}
        }

        Ok(())
    }
}
