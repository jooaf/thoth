use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::io;
use tui_scrollview::ScrollView;
use tui_textarea::TextArea;

fn main() -> Result<(), io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut textareas: Vec<TextArea> = vec![];
    let mut focused_index = 0;
    let mut viewport = Rect::new(0, 0, 0, 0);
    let mut locked = false;
    let mut scroll_view = ScrollView::default();

    let mut edit_mode = false;

    loop {
        terminal.draw(|f| {
            let size = f.size();
            viewport = Rect::new(0, 0, size.width, size.height - 1);

            let total_height = textareas.len() as u16 * 5;
            let offset = if total_height > viewport.height {
                (focused_index as u16 * 5).saturating_sub(viewport.height - 5)
            } else {
                0
            };

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    textareas
                        .iter()
                        .map(|_| Constraint::Length(5))
                        .chain(std::iter::once(Constraint::Percentage(100)))
                        .collect::<Vec<_>>(),
                )
                .split(Rect {
                    y: viewport.y + offset,
                    ..viewport
                });
            // terminal.draw(|f| {
            //     let chunks = Layout::default()
            //         .direction(Direction::Vertical)
            //         .margin(1)
            //         .constraints(
            //             textareas
            //                 .iter()
            //                 .map(|_| Constraint::Length(5))
            //                 .chain(std::iter::once(Constraint::Percentage(100)))
            //                 .collect::<Vec<_>>(),
            //         )
            //         .split(f.size());

            // for (i, (textarea, chunk)) in textareas.iter_mut().zip(chunks.iter()).enumerate() {
            //     let mut block = Block::default()
            //         .title(format!("Textarea {}", i + 1))
            //         .borders(Borders::ALL);

            //     if i == focused_index {
            //         block = block.style(Style::default().fg(Color::Yellow).bg(Color::Black));
            //     } else {
            //         block = block.style(Style::default().fg(Color::White).bg(Color::Black));
            //     }

            //     textarea.set_block(block);
            //     let widget = textarea.widget();
            //     f.render_widget(widget, *chunk);
            // }

            for (i, (textarea, chunk)) in textareas.iter_mut().zip(chunks.iter()).enumerate() {
                let mut block = Block::default()
                    .title(format!("Textarea {}", i + 1))
                    .borders(Borders::ALL);

                if i == focused_index {
                    block = block.style(Style::default().fg(Color::Black).bg(Color::Yellow));
                    textarea.set_style(Style::default().fg(Color::White).bg(Color::Yellow));
                    textarea
                        .set_cursor_line_style(Style::default().fg(Color::Black).bg(Color::Yellow));
                    textarea.set_cursor_style(Style::default().fg(Color::Black).bg(Color::White));
                } else {
                    block = block.style(Style::default().fg(Color::Gray).bg(Color::Black));
                    textarea.set_style(Style::default().fg(Color::Gray).bg(Color::Black));
                }

                if i != focused_index {
                    textarea.set_cursor_style(Style::default().fg(Color::Gray).bg(Color::Black));
                }

                // if i != focused_index {
                //     textarea.hide_cursor();
                // }

                textarea.set_block(block);
                let widget = textarea.widget();
                f.render_widget(widget, *chunk);
            }

            if let Some(textarea) = textareas.get_mut(focused_index) {
                textarea.set_cursor_line_style(Style::default().fg(Color::Black).bg(Color::Yellow));
                textarea.set_cursor_style(
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::White)
                        .add_modifier(Modifier::SLOW_BLINK),
                );
            }

            let help_text = Paragraph::new("Press 'a' to add a textarea, 'q' to quit.")
                .style(Style::default().fg(Color::LightCyan));
            f.render_widget(help_text, chunks[chunks.len() - 1]);
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('a') => {
                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                        if !edit_mode {
                            let new_textarea = TextArea::default();
                            textareas.push(new_textarea);
                        }
                    }
                }
                KeyCode::Char('b') => {
                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                        break;
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
                        if focused_index < viewport.y as usize / 5 {
                            viewport = Rect {
                                y: viewport.y.saturating_sub(5),
                                ..viewport
                            };
                        }
                    }
                }
                KeyCode::Down => {
                    if edit_mode {
                        textareas[focused_index].move_cursor(tui_textarea::CursorMove::Down);
                    } else if focused_index < textareas.len() - 1 {
                        focused_index += 1;
                        if focused_index >= (viewport.y + viewport.height) as usize / 5 {
                            viewport = Rect {
                                y: (viewport.y + 5)
                                    .min(textareas.len() as u16 * 5 - viewport.height),
                                ..viewport
                            };
                        }
                    }
                }
                _ => {
                    if edit_mode {
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
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
