use crate::config::NotificationHandling;
use phaedra_dynamic::{FromDynamic, ToDynamic};
use termwiz::hyperlink;

#[derive(Debug, Clone, FromDynamic, ToDynamic)]
pub struct TerminalFeatureConfig {
    #[dynamic(default = "default_true")]
    pub enable_kitty_graphics: bool,
    #[dynamic(default)]
    pub enable_kitty_keyboard: bool,
    #[dynamic(default)]
    pub enable_title_reporting: bool,
    #[dynamic(default = "default_true")]
    pub allow_download_protocols: bool,
    #[dynamic(default = "default_true")]
    pub allow_win32_input_mode: bool,
    #[dynamic(default = "default_true")]
    pub detect_password_input: bool,
    #[dynamic(default = "default_enq_answerback")]
    pub enq_answerback: String,
    #[dynamic(default)]
    pub notification_handling: NotificationHandling,
    #[dynamic(default = "default_hyperlink_rules")]
    pub hyperlink_rules: Vec<hyperlink::Rule>,
}

impl Default for TerminalFeatureConfig {
    fn default() -> Self {
        Self {
            enable_kitty_graphics: default_true(),
            enable_kitty_keyboard: false,
            enable_title_reporting: false,
            allow_download_protocols: default_true(),
            allow_win32_input_mode: default_true(),
            detect_password_input: default_true(),
            enq_answerback: default_enq_answerback(),
            notification_handling: NotificationHandling::default(),
            hyperlink_rules: default_hyperlink_rules(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_enq_answerback() -> String {
    String::new()
}

pub(crate) fn default_hyperlink_rules() -> Vec<hyperlink::Rule> {
    vec![
        hyperlink::Rule::with_highlight(r"\((\w+://\S+)\)", "$1", 1).unwrap(),
        hyperlink::Rule::with_highlight(r"\[(\w+://\S+)\]", "$1", 1).unwrap(),
        hyperlink::Rule::with_highlight(r"<(\w+://\S+)>", "$1", 1).unwrap(),
        hyperlink::Rule::new(hyperlink::CLOSING_PARENTHESIS_HYPERLINK_PATTERN, "$0").unwrap(),
        hyperlink::Rule::new(hyperlink::GENERIC_HYPERLINK_PATTERN, "$0").unwrap(),
        hyperlink::Rule::new(r"\b\w+@[\w-]+(\.[\w-]+)+\b", "mailto:$0").unwrap(),
    ]
}
