use anyhow::Result;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};
use thoth_cli::{
    format_json, format_markdown, get_save_file_path, EditorClipboard, ScrollableTextArea,
    TitlePopup, TitleSelectPopup,
};
use tui_textarea::TextArea;

#[cfg(test)]
mod test_utils {
    use super::*;
    use std::sync::{Arc, Mutex};

    pub struct MockClipboard {
        content: String,
    }

    impl MockClipboard {
        pub fn new() -> Self {
            MockClipboard {
                content: String::new(),
            }
        }

        pub fn set_content(&mut self, content: String) -> Result<()> {
            self.content = content;
            Ok(())
        }

        pub fn get_content(&self) -> Result<String> {
            Ok(self.content.clone())
        }
    }
}

#[test]
fn test_full_application_flow() {
    // Initialize ScrollableTextArea
    let mut sta = ScrollableTextArea::new();

    // Add textareas
    sta.add_textarea(TextArea::default(), "Note 1".to_string());
    sta.add_textarea(TextArea::default(), "Note 2".to_string());
    assert_eq!(sta.textareas.len(), 2);
    assert_eq!(sta.titles.len(), 2);

    // Edit content
    sta.textareas[0].insert_str("This is the content of Note 1");
    sta.textareas[1].insert_str("This is the content of Note 2");

    // Test focus movement
    sta.move_focus(1);
    assert_eq!(sta.focused_index, 0);
    sta.move_focus(-1);
    assert_eq!(sta.focused_index, 1);
    sta.move_focus(1);
    assert_eq!(sta.focused_index, 0);

    // Test title change
    sta.change_title("Updated Note 1".to_string());
    assert_eq!(sta.titles[0], "Updated Note 1");

    // Test clipboard content extraction
    let expected_content = sta.test_get_clipboard_content();
    assert_eq!(expected_content, "This is the content of Note 1");

    // Create and test with mock clipboard
    let mut mock_clipboard = test_utils::MockClipboard::new();
    let _ = mock_clipboard.set_content(expected_content.clone());
    let clipboard_content = mock_clipboard.get_content().unwrap();
    assert_eq!(clipboard_content, "This is the content of Note 1");

    // Test remove textarea
    sta.remove_textarea(1);
    assert_eq!(sta.textareas.len(), 1);
    assert_eq!(sta.titles.len(), 1);

    // Test full screen toggle
    sta.toggle_full_screen();
    assert!(sta.full_screen_mode);
    assert!(!sta.edit_mode);

    // Test markdown formatting
    let markdown_content = "# Header\n\nThis is **bold** and *italic* text.";
    let formatted_markdown = format_markdown(markdown_content).unwrap();
    assert!(formatted_markdown.contains("# Header"));
    assert!(formatted_markdown.contains("**bold**"));
    assert!(formatted_markdown.contains("*italic*"));

    // Test JSON formatting
    let json_content = r#"{"name":"John","age":30}"#;
    let formatted_json = format_json(json_content).unwrap();
    assert!(formatted_json.contains("\"name\": \"John\""));
    assert!(formatted_json.contains("\"age\": 30"));

    // Test TitlePopup
    let mut title_popup = TitlePopup::new();
    title_popup.title = "New Title".to_string();
    title_popup.visible = true;
    assert_eq!(title_popup.title, "New Title");
    assert!(title_popup.visible);

    // Test TitleSelectPopup
    let mut title_select_popup = TitleSelectPopup::new();
    title_select_popup.titles = vec!["Title1".to_string(), "Title2".to_string()];
    title_select_popup.selected_index = 1;
    title_select_popup.visible = true;
    assert_eq!(title_select_popup.titles.len(), 2);
    assert_eq!(title_select_popup.selected_index, 1);
    assert!(title_select_popup.visible);

    // Test save file path
    let save_path = get_save_file_path();
    assert!(save_path.ends_with("thoth_notes.md"));
}

#[test]
fn test_clipboard_functionality() {
    // Create a mock clipboard
    let mut mock_clipboard = test_utils::MockClipboard::new();

    // Initialize ScrollableTextArea
    let mut sta = ScrollableTextArea::new();

    // Create a textarea with content
    let mut textarea = TextArea::default();
    textarea.insert_str("Test clipboard content");
    sta.add_textarea(textarea, "Clipboard Test".to_string());

    // Get the content that would be copied
    let content = sta.textareas[sta.focused_index].lines().join("\n");

    // Store it in our mock clipboard
    mock_clipboard.set_content(content).unwrap();

    // Retrieve from mock clipboard
    let clipboard_content = mock_clipboard.get_content().unwrap();

    // Verify content
    assert_eq!(clipboard_content, "Test clipboard content");

    // Test copy selection by mocking line selection
    sta.start_sel = 0;
    let current_line = sta.textareas[sta.focused_index].lines()[0].clone();
    mock_clipboard.set_content(current_line.clone()).unwrap();

    // Verify selection content
    assert_eq!(
        mock_clipboard.get_content().unwrap(),
        "Test clipboard content"
    );
}
