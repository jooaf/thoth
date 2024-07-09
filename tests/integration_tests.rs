use thoth::{ScrollableTextArea, TitlePopup, TitleSelectPopup};
use tui_textarea::TextArea;

#[test]
fn test_scrollable_textarea_integration() {
    let mut sta = ScrollableTextArea::new();
    sta.add_textarea(TextArea::default(), "Test1".to_string());
    sta.add_textarea(TextArea::default(), "Test2".to_string());

    assert_eq!(sta.textareas.len(), 2);
    assert_eq!(sta.titles.len(), 2);
    assert_eq!(sta.focused_index, 1);

    sta.move_focus(-1);
    assert_eq!(sta.focused_index, 0);

    sta.change_title("Updated Test1".to_string());
    assert_eq!(sta.titles[0], "Updated Test1");

    sta.remove_textarea(0);
    assert_eq!(sta.textareas.len(), 1);
    assert_eq!(sta.titles.len(), 1);
    assert_eq!(sta.titles[0], "Test2");
}

#[test]
fn test_title_popup_integration() {
    let mut popup = TitlePopup::new();
    popup.title = "New Title".to_string();
    popup.visible = true;

    assert_eq!(popup.title, "New Title");
    assert!(popup.visible);
}

#[test]
fn test_title_select_popup_integration() {
    let mut popup = TitleSelectPopup::new();
    popup.titles = vec!["Title1".to_string(), "Title2".to_string()];
    popup.selected_index = 1;
    popup.visible = true;

    assert_eq!(popup.titles.len(), 2);
    assert_eq!(popup.selected_index, 1);
    assert!(popup.visible);
}
