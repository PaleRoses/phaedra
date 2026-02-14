use config::keyassignment::*;

use crate::input_effect::InputEffect;

pub fn interpret_assignment(assignment: &KeyAssignment) -> Vec<InputEffect> {
    match assignment {
        KeyAssignment::SpawnTab(domain) => vec![InputEffect::SpawnTab {
            domain: domain.clone(),
        }],
        KeyAssignment::SpawnWindow => vec![InputEffect::SpawnWindow],
        KeyAssignment::ToggleFullScreen => vec![InputEffect::ToggleFullScreen],
        KeyAssignment::ToggleAlwaysOnTop => vec![InputEffect::ToggleAlwaysOnTop],
        KeyAssignment::ToggleAlwaysOnBottom => vec![InputEffect::ToggleAlwaysOnBottom],
        KeyAssignment::SetWindowLevel(level) => vec![InputEffect::SetWindowLevel(level.clone())],
        KeyAssignment::CopyTo(destination) => vec![InputEffect::CopySelection {
            destination: *destination,
        }],
        KeyAssignment::CopyTextTo { text, destination } => vec![InputEffect::CopyText {
            text: text.clone(),
            destination: *destination,
        }],
        KeyAssignment::PasteFrom(source) => vec![InputEffect::Paste { source: *source }],
        KeyAssignment::ActivateTabRelative(delta) => vec![InputEffect::ActivateTabRelative {
            delta: *delta,
            wrap: true,
        }],
        KeyAssignment::ActivateTabRelativeNoWrap(delta) => {
            vec![InputEffect::ActivateTabRelative {
                delta: *delta,
                wrap: false,
            }]
        }
        KeyAssignment::IncreaseFontSize => vec![InputEffect::AdjustFontSize { delta: 1.0 }],
        KeyAssignment::DecreaseFontSize => vec![InputEffect::AdjustFontSize { delta: -1.0 }],
        KeyAssignment::ResetFontSize => vec![InputEffect::ResetFontSize],
        KeyAssignment::ResetFontAndWindowSize => vec![InputEffect::ResetFontAndWindowSize],
        KeyAssignment::ActivateTab(index) => vec![InputEffect::ActivateTab { index: *index }],
        KeyAssignment::ActivateLastTab => vec![InputEffect::ActivateLastTab],
        KeyAssignment::SendString(text) => vec![InputEffect::SendString { text: text.clone() }],
        KeyAssignment::SendKey(key) => vec![InputEffect::SendKey { key: key.clone() }],
        KeyAssignment::Nop => vec![InputEffect::Nop],
        KeyAssignment::DisableDefaultAssignment => vec![InputEffect::Nop],
        KeyAssignment::Hide => vec![InputEffect::HideWindow],
        KeyAssignment::Show => vec![InputEffect::ShowWindow],
        KeyAssignment::CloseCurrentTab { confirm } => {
            vec![InputEffect::CloseTab { confirm: *confirm }]
        }
        KeyAssignment::ReloadConfiguration => vec![InputEffect::ReloadConfiguration],
        KeyAssignment::MoveTabRelative(delta) => {
            vec![InputEffect::MoveTabRelative { delta: *delta }]
        }
        KeyAssignment::MoveTab(index) => vec![InputEffect::MoveTab { index: *index }],
        KeyAssignment::ScrollByPage(pages) => vec![
            InputEffect::ScrollByPage {
                pages: (*pages).into(),
            },
            InputEffect::Invalidate,
        ],
        KeyAssignment::ScrollByLine(lines) => vec![
            InputEffect::ScrollByLine { lines: *lines },
            InputEffect::Invalidate,
        ],
        KeyAssignment::ScrollByCurrentEventWheelDelta => {
            vec![InputEffect::ScrollByWheelDelta, InputEffect::Invalidate]
        }
        KeyAssignment::ScrollToPrompt(direction) => vec![
            InputEffect::ScrollToPrompt {
                direction: *direction,
            },
            InputEffect::Invalidate,
        ],
        KeyAssignment::ScrollToTop => vec![InputEffect::ScrollToTop, InputEffect::Invalidate],
        KeyAssignment::ScrollToBottom => vec![InputEffect::ScrollToBottom, InputEffect::Invalidate],
        KeyAssignment::ShowTabNavigator => vec![InputEffect::ShowTabNavigator],
        KeyAssignment::ShowDebugOverlay => vec![InputEffect::ShowDebugOverlay],
        KeyAssignment::HideApplication => vec![InputEffect::HideApplication],
        KeyAssignment::QuitApplication => vec![InputEffect::QuitApplication],
        KeyAssignment::SpawnCommandInNewTab(command) => {
            vec![InputEffect::SpawnCommandInNewTab {
                command: command.clone(),
            }]
        }
        KeyAssignment::SpawnCommandInNewWindow(command) => {
            vec![InputEffect::SpawnCommandInNewWindow {
                command: command.clone(),
            }]
        }
        KeyAssignment::SplitHorizontal(command) => vec![InputEffect::SplitPane {
            split: SplitPane {
                direction: PaneDirection::Right,
                size: SplitSize::Percent(50),
                command: command.clone(),
                top_level: false,
            },
        }],
        KeyAssignment::SplitVertical(command) => vec![InputEffect::SplitPane {
            split: SplitPane {
                direction: PaneDirection::Down,
                size: SplitSize::Percent(50),
                command: command.clone(),
                top_level: false,
            },
        }],
        KeyAssignment::ShowLauncher => vec![InputEffect::ShowLauncher { args: None }],
        KeyAssignment::ShowLauncherArgs(args) => vec![InputEffect::ShowLauncher {
            args: Some(args.clone()),
        }],
        KeyAssignment::ClearScrollback(mode) => vec![InputEffect::ClearScrollback { mode: *mode }],
        KeyAssignment::Search(pattern) => vec![InputEffect::ShowSearch {
            pattern: pattern.clone(),
        }],
        KeyAssignment::ActivateCopyMode => vec![InputEffect::ShowCopyMode],
        KeyAssignment::SelectTextAtMouseCursor(mode) => {
            vec![InputEffect::SelectAtMouseCursor { mode: *mode }]
        }
        KeyAssignment::ExtendSelectionToMouseCursor(mode) => {
            vec![InputEffect::ExtendSelectionToMouse { mode: *mode }]
        }
        KeyAssignment::OpenLinkAtMouseCursor => vec![InputEffect::OpenLinkAtMouseCursor],
        KeyAssignment::ClearSelection => vec![InputEffect::ClearSelection],
        KeyAssignment::CompleteSelection(destination) => vec![InputEffect::CompleteSelection {
            destination: *destination,
        }],
        KeyAssignment::CompleteSelectionOrOpenLinkAtMouseCursor(destination) => {
            vec![InputEffect::CompleteSelectionOrOpenLink {
                destination: *destination,
            }]
        }
        KeyAssignment::StartWindowDrag => vec![InputEffect::StartWindowDrag],
        KeyAssignment::AdjustPaneSize(direction, amount) => vec![InputEffect::AdjustPaneSize {
            direction: *direction,
            amount: *amount,
        }],
        KeyAssignment::ActivatePaneDirection(direction) => {
            vec![InputEffect::ActivatePaneDirection {
                direction: *direction,
            }]
        }
        KeyAssignment::ActivatePaneByIndex(index) => {
            vec![InputEffect::ActivatePaneByIndex { index: *index }]
        }
        KeyAssignment::TogglePaneZoomState => vec![InputEffect::TogglePaneZoom],
        KeyAssignment::SetPaneZoomState(zoomed) => {
            vec![InputEffect::SetPaneZoom { zoomed: *zoomed }]
        }
        KeyAssignment::CloseCurrentPane { confirm } => {
            vec![InputEffect::ClosePane { confirm: *confirm }]
        }
        KeyAssignment::EmitEvent(name) => vec![InputEffect::EmitEvent { name: name.clone() }],
        KeyAssignment::QuickSelect => vec![InputEffect::ShowQuickSelect { args: None }],
        KeyAssignment::QuickSelectArgs(args) => vec![InputEffect::ShowQuickSelect {
            args: Some(args.clone()),
        }],
        KeyAssignment::Multiple(assignments) => vec![InputEffect::Multiple(
            assignments.iter().flat_map(interpret_assignment).collect(),
        )],
        KeyAssignment::SwitchToWorkspace { name, spawn } => vec![InputEffect::SwitchToWorkspace {
            name: name.clone(),
            spawn: spawn.clone(),
        }],
        KeyAssignment::SwitchWorkspaceRelative(delta) => {
            vec![InputEffect::SwitchWorkspaceRelative { delta: *delta }]
        }
        KeyAssignment::ActivateKeyTable {
            name,
            timeout_milliseconds,
            replace_current,
            one_shot,
            until_unknown,
            prevent_fallback,
        } => vec![InputEffect::ActivateKeyTable {
            name: name.clone(),
            timeout_milliseconds: *timeout_milliseconds,
            replace_current: *replace_current,
            one_shot: *one_shot,
            until_unknown: *until_unknown,
            prevent_fallback: *prevent_fallback,
        }],
        KeyAssignment::PopKeyTable => vec![InputEffect::PopKeyTable],
        KeyAssignment::ClearKeyTableStack => vec![InputEffect::ClearKeyTableStack],
        KeyAssignment::DetachDomain(domain) => vec![InputEffect::DetachDomain {
            domain: domain.clone(),
        }],
        KeyAssignment::AttachDomain(name) => vec![InputEffect::AttachDomain { name: name.clone() }],
        KeyAssignment::CopyMode(assignment) => vec![InputEffect::CopyMode {
            assignment: assignment.clone(),
        }],
        KeyAssignment::RotatePanes(direction) => vec![InputEffect::RotatePanes {
            direction: direction.clone(),
        }],
        KeyAssignment::SplitPane(split) => vec![InputEffect::SplitPane {
            split: split.clone(),
        }],
        KeyAssignment::PaneSelect(args) => vec![InputEffect::ShowPaneSelect { args: args.clone() }],
        KeyAssignment::CharSelect(args) => vec![InputEffect::ShowCharSelect { args: args.clone() }],
        KeyAssignment::ResetTerminal => vec![InputEffect::ResetTerminal],
        KeyAssignment::OpenUri(uri) => vec![InputEffect::OpenUri { uri: uri.clone() }],
        KeyAssignment::ActivateCommandPalette => vec![InputEffect::ShowCommandPalette],
        KeyAssignment::ActivateWindow(index) => vec![InputEffect::ActivateWindow { index: *index }],
        KeyAssignment::ActivateWindowRelative(delta) => vec![InputEffect::ActivateWindowRelative {
            delta: *delta,
            wrap: true,
        }],
        KeyAssignment::ActivateWindowRelativeNoWrap(delta) => {
            vec![InputEffect::ActivateWindowRelative {
                delta: *delta,
                wrap: false,
            }]
        }
        KeyAssignment::PromptInputLine(args) => {
            vec![InputEffect::ShowPromptInput { args: args.clone() }]
        }
        KeyAssignment::InputSelector(args) => {
            vec![InputEffect::ShowInputSelector { args: args.clone() }]
        }
        KeyAssignment::Confirmation(args) => {
            vec![InputEffect::ShowConfirmation { args: args.clone() }]
        }
    }
}
