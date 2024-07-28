use std::cmp::{max, min};

use crate::{MarkdownRenderer, ORANGE};
use anyhow;
use anyhow::Result;
use copypasta::{ClipboardContext, ClipboardProvider};
use rand::Rng;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use std::collections::HashSet;
use tui_textarea::TextArea;

pub struct ScrollableTextArea {
    pub textareas: Vec<TextArea<'static>>,
    pub titles: Vec<String>,
    pub scroll: usize,
    pub focused_index: usize,
    pub edit_mode: bool,
    pub full_screen_mode: bool,
    pub viewport_height: u16,
    pub start_sel: usize,
    pub markdown_renderer: MarkdownRenderer,
}

impl Default for ScrollableTextArea {
    fn default() -> Self {
        Self::new()
    }
}

impl ScrollableTextArea {
    pub fn new() -> Self {
        ScrollableTextArea {
            textareas: Vec::with_capacity(10),
            titles: Vec::with_capacity(10),
            scroll: 0,
            focused_index: 0,
            edit_mode: false,
            full_screen_mode: false,
            viewport_height: 0,
            start_sel: 0,
            markdown_renderer: MarkdownRenderer::new(),
        }
    }

    pub fn toggle_full_screen(&mut self) {
        self.full_screen_mode = !self.full_screen_mode;
        if self.full_screen_mode {
            self.edit_mode = false;
        }
    }

    pub fn change_title(&mut self, new_title: String) {
        let unique_title = self.generate_unique_title(new_title);
        if self.focused_index < self.titles.len() {
            self.titles[self.focused_index] = unique_title;
        }
    }

    fn generate_unique_title(&self, base_title: String) -> String {
        if !self.titles.contains(&base_title) {
            return base_title;
        }

        let existing_titles: HashSet<String> = self.titles.iter().cloned().collect();
        let mut rng = rand::thread_rng();
        let mut new_title = base_title.clone();
        let mut counter = 1;

        while existing_titles.contains(&new_title) {
            if counter <= 5 {
                new_title = format!("{} {}", base_title, counter);
            } else {
                new_title = format!("{} {}", base_title, rng.gen_range(100..1000));
            }
            counter += 1;
        }

        new_title
    }

    pub fn add_textarea(&mut self, textarea: TextArea<'static>, title: String) {
        let new_index = if self.textareas.is_empty() {
            0
        } else {
            self.focused_index + 1
        };

        let unique_title = self.generate_unique_title(title);
        self.textareas.insert(new_index, textarea);
        self.titles.insert(new_index, unique_title);
        self.focused_index = new_index;
        self.adjust_scroll_to_focused();
    }

    pub fn copy_textarea_contents(&self) -> Result<()> {
        if let Some(textarea) = self.textareas.get(self.focused_index) {
            let content = textarea.lines().join("\n");
            let mut ctx = ClipboardContext::new()
                .map_err(|e| anyhow::anyhow!("Failed to create clipboard context: {}", e))?;
            ctx.set_contents(content)
                .map_err(|e| anyhow::anyhow!("Failed to set clipboard contents: {}", e))?;
        }
        Ok(())
    }

    pub fn jump_to_textarea(&mut self, index: usize) {
        if index < self.textareas.len() {
            self.focused_index = index;
            self.adjust_scroll_to_focused();
        }
    }

    pub fn remove_textarea(&mut self, index: usize) {
        if index < self.textareas.len() {
            self.textareas.remove(index);
            self.titles.remove(index);
            if self.focused_index >= self.textareas.len() {
                self.focused_index = self.textareas.len().saturating_sub(1);
            }
            self.scroll = self.scroll.min(self.focused_index);
        }
    }

    pub fn move_focus(&mut self, direction: isize) {
        let new_index = (self.focused_index as isize + direction).max(0) as usize;
        if new_index < self.textareas.len() {
            self.focused_index = new_index;
            self.adjust_scroll_to_focused();
        }
    }

    pub fn adjust_scroll_to_focused(&mut self) {
        if self.focused_index < self.scroll {
            self.scroll = self.focused_index;
        } else {
            let mut height_sum = 0;
            for i in self.scroll..=self.focused_index {
                let textarea_height = self.textareas[i].lines().len().max(3) as u16 + 2;
                height_sum += textarea_height;

                if height_sum > self.viewport_height {
                    self.scroll = i;
                    break;
                }
            }
        }

        while self.calculate_height_to_focused() > self.viewport_height
            && self.scroll < self.focused_index
        {
            self.scroll += 1;
        }
    }

    pub fn calculate_height_to_focused(&self) -> u16 {
        self.textareas[self.scroll..=self.focused_index]
            .iter()
            .map(|ta| ta.lines().len().max(3) as u16 + 2)
            .sum()
    }

    pub fn initialize_scroll(&mut self) {
        self.scroll = 0;
        self.focused_index = 0;
    }

    pub fn copy_focused_textarea_contents(&self) -> anyhow::Result<()> {
        if let Some(textarea) = self.textareas.get(self.focused_index) {
            let content = textarea.lines().join("\n");
            let mut ctx = ClipboardContext::new().unwrap();
            ctx.set_contents(content).unwrap();
        }
        Ok(())
    }

    pub fn copy_selection_contents(&mut self) -> anyhow::Result<()> {
        if let Some(textarea) = self.textareas.get(self.focused_index) {
            let all_lines = textarea.lines();
            let (cur_row, _) = textarea.cursor();
            let min_row = min(cur_row, self.start_sel);
            let max_row = max(cur_row, self.start_sel);

            if max_row <= all_lines.len() {
                let content = all_lines[min_row..max_row].join("\n");
                let mut ctx = ClipboardContext::new().unwrap();
                ctx.set_contents(content).unwrap();
            }
        }
        // reset selection
        self.start_sel = 0;
        Ok(())
    }

    fn render_full_screen_edit(&mut self, f: &mut Frame, area: Rect) {
        let textarea = &mut self.textareas[self.focused_index];
        let title = &self.titles[self.focused_index];

        let block = Block::default()
            .title(title.clone())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(ORANGE));

        let edit_style = Style::default().fg(Color::White).bg(Color::Black);
        let cursor_style = Style::default().fg(Color::White).bg(ORANGE);

        textarea.set_block(block);
        textarea.set_style(edit_style);
        textarea.set_cursor_style(cursor_style);
        textarea.set_selection_style(Style::default().bg(Color::Red));
        f.render_widget(textarea.widget(), area);
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) -> Result<()> {
        self.viewport_height = area.height;

        if self.full_screen_mode {
            if self.edit_mode {
                self.render_full_screen_edit(f, area);
            } else {
                self.render_full_screen(f, area)?;
            }
        } else {
            let mut remaining_height = area.height;
            let mut visible_textareas = Vec::with_capacity(self.textareas.len());

            for (i, textarea) in self.textareas.iter_mut().enumerate().skip(self.scroll) {
                if remaining_height == 0 {
                    break;
                }

                let content_height = textarea.lines().len() as u16 + 2;
                let is_focused = i == self.focused_index;
                let is_editing = is_focused && self.edit_mode;

                let height = if is_editing {
                    remaining_height
                } else {
                    content_height.min(remaining_height).max(3)
                };

                visible_textareas.push((i, textarea, height));
                remaining_height = remaining_height.saturating_sub(height);

                if is_editing {
                    break;
                }
            }

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    visible_textareas
                        .iter()
                        .map(|(_, _, height)| Constraint::Length(*height))
                        .collect::<Vec<_>>(),
                )
                .split(area);

            for ((i, textarea, _), chunk) in visible_textareas.into_iter().zip(chunks.iter()) {
                let title = &self.titles[i];
                let is_focused = i == self.focused_index;
                let is_editing = is_focused && self.edit_mode;

                let style = if is_focused {
                    if is_editing {
                        Style::default().fg(Color::White).bg(Color::Black)
                    } else {
                        Style::default().fg(Color::Black).bg(Color::DarkGray)
                    }
                } else {
                    Style::default().fg(Color::White).bg(Color::Reset)
                };

                let block = Block::default()
                    .title(title.clone())
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(ORANGE))
                    .style(style);

                if is_editing {
                    textarea.set_block(block);
                    textarea.set_style(style);
                    textarea.set_cursor_style(Style::default().fg(Color::White).bg(ORANGE));
                    f.render_widget(textarea.widget(), *chunk);
                } else {
                    let content = textarea.lines().join("\n");
                    let rendered_markdown = self
                        .markdown_renderer
                        .render_markdown(&content, f.size().width as usize - 2)?;
                    let paragraph = Paragraph::new(rendered_markdown)
                        .block(block)
                        .wrap(Wrap { trim: true });
                    f.render_widget(paragraph, *chunk);
                }
            }
        }
        Ok(())
    }

    fn render_full_screen(&mut self, f: &mut Frame, area: Rect) -> Result<()> {
        let textarea = &mut self.textareas[self.focused_index];
        textarea.set_selection_style(Style::default().bg(Color::Red));
        let title = &self.titles[self.focused_index];

        let block = Block::default()
            .title(title.clone())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(ORANGE));

        let content = textarea.lines().join("\n");
        let rendered_markdown = self
            .markdown_renderer
            .render_markdown(&content, f.size().width as usize - 2)?;

        let paragraph = Paragraph::new(rendered_markdown)
            .block(block)
            .wrap(Wrap { trim: true })
            .scroll((self.scroll as u16, 0));

        f.render_widget(paragraph, area);
        Ok(())
    }

    #[cfg(test)]
    pub fn with_textareas(textareas: Vec<TextArea<'static>>, titles: Vec<String>) -> Self {
        ScrollableTextArea {
            textareas,
            titles,
            scroll: 0,
            focused_index: 0,
            edit_mode: false,
            full_screen_mode: false,
            viewport_height: 0,
            start_sel: 0,
            markdown_renderer: MarkdownRenderer::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_textarea() {
        let mut sta = ScrollableTextArea::new();
        sta.add_textarea(TextArea::default(), "Test".to_string());
        assert_eq!(sta.textareas.len(), 1);
        assert_eq!(sta.titles.len(), 1);
        assert_eq!(sta.focused_index, 0);
    }

    #[test]
    fn test_move_focus() {
        let mut sta = ScrollableTextArea::with_textareas(
            vec![TextArea::default(), TextArea::default()],
            vec!["Test1".to_string(), "Test2".to_string()],
        );
        sta.move_focus(1);
        assert_eq!(sta.focused_index, 1);
        sta.move_focus(-1);
        assert_eq!(sta.focused_index, 0);
    }

    #[test]
    fn test_remove_textarea() {
        let mut sta = ScrollableTextArea::with_textareas(
            vec![TextArea::default(), TextArea::default()],
            vec!["Test1".to_string(), "Test2".to_string()],
        );
        sta.remove_textarea(0);
        assert_eq!(sta.textareas.len(), 1);
        assert_eq!(sta.titles.len(), 1);
        assert_eq!(sta.titles[0], "Test2");
    }

    #[test]
    fn test_change_title() {
        let mut sta =
            ScrollableTextArea::with_textareas(vec![TextArea::default()], vec!["Test".to_string()]);
        sta.change_title("New Title".to_string());
        assert_eq!(sta.titles[0], "New Title");
    }

    #[test]
    fn test_toggle_full_screen() {
        let mut sta = ScrollableTextArea::new();
        assert!(!sta.full_screen_mode);
        sta.toggle_full_screen();
        assert!(sta.full_screen_mode);
        assert!(!sta.edit_mode);
    }

    #[test]
    fn test_copy_textarea_contents() {
        let mut sta = ScrollableTextArea::new();
        let mut textarea = TextArea::default();
        textarea.insert_str("Test content");
        sta.add_textarea(textarea, "Test".to_string());

        let result = sta.copy_textarea_contents();

        match result {
            Ok(_) => println!("Clipboard operation succeeded"),
            Err(e) => {
                let error_message = e.to_string();
                assert!(
                    error_message.contains("clipboard") || error_message.contains("display"),
                    "Unexpected error: {}",
                    error_message
                );
            }
        }
    }

    #[test]
    fn test_jump_to_textarea() {
        let mut sta = ScrollableTextArea::new();
        sta.add_textarea(TextArea::default(), "Test1".to_string());
        sta.add_textarea(TextArea::default(), "Test2".to_string());
        sta.jump_to_textarea(1);
        assert_eq!(sta.focused_index, 1);
    }
}
