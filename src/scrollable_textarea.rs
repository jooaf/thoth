use crate::{MarkdownRenderer, ORANGE};
use anyhow;
use copypasta::{ClipboardContext, ClipboardProvider};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use tui_textarea::TextArea;

pub struct ScrollableTextArea {
    pub textareas: Vec<TextArea<'static>>,
    pub titles: Vec<String>,
    pub scroll: usize,
    pub focused_index: usize,
    pub edit_mode: bool,
    pub full_screen_mode: bool,
    pub viewport_height: u16,
    markdown_renderer: MarkdownRenderer,
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
            markdown_renderer: MarkdownRenderer::new(),
        }
    }

    pub fn toggle_full_screen(&mut self) {
        self.full_screen_mode = !self.full_screen_mode;
        if self.full_screen_mode {
            self.edit_mode = false;
        }
    }

    pub fn add_textarea(&mut self, textarea: TextArea<'static>, title: String) {
        let new_index = if self.textareas.is_empty() {
            0
        } else {
            self.focused_index + 1
        };

        self.textareas.insert(new_index, textarea);
        self.titles.insert(new_index, title);
        self.focused_index = new_index;
        self.adjust_scroll_to_focused();
    }

    pub fn copy_textarea_contents(&self) -> anyhow::Result<()> {
        if let Some(textarea) = self.textareas.get(self.focused_index) {
            let content = textarea.lines().join("\n");
            let mut ctx = ClipboardContext::new().unwrap();
            ctx.set_contents(content).unwrap();
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

    pub fn change_title(&mut self, new_title: String) {
        if self.focused_index < self.titles.len() {
            self.titles[self.focused_index] = new_title;
        }
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

    fn render_full_screen_edit(&mut self, f: &mut Frame, area: Rect) {
        let textarea = &mut self.textareas[self.focused_index];
        let title = &self.titles[self.focused_index];

        let block = Block::default()
            .title(title.clone())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(ORANGE));

        let edit_style = Style::default().fg(Color::Black).bg(Color::DarkGray);
        let cursor_style = Style::default().fg(Color::White).bg(Color::Black);

        textarea.set_block(block);
        textarea.set_style(edit_style);
        textarea.set_cursor_style(cursor_style);
        f.render_widget(textarea.widget(), area);
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        self.viewport_height = area.height;

        if self.full_screen_mode {
            if self.edit_mode {
                self.render_full_screen_edit(f, area);
            } else {
                self.render_full_screen(f, area);
            }
        } else {
            // Normal mode rendering
            let mut remaining_height = area.height;
            let mut visible_textareas = Vec::with_capacity(self.textareas.len());

            const MAX_HEIGHT: u16 = 10;

            for (i, textarea) in self.textareas.iter_mut().enumerate().skip(self.scroll) {
                if remaining_height == 0 {
                    break;
                }

                let content_height = textarea.lines().len() as u16 + 2;
                let is_focused = i == self.focused_index;
                let is_editing = is_focused && self.edit_mode;

                let height = if is_editing && content_height > MAX_HEIGHT {
                    remaining_height
                } else {
                    content_height.min(remaining_height).min(MAX_HEIGHT)
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
                        Style::default().fg(Color::Black).bg(Color::DarkGray)
                    } else {
                        Style::default().fg(Color::Black).bg(Color::Gray)
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
                    textarea.set_cursor_style(Style::default().fg(Color::White).bg(Color::Black));
                    f.render_widget(textarea.widget(), *chunk);
                } else {
                    let content = textarea.lines().join("\n");
                    let rendered_markdown = self.markdown_renderer.render_markdown(content);
                    let paragraph = Paragraph::new(rendered_markdown)
                        .block(block)
                        .wrap(Wrap { trim: true });
                    f.render_widget(paragraph, *chunk);
                }
            }
        }
    }

    fn render_full_screen(&self, f: &mut Frame, area: Rect) {
        let textarea = &self.textareas[self.focused_index];
        let title = &self.titles[self.focused_index];

        let block = Block::default()
            .title(title.clone())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(ORANGE));

        let content = textarea.lines().join("\n");
        let rendered_markdown = self.markdown_renderer.render_markdown(content);

        let paragraph = Paragraph::new(rendered_markdown)
            .block(block)
            .wrap(Wrap { trim: true })
            .scroll((self.scroll as u16, 0));

        f.render_widget(paragraph, area);
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
}
