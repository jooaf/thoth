use crossterm::{
    event::{
        self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event, KeyCode, KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Position, Rect, Size},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget},
    Frame, Terminal,
};
use ratatui::{prelude::*, style::palette::tailwind, widgets::*};
use std::io;
use tui_scrollview::{ScrollView, ScrollViewState};
use tui_textarea::TextArea;

struct TitlePopup {
    visible: bool,
    title: String,
}

impl Default for TitlePopup {
    fn default() -> Self {
        TitlePopup {
            visible: false,
            title: String::new(),
        }
    }
}

fn main() -> Result<(), io::Error> {
    enable_raw_mode()?;
    let mut title_popup = TitlePopup::default();
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        EnableBracketedPaste
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut textareas: Vec<TextArea> = vec![];
    let mut focused_index = 0;
    let mut edit_mode = false;
    let mut scroll_view_state = ScrollViewState::default();

    loop {
        terminal.draw(|f| {
            let palette = tailwind::ORANGE;
            let fg = palette.c900;
            let bg = palette.c300;
            let keys_fg = palette.c50;
            let keys_bg = palette.c600;
            let header = Line::from(vec![
                "Thoth ".into(),
                "  ↑ | ↓ | Add: Ctrl+a | Delete: Ctrl+d | Quit: Ctrl+e | Edit: Enter "
                    .fg(keys_fg)
                    .bg(keys_bg),
            ])
            .style((fg, bg));

            let size = f.size();
            let header_height = 1;
            let viewport = Rect::new(
                0,
                header_height,
                size.width,
                size.height - header_height - 1,
            );

            if title_popup.visible {
                let popup_width = 30;
                let popup_height = 3;
                let popup_x = (size.width - popup_width) / 2;
                let popup_y = (size.height - popup_height) / 2;
                let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

                let popup = Block::default().title("Edit Title").borders(Borders::ALL);
                f.render_widget(popup, popup_area);

                let input_area = popup_area.inner(&Margin::new(1, 1));
                let input = Paragraph::new(title_popup.title.clone())
                    .style(Style::default().fg(Color::White))
                    .block(Block::default());
                f.render_widget(input, input_area);
            }

            f.render_widget(header, Rect::new(0, 0, size.width, header_height));

            let content_height = textareas
                .iter()
                .map(|textarea| textarea.lines().len() as u16 + 2)
                .sum();
            let scroll_view_size = Size::new(viewport.width, content_height);

            let mut scroll_view = ScrollView::new(scroll_view_size);
            render_textareas(
                &mut textareas,
                &mut scroll_view.buf_mut(),
                focused_index,
                edit_mode,
            );

            f.render_stateful_widget(scroll_view, viewport, &mut scroll_view_state);

            let footer = Line::from(vec!["Normal: Esc | Change Title: Ctrl+c  "
                .fg(keys_fg)
                .bg(keys_bg)])
            .style((fg, keys_bg));
            f.render_widget(footer, Rect::new(0, size.height - 1, size.width, 1));
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('a') => {
                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                        if !edit_mode {
                            let new_textarea = TextArea::default();
                            textareas.push(new_textarea);
                        }
                    } else {
                        textareas[focused_index].input(key);
                    }
                }
                KeyCode::Char('d') => {
                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                        if !edit_mode {
                            textareas.pop();
                        }
                    } else {
                        textareas[focused_index].input(key);
                    }
                }
                KeyCode::Char('f') => {
                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                        title_popup.visible = !title_popup.visible;
                        if !title_popup.visible {
                            title_popup.title.clear();
                        }
                    } else {
                        textareas[focused_index].input(key);
                    }
                }
                KeyCode::Char('e') => {
                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                        break;
                    } else {
                        textareas[focused_index].input(key);
                    }
                }
                KeyCode::Enter => {
                    if !edit_mode {
                        edit_mode = true;
                    } else {
                        textareas[focused_index].insert_newline();
                    }
                }
                KeyCode::Esc => {
                    edit_mode = false;
                }
                KeyCode::Up => {
                    if edit_mode {
                        textareas[focused_index].move_cursor(tui_textarea::CursorMove::Up);
                    } else if focused_index > 0 {
                        focused_index -= 1;
                        let lines_up: u16 = textareas[..focused_index]
                            .iter()
                            .map(|area| area.lines().len() as u16 + 2)
                            .sum();
                        let offset = scroll_view_state.offset();

                        scroll_view_state.set_offset(Position::new(offset.x, lines_up))
                    }
                }
                KeyCode::Down => {
                    if textareas.len() != 0 {
                        if edit_mode {
                            textareas[focused_index].move_cursor(tui_textarea::CursorMove::Down);
                        } else if focused_index < textareas.len() - 1 {
                            focused_index += 1;

                            let lines_up: u16 = textareas[..focused_index]
                                .iter()
                                .map(|area| area.lines().len() as u16 + 2)
                                .sum();
                            let offset = scroll_view_state.offset();

                            scroll_view_state.set_offset(Position::new(offset.x, lines_up))
                        }
                    }
                }
                _ => {
                    if title_popup.visible {
                        match key.code {
                            KeyCode::Enter => {
                                if let Some(textarea) = textareas.get_mut(focused_index) {
                                    textarea.set_block(
                                        Block::default()
                                            .title(title_popup.title.clone())
                                            .borders(Borders::ALL),
                                    );
                                }
                                title_popup.visible = false;
                                title_popup.title.clear();
                            }
                            KeyCode::Char(c) => {
                                title_popup.title.push(c);
                            }
                            KeyCode::Backspace => {
                                title_popup.title.pop();
                            }
                            KeyCode::Esc => {
                                title_popup.visible = false;
                                title_popup.title.clear();
                            }
                            _ => {}
                        }
                    } else if edit_mode {
                        if let Some(textarea) = textareas.get_mut(focused_index) {
                            textarea.input(key);
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
        DisableMouseCapture,
        DisableBracketedPaste
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn render_textareas(
    textareas: &mut [TextArea],
    buf: &mut ratatui::buffer::Buffer,
    focused_index: usize,
    edit_mode: bool,
) {
    let area = buf.area;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            textareas
                .iter()
                .map(|textarea| Constraint::Length(textarea.lines().len() as u16 + 2))
                .collect::<Vec<_>>(),
        )
        .split(area);

    for (i, (textarea, chunk)) in textareas.iter_mut().zip(chunks.iter()).enumerate() {
        let mut block = Block::default()
            .title(format!("Textarea {}", i + 1))
            .borders(Borders::ALL);

        if i == focused_index {
            block = block.style(Style::default().fg(Color::Black).bg(Color::Yellow));
            textarea.set_style(Style::default().fg(Color::Black).bg(Color::Yellow));
            textarea.set_cursor_line_style(Style::default().fg(Color::Black).bg(Color::Yellow));
            textarea.set_cursor_style(Style::default().fg(Color::Black).bg(Color::White));
        } else {
            block = block.style(Style::default().fg(Color::Gray).bg(Color::Black));
            textarea.set_style(Style::default().fg(Color::Gray).bg(Color::Black));
            textarea.set_cursor_style(Style::default().fg(Color::Gray).bg(Color::Black));
        }

        textarea.set_block(block);
        let chunk = chunk.intersection(Rect {
            width: area.width,
            ..*chunk
        });
        let widget = textarea.widget();
        Widget::render(widget, chunk, buf);
    }

    if let Some(textarea) = textareas.get_mut(focused_index) {
        if edit_mode {
            textarea.set_cursor_line_style(Style::default().fg(Color::Black).bg(Color::Yellow));
            textarea.set_cursor_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::SLOW_BLINK),
            );
        }
    }
}
