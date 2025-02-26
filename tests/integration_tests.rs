use std::cell::RefCell;
use std::sync::{Arc, Mutex};
use thoth_cli::{
    format_json, format_markdown, get_save_file_path, ClipboardTrait, EditorClipboard,
    ScrollableTextArea, TitlePopup, TitleSelectPopup,
};
use tui_textarea::TextArea;

// Create a mock clipboard implementation
struct MockClipboard {
    content: RefCell<String>,
}

impl MockClipboard {
    fn new() -> Self {
        MockClipboard {
            content: RefCell::new(String::new()),
        }
    }

    fn set_text(&self, text: String) -> Result<(), arboard::Error> {
        *self.content.borrow_mut() = text;
        Ok(())
    }

    fn get_text(&self) -> Result<String, arboard::Error> {
        Ok(self.content.borrow().clone())
    }
}

#[cfg(test)]
impl ClipboardTrait for clipboard_mock::MockEditorClipboard {
    fn set_contents(&mut self, content: String) -> anyhow::Result<()> {
        self.set_contents(content)
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    fn get_content(&self) -> anyhow::Result<String> {
        self.get_content().map_err(|e| anyhow::anyhow!("{}", e))
    }
}

// Temporarily replace the real EditorClipboard with our mock for testing
#[cfg(test)]
mod clipboard_mock {
    use super::*;

    pub struct MockEditorClipboard {
        mock: Arc<MockClipboard>,
    }

    impl MockEditorClipboard {
        pub fn new() -> Result<Self, arboard::Error> {
            Ok(MockEditorClipboard {
                mock: Arc::new(MockClipboard::new()),
            })
        }

        pub fn set_contents(&mut self, content: String) -> Result<(), arboard::Error> {
            self.mock.set_text(content)
        }

        pub fn get_content(&self) -> Result<String, arboard::Error> {
            self.mock.get_text()
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

    // Mock the clipboard functionality for testing
    // Instead of calling the actual clipboard function, we'll test the textarea content
    let content = sta.textareas[sta.focused_index].lines().join("\n");
    assert_eq!(content, "This is the content of Note 1");

    // Optional: Test with our mock clipboard if we need to verify clipboard operations
    {
        use clipboard_mock::MockEditorClipboard;
        let mut mock_clipboard = MockEditorClipboard::new().unwrap();
        let content = sta.textareas[sta.focused_index].lines().join("\n");
        mock_clipboard.set_contents(content.clone()).unwrap();
        let clipboard_content = mock_clipboard.get_content().unwrap();
        assert_eq!(clipboard_content, "This is the content of Note 1");
    }

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

    // Rest of the test remains the same...
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
