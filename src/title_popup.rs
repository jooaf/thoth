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
}
