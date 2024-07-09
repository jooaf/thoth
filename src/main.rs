use anyhow::Result;
use copypasta::{ClipboardContext, ClipboardProvider};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    fs::File,
    io::{self, BufRead, BufReader, Write},
    path::Path,
};
use thoth::{
    ui::{render_header, render_title_popup, render_title_select_popup},
    ScrollableTextArea, TitlePopup, TitleSelectPopup, SAVE_FILE,
};
use tui_textarea::TextArea;

fn main() -> Result<()> {
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
            let chunks = ratatui::layout::Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints(
                    [
                        ratatui::layout::Constraint::Length(1),
                        ratatui::layout::Constraint::Min(1),
                    ]
                    .as_ref(),
                )
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
