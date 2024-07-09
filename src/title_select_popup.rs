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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_title_select_popup() {
        let popup = TitleSelectPopup::new();
        assert!(popup.titles.is_empty());
        assert_eq!(popup.selected_index, 0);
        assert_eq!(popup.visible, false);
    }
}
