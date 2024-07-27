use thoth_cli::{
    format_json, format_markdown, get_save_file_path, ScrollableTextArea, TitlePopup,
    TitleSelectPopup,
};
use tui_textarea::TextArea;

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
    assert_eq!(sta.focused_index, 1);
    sta.move_focus(-1);
    assert_eq!(sta.focused_index, 0);

    // Test title change
    sta.change_title("Updated Note 1".to_string());
    assert_eq!(sta.titles[0], "Updated Note 1");

    // Test copy functionality (note: this should return an error)
    // since the display is not connected in github actions
    assert!(sta.copy_textarea_contents().is_err());

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
fn test_scrollable_textarea_scroll_behavior() {
    let mut sta = ScrollableTextArea::new();
    for i in 0..20 {
        sta.add_textarea(TextArea::default(), format!("Note {}", i));
    }

    sta.viewport_height = 10;
    sta.focused_index = 15;
    sta.adjust_scroll_to_focused();

    assert!(sta.scroll > 0);
    assert!(sta.scroll <= sta.focused_index);
}

#[test]
fn test_markdown_renderer_with_code_blocks() {
    let renderer = thoth_cli::MarkdownRenderer::new();
    let markdown =
        "# Header\n\n```rust\nfn main() {\n    println!(\"Hello, world!\");\n}\n```".to_string();
    let rendered = renderer.render_markdown(&markdown, 40).unwrap();

    assert!(rendered.lines.len() > 5);
    assert!(rendered.lines[0]
        .spans
        .iter()
        .any(|span| span.content.contains("Header")));
}
