pub struct TitleSelectPopup {
    pub titles: Vec<String>,
    pub selected_index: usize,
    pub visible: bool,
    pub scroll_offset: usize,
}

impl TitleSelectPopup {
    pub fn new() -> Self {
        TitleSelectPopup {
            titles: Vec::new(),
            selected_index: 0,
            visible: false,
            scroll_offset: 0,
        }
    }

    pub fn move_selection_up(&mut self, visible_items: usize) {
        if self.titles.is_empty() {
            return;
        }

        if self.selected_index > 0 {
            self.selected_index -= 1;
        } else {
            self.selected_index = self.titles.len() - 1;
        }

        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        }
        if self.selected_index == self.titles.len() - 1 {
            self.scroll_offset = self.titles.len().saturating_sub(visible_items);
        }
    }

    pub fn move_selection_down(&mut self, visible_items: usize) {
        if self.titles.is_empty() {
            return;
        }

        if self.selected_index < self.titles.len() - 1 {
            self.selected_index += 1;
        } else {
            self.selected_index = 0;
            self.scroll_offset = 0;
        }

        let max_scroll = self.titles.len().saturating_sub(visible_items);
        if self.selected_index >= self.scroll_offset + visible_items {
            self.scroll_offset = (self.selected_index + 1).saturating_sub(visible_items);
            if self.scroll_offset > max_scroll {
                self.scroll_offset = max_scroll;
            }
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

    #[test]
    fn test_wrap_around_selection() {
        let mut popup = TitleSelectPopup::new();
        popup.titles = vec!["1".to_string(), "2".to_string(), "3".to_string()];

        popup.selected_index = 0;
        popup.move_selection_up(2);
        assert_eq!(popup.selected_index, 2);
        assert_eq!(popup.scroll_offset, 1);

        popup.selected_index = 2;
        popup.move_selection_down(2);
        assert_eq!(popup.selected_index, 0);
        assert_eq!(popup.scroll_offset, 0);
    }
}
