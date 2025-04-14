#[derive(Clone)]
pub struct CodeBlockPopup {
    pub code_blocks: Vec<CodeBlock>,
    pub filtered_blocks: Vec<CodeBlock>,
    pub selected_index: usize,
    pub visible: bool,
    pub scroll_offset: usize,
}

#[derive(Clone)]
pub struct CodeBlock {
    pub content: String,
    pub language: String,
    pub start_line: usize,
    pub end_line: usize,
}

impl CodeBlock {
    pub fn new(content: String, language: String, start_line: usize, end_line: usize) -> Self {
        Self {
            content,
            language,
            start_line,
            end_line,
        }
    }
}

impl Default for CodeBlockPopup {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeBlockPopup {
    pub fn new() -> Self {
        CodeBlockPopup {
            code_blocks: Vec::new(),
            filtered_blocks: Vec::new(),
            selected_index: 0,
            visible: false,
            scroll_offset: 0,
        }
    }

    pub fn set_code_blocks(&mut self, code_blocks: Vec<CodeBlock>) {
        self.code_blocks = code_blocks;
        self.filtered_blocks = self.code_blocks.clone();
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    pub fn move_selection_up(&mut self, visible_items: usize) {
        if self.filtered_blocks.is_empty() {
            return;
        }

        if self.selected_index > 0 {
            self.selected_index -= 1;
        } else {
            self.selected_index = self.filtered_blocks.len() - 1;
        }

        if self.selected_index <= self.scroll_offset {
            self.scroll_offset = self.selected_index;
        }
        if self.selected_index == self.filtered_blocks.len() - 1 {
            self.scroll_offset = self.filtered_blocks.len().saturating_sub(visible_items);
        }
    }

    pub fn move_selection_down(&mut self, visible_items: usize) {
        if self.filtered_blocks.is_empty() {
            return;
        }

        if self.selected_index < self.filtered_blocks.len() - 1 {
            self.selected_index += 1;
        } else {
            self.selected_index = 0;
            self.scroll_offset = 0;
        }

        let max_scroll = self.filtered_blocks.len().saturating_sub(visible_items);
        if self.selected_index >= self.scroll_offset + visible_items {
            self.scroll_offset = (self.selected_index + 1).saturating_sub(visible_items);
            if self.scroll_offset > max_scroll {
                self.scroll_offset = max_scroll;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_code_block_popup() {
        let popup = CodeBlockPopup::new();
        assert!(popup.code_blocks.is_empty());
        assert_eq!(popup.selected_index, 0);
        assert!(!popup.visible);
    }

    #[test]
    fn test_set_code_blocks() {
        let mut popup = CodeBlockPopup::new();
        let blocks = vec![
            CodeBlock::new("fn main() {}".to_string(), "rust".to_string(), 0, 2),
            CodeBlock::new("def hello(): pass".to_string(), "python".to_string(), 3, 5),
        ];
        popup.set_code_blocks(blocks);
        assert_eq!(popup.code_blocks.len(), 2);
        assert_eq!(popup.code_blocks[0].language, "rust");
        assert_eq!(popup.code_blocks[1].language, "python");
    }

    #[test]
    fn test_move_selection() {
        let mut popup = CodeBlockPopup::new();
        let blocks = vec![
            CodeBlock::new("Block 1".to_string(), "rust".to_string(), 0, 2),
            CodeBlock::new("Block 2".to_string(), "python".to_string(), 3, 5),
            CodeBlock::new("Block 3".to_string(), "js".to_string(), 6, 8),
        ];
        popup.set_code_blocks(blocks);

        popup.move_selection_down(2);
        assert_eq!(popup.selected_index, 1);

        popup.move_selection_up(2);
        assert_eq!(popup.selected_index, 0);

        popup.move_selection_up(2);
        assert_eq!(popup.selected_index, 2);

        popup.move_selection_down(2);
        assert_eq!(popup.selected_index, 0);
    }
}
