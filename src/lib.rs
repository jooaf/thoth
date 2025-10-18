pub mod cli;
pub mod clipboard;
pub mod code_block_popup;
pub mod config;
pub mod formatter;
pub mod markdown_renderer;
pub mod scrollable_textarea;
pub mod theme;
pub mod title_popup;
pub mod title_select_popup;
pub mod ui;
pub mod ui_handler;
pub mod utils;

pub use clipboard::ClipboardTrait;
pub use clipboard::EditorClipboard;
pub use code_block_popup::CodeBlockPopup;
pub use config::{ThemeMode, ThothConfig};
use dirs::home_dir;
pub use formatter::{format_json, format_markdown};
pub use markdown_renderer::MarkdownRenderer;
pub use scrollable_textarea::ScrollableTextArea;
use std::path::PathBuf;
pub use theme::{ThemeColors, DARK_MODE_COLORS, LIGHT_MODE_COLORS};
pub use title_popup::TitlePopup;
pub use title_select_popup::TitleSelectPopup;
pub use utils::{load_textareas, save_textareas};

pub fn get_save_file_path() -> PathBuf {
    // First, check environment variable
    if let Ok(custom_dir) = std::env::var("THOTH_NOTES_DIR") {
        return PathBuf::from(shellexpand::tilde(&custom_dir).to_string()).join("thoth_notes.md");
    }

    // Second, check config file
    if let Ok(config) = ThothConfig::load() {
        if let Some(notes_dir) = config.notes_dir {
            return PathBuf::from(shellexpand::tilde(&notes_dir).to_string())
                .join("thoth_notes.md");
        }
    }

    // Finally, use default
    home_dir().unwrap_or_default().join("thoth_notes.md")
}

pub fn get_save_backup_file_path() -> PathBuf {
    let main_path = get_save_file_path();
    let parent = main_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    parent.join("thoth_notes_backup.md")
}

pub fn get_clipboard_backup_file_path() -> PathBuf {
    let main_path = get_save_file_path();
    let parent = main_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    parent.join("thoth_clipboard.txt")
}

// The ORANGE constant is kept for backward compatibility
pub const ORANGE: ratatui::style::Color = ratatui::style::Color::Rgb(255, 165, 0);
pub const DAEMONIZE_ARG: &str = "__thoth_copy_daemonize";
pub const MIN_TEXTAREA_HEIGHT: usize = 3;
pub const BORDER_PADDING_SIZE: usize = 2;
