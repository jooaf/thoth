pub struct TitlePopup {
    pub title: String,
    pub visible: bool,
}

impl TitlePopup {
    pub fn new() -> Self {
        TitlePopup {
            title: String::new(),
            visible: false,
        }
    }
}

impl Default for TitlePopup {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_title_popup() {
        let popup = TitlePopup::new();
        assert_eq!(popup.title, "");
        assert!(!popup.visible);
    }

    #[test]
    fn test_title_popup_visibility() {
        let mut popup = TitlePopup::new();
        popup.visible = true;
        assert!(popup.visible);
    }

    #[test]
    fn test_title_popup_set_title() {
        let mut popup = TitlePopup::new();
        popup.title = "New Title".to_string();
        assert_eq!(popup.title, "New Title");
    }
}
