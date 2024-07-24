pub struct TitleSelectPopup {
    pub titles: Vec<String>,
    pub selected_index: usize,
    pub visible: bool,
}

impl TitleSelectPopup {
    pub fn new() -> Self {
        TitleSelectPopup {
            titles: Vec::new(),
            selected_index: 0,
            visible: false,
        }
    }
}

impl Default for TitleSelectPopup {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_title_select_popup() {
        let popup = TitleSelectPopup::new();
        assert!(popup.titles.is_empty());
        assert_eq!(popup.selected_index, 0);
        assert!(!popup.visible);
    }

    #[test]
    fn test_title_select_popup_add_titles() {
        let mut popup = TitleSelectPopup::new();
        popup.titles = vec!["Title1".to_string(), "Title2".to_string()];
        assert_eq!(popup.titles.len(), 2);
        assert_eq!(popup.titles[0], "Title1");
        assert_eq!(popup.titles[1], "Title2");
    }
}
