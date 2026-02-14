pub enum TerminalEffect {
    WritePty(Vec<u8>),
    Bell,
    TitleChanged(String),
    IconTitleChanged(String),
    TabTitleChanged(String),
    ToastNotification {
        title: Option<String>,
        body: String,
        focus: bool,
    },
    WorkingDirectoryChanged(Option<url::Url>),
    PaletteChanged,
    SetUserVar {
        name: String,
        value: String,
    },
    OutputSinceFocusLost,
    ProgressChanged(Option<u8>),
    SetClipboard {
        clipboard: Option<String>,
        selection: Option<String>,
    },
    SaveToDownloads {
        name: Option<String>,
        data: Vec<u8>,
    },
    DeviceControl(Box<crate::DeviceControlMode>),
}
