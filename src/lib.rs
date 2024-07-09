pub mod markdown_renderer;
pub mod scrollable_textarea;
pub mod title_popup;
pub mod title_select_popup;
pub mod ui;

pub use markdown_renderer::MarkdownRenderer;
pub use scrollable_textarea::ScrollableTextArea;
pub use title_popup::TitlePopup;
pub use title_select_popup::TitleSelectPopup;

pub const SAVE_FILE: &str = "thoth_notes.md";
pub const EMBEDDED_FILE: &str = include_str!("../thoth_notes.md");
pub const ORANGE: ratatui::style::Color = ratatui::style::Color::Rgb(255, 165, 0);
