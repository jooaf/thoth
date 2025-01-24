use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

pub struct TitleSelectPopup {
    pub titles: Vec<String>,
    pub filtered_titles: Vec<TitleMatch>,
    pub selected_index: usize,
    pub visible: bool,
    pub scroll_offset: usize,
    pub search_query: String,
}

pub struct TitleMatch {
    pub title: String,
    pub index: usize,
    pub score: i64,
}

impl TitleMatch {
    pub fn new(title: String, index: usize, score: i64) -> Self {
        Self {
            title,
            index,
            score,
        }
    }
}

impl Default for TitleSelectPopup {
    fn default() -> Self {
        Self::new()
    }
}

impl TitleSelectPopup {
    pub fn new() -> Self {
        TitleSelectPopup {
            titles: Vec::new(),
            filtered_titles: Vec::new(),
            selected_index: 0,
            visible: false,
            scroll_offset: 0,
            search_query: String::new(),
        }
    }

    pub fn set_titles(&mut self, titles: Vec<String>) {
        self.titles = titles;
        self.filtered_titles = self
            .titles
            .iter()
            .enumerate()
            .map(|(idx, title)| TitleMatch::new(title.clone(), idx, 0))
            .collect();
    }

    pub fn reset_filtered_titles(&mut self) {
        self.filtered_titles = self
            .titles
            .iter()
            .enumerate()
            .map(|(idx, title)| TitleMatch::new(title.clone(), idx, 0))
            .collect();
    }

    pub fn move_selection_up(&mut self, visible_items: usize) {
        if self.filtered_titles.is_empty() {
            return;
        }

        if self.selected_index > 0 {
            self.selected_index -= 1;
        } else {
            self.selected_index = self.filtered_titles.len() - 1;
        }

        if self.selected_index <= self.scroll_offset {
            self.scroll_offset = self.selected_index;
        }
        if self.selected_index == self.filtered_titles.len() - 1 {
            self.scroll_offset = self.filtered_titles.len().saturating_sub(visible_items);
        }
    }

    pub fn move_selection_down(&mut self, visible_items: usize) {
        if self.filtered_titles.is_empty() {
            return;
        }

        if self.selected_index < self.filtered_titles.len() - 1 {
            self.selected_index += 1;
        } else {
            self.selected_index = 0;
            self.scroll_offset = 0;
        }

        let max_scroll = self.filtered_titles.len().saturating_sub(visible_items);
        if self.selected_index >= self.scroll_offset + visible_items {
            self.scroll_offset = (self.selected_index + 1).saturating_sub(visible_items);
            if self.scroll_offset > max_scroll {
                self.scroll_offset = max_scroll;
            }
        }
    }

    pub fn update_search(&mut self) {
        let matcher = SkimMatcherV2::default();

        let mut matched_titles: Vec<TitleMatch> = self
            .titles
            .iter()
            .enumerate()
            .filter_map(|(idx, title)| {
                matcher
                    .fuzzy_match(title, &self.search_query)
                    .map(|score| TitleMatch::new(title.clone(), idx, score))
            })
            .collect();

        matched_titles.sort_by(|a, b| b.score.cmp(&a.score));

        self.filtered_titles = matched_titles;

        if !self.filtered_titles.is_empty() {
            self.selected_index = 0;
            self.scroll_offset = 0;
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
        assert!(!popup.visible);
    }

    #[test]
    fn test_title_select_popup_add_titles() {
        let mut popup = TitleSelectPopup::new();
        let titles = vec!["Title1".to_string(), "Title2".to_string()];
        popup.set_titles(titles);
        assert_eq!(popup.titles.len(), 2);
        assert_eq!(popup.titles[0], "Title1");
        assert_eq!(popup.titles[1], "Title2");
        assert_eq!(popup.filtered_titles.len(), 2);
    }

    #[test]
    fn test_wrap_around_selection() {
        let mut popup = TitleSelectPopup::new();
        popup.set_titles(vec!["1".to_string(), "2".to_string(), "3".to_string()]);

        popup.selected_index = 0;
        popup.move_selection_up(2);
        assert_eq!(popup.selected_index, 2);
        assert_eq!(popup.scroll_offset, 1);

        popup.selected_index = 2;
        popup.move_selection_down(2);
        assert_eq!(popup.selected_index, 0);
        assert_eq!(popup.scroll_offset, 0);
    }

    #[test]
    fn test_search_filtering() {
        let mut popup = TitleSelectPopup::new();
        popup.set_titles(vec![
            "Apple".to_string(),
            "Banana".to_string(),
            "Apricot".to_string(),
        ]);

        popup.search_query = "ap".to_string();
        popup.update_search();
        assert_eq!(popup.filtered_titles.len(), 2);
        assert!(popup.filtered_titles.iter().any(|tm| tm.title == "Apple"));
        assert!(popup.filtered_titles.iter().any(|tm| tm.title == "Apricot"));

        assert!(popup.filtered_titles[0].score >= popup.filtered_titles[1].score);
    }
}
