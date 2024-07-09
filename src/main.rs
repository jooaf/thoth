use anyhow;
use copypasta::{ClipboardContext, ClipboardProvider};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs, Widget},
    Frame, Terminal,
};
use std::path::Path;
use std::{
    fs::File,
    io::{self, BufRead, BufReader, Write},
};
use tui_textarea::TextArea;

const SAVE_FILE: &str = "thoth_notes.md";
const ORANGE: Color = Color::Rgb(255, 165, 0);

struct ScrollableTextArea {
    textareas: Vec<TextArea<'static>>,
    titles: Vec<String>,
    scroll: usize,
    focused_index: usize,
    edit_mode: bool,
    viewport_height: u16,
}

struct TitlePopup {
    title: String,
    visible: bool,
}

struct TitleSelectPopup {
    titles: Vec<String>,
    selected_index: usize,
    visible: bool,
}

impl TitleSelectPopup {
    fn new() -> Self {
        TitleSelectPopup {
            titles: Vec::new(),
            selected_index: 0,
            visible: false,
        }
    }
}

impl TitlePopup {
    fn new() -> Self {
        TitlePopup {
            title: String::new(),
            visible: false,
        }
    }
}

impl ScrollableTextArea {
    fn new() -> Self {
        ScrollableTextArea {
            textareas: Vec::with_capacity(10), // Pre-allocate space for 10 textareas
            titles: Vec::with_capacity(10),
            scroll: 0,
            focused_index: 0,
            edit_mode: false,
            viewport_height: 0,
        }
    }

    fn add_textarea(&mut self, textarea: TextArea<'static>, title: String) {
        self.textareas.push(textarea);
        self.titles.push(title);
        self.focused_index = self.textareas.len() - 1;
        self.adjust_scroll_to_focused();
    }

    fn copy_textarea_contents(&self) -> anyhow::Result<()> {
        if let Some(textarea) = self.textareas.get(self.focused_index) {
            let content = textarea.lines().join("\n");
            let mut ctx = ClipboardContext::new().unwrap();
            ctx.set_contents(content).unwrap();
        }
        Ok(())
    }

    fn jump_to_textarea(&mut self, index: usize) {
        if index < self.textareas.len() {
            self.focused_index = index;
            self.adjust_scroll_to_focused();
        }
    }

    fn remove_textarea(&mut self, index: usize) {
        if index < self.textareas.len() {
            self.textareas.remove(index);
            self.titles.remove(index);
            if self.focused_index >= self.textareas.len() {
                self.focused_index = self.textareas.len().saturating_sub(1);
            }
            self.scroll = self.scroll.min(self.focused_index);
        }
    }

    fn move_focus(&mut self, direction: isize) {
        let new_index = (self.focused_index as isize + direction).max(0) as usize;
        if new_index < self.textareas.len() {
            self.focused_index = new_index;
            self.adjust_scroll_to_focused();
        }
    }

    fn adjust_scroll_to_focused(&mut self) {
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

    fn calculate_height_to_focused(&self) -> u16 {
        self.textareas[self.scroll..=self.focused_index]
            .iter()
            .map(|ta| ta.lines().len().max(3) as u16 + 2)
            .sum()
    }

    fn change_title(&mut self, new_title: String) {
        if self.focused_index < self.titles.len() {
            self.titles[self.focused_index] = new_title;
        }
    }

    fn initialize_scroll(&mut self) {
        self.scroll = 0;
        self.focused_index = 0;
    }

    fn render(&mut self, f: &mut Frame, area: Rect) {
        self.viewport_height = area.height;
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

        for ((i, textarea, height), chunk) in visible_textareas.into_iter().zip(chunks.iter()) {
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

            let content_height = textarea.lines().len() as u16;
            let visible_lines = height.saturating_sub(2);

            if content_height > visible_lines && !is_editing {
                let truncated_content: String = textarea
                    .lines()
                    .iter()
                    .take(visible_lines as usize)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n");

                let truncated_text = format!("{}\n...", truncated_content);
                let truncated_paragraph = Paragraph::new(truncated_text).block(block);
                f.render_widget(truncated_paragraph, *chunk);
            } else {
                textarea.set_block(block);
                textarea.set_style(style);
                if is_editing {
                    textarea.set_cursor_style(Style::default().fg(Color::White).bg(Color::Black));
                } else {
                    textarea.set_cursor_style(style);
                }
                f.render_widget(textarea.widget(), *chunk);
            }
        }
    }
}

fn render_title_select_popup(f: &mut Frame, popup: &TitleSelectPopup) {
    let area = centered_rect(60, 60, f.size());
    f.render_widget(ratatui::widgets::Clear, area);

    let items: Vec<Line> = popup
        .titles
        .iter()
        .enumerate()
        .map(|(i, title)| {
            if i == popup.selected_index {
                Line::from(vec![Span::styled(
                    format!("‚ñ∂ {}", title),
                    Style::default().fg(Color::Yellow),
                )])
            } else {
                Line::from(vec![Span::raw(format!("  {}", title))])
            }
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ORANGE))
        .title("Select Title");

    let paragraph = Paragraph::new(items)
        .block(block)
        .wrap(ratatui::widgets::Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn main() -> Result<(), io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut scrollable_textarea = ScrollableTextArea::new();
    let mut title_popup = TitlePopup::new();

    if Path::new(SAVE_FILE).exists() {
        let (loaded_textareas, loaded_titles) = load_textareas()?;
        for (textarea, title) in loaded_textareas.into_iter().zip(loaded_titles) {
            scrollable_textarea.add_textarea(textarea, title);
        }
    } else {
        scrollable_textarea.add_textarea(TextArea::default(), String::from("New Textarea"));
    }

    scrollable_textarea.initialize_scroll();
    let mut title_select_popup = TitleSelectPopup::new();

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Min(1)].as_ref())
                .split(f.size());

            render_header(f, chunks[0]);
            scrollable_textarea.render(f, chunks[1]);

            if title_popup.visible {
                render_title_popup(f, &title_popup);
            } else if title_select_popup.visible {
                render_title_select_popup(f, &title_select_popup);
            }
        })?;

        if let Event::Key(key) = event::read()? {
            if title_popup.visible {
                match key.code {
                    KeyCode::Enter => {
                        scrollable_textarea.change_title(title_popup.title.clone());
                        title_popup.visible = false;
                        title_popup.title.clear();
                    }
                    KeyCode::Esc => {
                        title_popup.visible = false;
                        title_popup.title.clear();
                    }
                    KeyCode::Char(c) => {
                        title_popup.title.push(c);
                    }
                    KeyCode::Backspace => {
                        title_popup.title.pop();
                    }
                    _ => {}
                }
            } else if title_select_popup.visible {
                match key.code {
                    KeyCode::Enter => {
                        scrollable_textarea.jump_to_textarea(title_select_popup.selected_index);
                        title_select_popup.visible = false;
                    }
                    KeyCode::Esc => {
                        title_select_popup.visible = false;
                    }
                    KeyCode::Up => {
                        if title_select_popup.selected_index > 0 {
                            title_select_popup.selected_index -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if title_select_popup.selected_index < title_select_popup.titles.len() - 1 {
                            title_select_popup.selected_index += 1;
                        }
                    }
                    _ => {}
                }
            } else {
                match key.code {
                    KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if let Err(e) = scrollable_textarea.copy_textarea_contents() {
                            eprintln!("Failed to copy to clipboard: {}", e);
                        }
                    }
                    KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        title_select_popup.titles = scrollable_textarea.titles.clone();
                        title_select_popup.selected_index = 0;
                        title_select_popup.visible = true;
                    }
                    KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        save_textareas(
                            &scrollable_textarea.textareas,
                            &scrollable_textarea.titles,
                        )?;
                        break;
                    }
                    KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        let new_textarea = TextArea::default();
                        scrollable_textarea
                            .add_textarea(new_textarea, String::from("New Textarea"));
                        scrollable_textarea.focused_index = scrollable_textarea.textareas.len() - 1;
                        scrollable_textarea.adjust_scroll_to_focused();
                    }
                    KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if scrollable_textarea.textareas.len() > 1 {
                            scrollable_textarea.remove_textarea(scrollable_textarea.focused_index);
                        }
                    }
                    KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        title_popup.visible = true;
                        title_popup.title =
                            scrollable_textarea.titles[scrollable_textarea.focused_index].clone();
                    }
                    KeyCode::Enter => {
                        if scrollable_textarea.edit_mode {
                            scrollable_textarea.textareas[scrollable_textarea.focused_index]
                                .insert_newline();
                        } else {
                            scrollable_textarea.edit_mode = true;
                        }
                    }
                    KeyCode::Esc => {
                        scrollable_textarea.edit_mode = false;
                    }
                    KeyCode::Up => {
                        if scrollable_textarea.edit_mode {
                            scrollable_textarea.textareas[scrollable_textarea.focused_index]
                                .move_cursor(tui_textarea::CursorMove::Up);
                        } else {
                            scrollable_textarea.move_focus(-1);
                        }
                    }
                    KeyCode::Down => {
                        if scrollable_textarea.edit_mode {
                            scrollable_textarea.textareas[scrollable_textarea.focused_index]
                                .move_cursor(tui_textarea::CursorMove::Down);
                        } else {
                            scrollable_textarea.move_focus(1);
                        }
                    }
                    _ => {
                        if scrollable_textarea.edit_mode {
                            scrollable_textarea.textareas[scrollable_textarea.focused_index]
                                .input(key);
                        }
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    Ok(())
}

fn render_header(f: &mut Frame, area: Rect) {
    let commands = "Quit: Ctrl+q | Add: Ctrl+n | Delete: Ctrl+d | Edit mode: Enter | Exit edit: Esc | Change Title: Ctrl+t | Copy Block: Ctrl+y | Select Block: Ctrl+j";
    let thoth = "Thoth";
    let total_length = commands.len() + thoth.len() + 1;
    let remaining_space = area.width as usize - total_length;

    let header = Line::from(vec![
        Span::styled(commands, Style::default().fg(ORANGE)),
        Span::styled(" ".repeat(remaining_space), Style::default().fg(ORANGE)),
        Span::styled(thoth, Style::default().fg(ORANGE)),
    ]);

    let tabs = Tabs::new(vec![header])
        .style(Style::default().bg(Color::Black))
        .divider(Span::styled("|", Style::default().fg(ORANGE)));

    f.render_widget(tabs, area);
}

fn render_title_popup(f: &mut Frame, popup: &TitlePopup) {
    let area = centered_rect(20, 20, f.size());
    f.render_widget(ratatui::widgets::Clear, area);

    let text = Paragraph::new(popup.title.as_str())
        .style(Style::default().bg(Color::DarkGray))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ORANGE))
                .title("Change Title"),
        );
    f.render_widget(text, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}

fn save_textareas(textareas: &[TextArea], titles: &[String]) -> io::Result<()> {
    let mut file = File::create(SAVE_FILE)?;
    for (textarea, title) in textareas.iter().zip(titles.iter()) {
        writeln!(file, "# {}", title)?;
        for line in textarea.lines() {
            writeln!(file, "{}", line)?;
        }
        writeln!(file)?;
    }
    Ok(())
}

fn load_textareas() -> io::Result<(Vec<TextArea<'static>>, Vec<String>)> {
    let file = File::open(SAVE_FILE)?;
    let reader = BufReader::new(file);
    let mut textareas = Vec::with_capacity(10);
    let mut titles = Vec::with_capacity(10);
    let mut current_textarea = TextArea::default();
    let mut current_title = String::new();

    for line in reader.lines() {
        let line = line?;
        if line.starts_with("# ") {
            if !current_title.is_empty() {
                textareas.push(current_textarea);
                titles.push(current_title);
                current_textarea = TextArea::default();
            }
            current_title = line[2..].to_string();
        } else {
            current_textarea.insert_str(&line);
            current_textarea.insert_newline();
        }
    }

    if !current_title.is_empty() {
        textareas.push(current_textarea);
        titles.push(current_title);
    }

    Ok((textareas, titles))
}
