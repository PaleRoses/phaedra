use config::keyassignment::*;
use config::KeyNoAction;
use config::window::WindowLevel;
use mux::pane::PaneId;

#[derive(Debug, Clone)]
pub enum InputEffect {
    ActivateKeyTable {
        name: String,
        timeout_milliseconds: Option<u64>,
        replace_current: bool,
        one_shot: bool,
        until_unknown: bool,
        prevent_fallback: bool,
    },
    PopKeyTable,
    ClearKeyTableStack,
    ActivateLeader {
        timeout_ms: u64,
    },
    SpawnTab {
        domain: SpawnTabDomain,
    },
    SpawnWindow,
    SpawnCommandInNewTab {
        command: SpawnCommand,
    },
    SpawnCommandInNewWindow {
        command: SpawnCommand,
    },
    SplitPane {
        split: SplitPane,
    },
    ToggleFullScreen,
    ToggleAlwaysOnTop,
    ToggleAlwaysOnBottom,
    SetWindowLevel(WindowLevel),
    HideWindow,
    ShowWindow,
    StartWindowDrag,
    AdjustFontSize {
        delta: f64,
    },
    ResetFontSize,
    ResetFontAndWindowSize,
    ActivateTab {
        index: isize,
    },
    ActivateTabRelative {
        delta: isize,
        wrap: bool,
    },
    ActivateLastTab,
    MoveTab {
        index: usize,
    },
    MoveTabRelative {
        delta: isize,
    },
    CloseTab {
        confirm: bool,
    },
    ActivatePaneByIndex {
        index: usize,
    },
    ActivatePaneDirection {
        direction: PaneDirection,
    },
    AdjustPaneSize {
        direction: PaneDirection,
        amount: usize,
    },
    TogglePaneZoom,
    SetPaneZoom {
        zoomed: bool,
    },
    ClosePane {
        confirm: bool,
    },
    RotatePanes {
        direction: RotationDirection,
    },
    ActivateWindow {
        index: usize,
    },
    ActivateWindowRelative {
        delta: isize,
        wrap: bool,
    },
    CopySelection {
        destination: ClipboardCopyDestination,
    },
    CopyText {
        text: String,
        destination: ClipboardCopyDestination,
    },
    Paste {
        source: ClipboardPasteSource,
    },
    CompleteSelection {
        destination: ClipboardCopyDestination,
    },
    CompleteSelectionOrOpenLink {
        destination: ClipboardCopyDestination,
    },
    ScrollByPage {
        pages: f64,
    },
    ScrollByLine {
        lines: isize,
    },
    ScrollByWheelDelta,
    ScrollToPrompt {
        direction: isize,
    },
    ScrollToTop,
    ScrollToBottom,
    SelectAtMouseCursor {
        mode: SelectionMode,
    },
    ExtendSelectionToMouse {
        mode: SelectionMode,
    },
    OpenLinkAtMouseCursor,
    ClearSelection,
    SendString {
        text: String,
    },
    SendKey {
        key: KeyNoAction,
    },
    SendToPane {
        pane_id: PaneId,
        data: Vec<u8>,
    },
    ResetTerminal,
    ClearScrollback {
        mode: ScrollbackEraseMode,
    },
    CopyMode {
        assignment: CopyModeAssignment,
    },
    ShowCopyMode,
    ShowSearch {
        pattern: Pattern,
    },
    ShowQuickSelect {
        args: Option<QuickSelectArguments>,
    },
    ShowTabNavigator,
    ShowDebugOverlay,
    ShowLauncher {
        args: Option<LauncherActionArgs>,
    },
    ShowPaneSelect {
        args: PaneSelectArguments,
    },
    ShowCharSelect {
        args: CharSelectArguments,
    },
    ShowCommandPalette,
    ShowPromptInput {
        args: PromptInputLine,
    },
    ShowInputSelector {
        args: InputSelector,
    },
    ShowConfirmation {
        args: Confirmation,
    },
    SwitchToWorkspace {
        name: Option<String>,
        spawn: Option<SpawnCommand>,
    },
    SwitchWorkspaceRelative {
        delta: isize,
    },
    DetachDomain {
        domain: SpawnTabDomain,
    },
    AttachDomain {
        name: String,
    },
    QuitApplication,
    HideApplication,
    ReloadConfiguration,
    OpenUri {
        uri: String,
    },
    EmitEvent {
        name: String,
    },
    Invalidate,
    UpdateTitle,
    Multiple(Vec<InputEffect>),
    Nop,
}

impl InputEffect {
    pub fn is_nop(&self) -> bool {
        matches!(self, InputEffect::Nop)
    }

    pub fn is_invalidate(&self) -> bool {
        matches!(self, InputEffect::Invalidate)
    }

    pub fn fold<T, F>(&self, init: T, f: &F) -> T
    where
        F: Fn(T, &InputEffect) -> T,
    {
        match self {
            InputEffect::Multiple(effects) => effects.iter().fold(init, |acc, e| e.fold(acc, f)),
            _ => f(init, self),
        }
    }
}
