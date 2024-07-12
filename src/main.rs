use anyhow::Result;
use clap::{Parser, Subcommand};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    fs::File,
    io::{self, BufRead, BufReader, Write},
};
use thoth::{
    get_save_file_path,
    ui::{render_header, render_title_popup, render_title_select_popup},
    ScrollableTextArea, TitlePopup, TitleSelectPopup,
};
use tui_textarea::TextArea;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Add { name: String, content: String },
    List,
    Delete { name: String },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Add { name, content }) => {
            add_block(name, content)?;
        }
        Some(Commands::List) => {
            list_blocks()?;
        }
        Some(Commands::Delete { name }) => {
            delete_block(name)?;
        }
        None => {
            run_ui()?;
        }
    }

    Ok(())
}

fn add_block(name: &str, content: &str) -> Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(get_save_file_path())?;

    writeln!(file, "# {}", name)?;
    writeln!(file, "{}", content)?;
    writeln!(file)?;

    println!("Block '{}' added successfully.", name);
    Ok(())
}

fn list_blocks() -> Result<()> {
    let file = File::open(get_save_file_path())?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line?;

        if let Some(strip) = line.strip_prefix("# ") {
            println!("{}", strip);
        }
    }

    Ok(())
}

fn delete_block(name: &str) -> Result<()> {
    let file = File::open(get_save_file_path())?;
    let reader = BufReader::new(file);
    let mut blocks = Vec::new();
    let mut current_block = Vec::new();
    let mut current_name = String::new();

    for line in reader.lines() {
        let line = line?;
        if let Some(strip) = line.strip_prefix("# ") {
            if !current_name.is_empty() {
                blocks.push((current_name, current_block));
                current_block = Vec::new();
            }
            current_name = strip.to_string();
        } else {
            current_block.push(line);
        }
    }

    if !current_name.is_empty() {
        blocks.push((current_name, current_block));
    }

    let mut file = File::create(get_save_file_path())?;
    let mut deleted = false;

    for (block_name, block_content) in blocks {
        if block_name != name {
            writeln!(file, "# {}", block_name)?;
            for line in block_content {
                writeln!(file, "{}", line)?;
            }
            writeln!(file)?;
        } else {
            deleted = true;
        }
    }

    if deleted {
        println!("Block '{}' deleted successfully.", name);
    } else {
        println!("Block '{}' not found.", name);
    }

    Ok(())
}

fn run_ui() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut scrollable_textarea = ScrollableTextArea::new();
    let mut title_popup = TitlePopup::new();
    let mut title_select_popup = TitleSelectPopup::new();

    if get_save_file_path().exists() {
        let (loaded_textareas, loaded_titles) = load_textareas()?;
        for (textarea, title) in loaded_textareas.into_iter().zip(loaded_titles) {
            scrollable_textarea.add_textarea(textarea, title);
        }
    } else {
        scrollable_textarea.add_textarea(TextArea::default(), String::from("New Textarea"));
    }

    scrollable_textarea.initialize_scroll();

    loop {
        terminal.draw(|f| {
            let size = f.size();
            let chunks = ratatui::layout::Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints(
                    [
                        ratatui::layout::Constraint::Length(1),
                        ratatui::layout::Constraint::Min(1),
                    ]
                    .as_ref(),
                )
                .split(size);

            render_header(f, chunks[0]);

            if scrollable_textarea.full_screen_mode {
                scrollable_textarea.render(f, size);
            } else {
                scrollable_textarea.render(f, chunks[1]);
            }

            if title_popup.visible {
                render_title_popup(f, &title_popup);
            } else if title_select_popup.visible {
                render_title_select_popup(f, &title_select_popup);
            }
        })?;

        if let Event::Key(key) = event::read()? {
            if scrollable_textarea.full_screen_mode {
                match key.code {
                    KeyCode::Esc => {
                        if scrollable_textarea.edit_mode {
                            scrollable_textarea.edit_mode = false;
                        } else {
                            scrollable_textarea.toggle_full_screen();
                        }
                    }
                    KeyCode::Enter => {
                        if !scrollable_textarea.edit_mode {
                            scrollable_textarea.edit_mode = true;
                        } else {
                            scrollable_textarea.textareas[scrollable_textarea.focused_index]
                                .insert_newline();
                        }
                    }
                    KeyCode::Up => {
                        if scrollable_textarea.edit_mode {
                            scrollable_textarea.textareas[scrollable_textarea.focused_index]
                                .move_cursor(tui_textarea::CursorMove::Up);
                        } else {
                            scrollable_textarea.scroll =
                                scrollable_textarea.scroll.saturating_sub(1);
                        }
                    }
                    KeyCode::Down => {
                        if scrollable_textarea.edit_mode {
                            scrollable_textarea.textareas[scrollable_textarea.focused_index]
                                .move_cursor(tui_textarea::CursorMove::Down);
                        } else {
                            scrollable_textarea.scroll += 1;
                        }
                    }
                    KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if let Err(e) = scrollable_textarea.copy_focused_textarea_contents() {
                            eprintln!("Failed to copy to clipboard: {}", e);
                        }
                    }
                    _ => {
                        if scrollable_textarea.edit_mode {
                            scrollable_textarea.textareas[scrollable_textarea.focused_index]
                                .input(key);
                        }
                    }
                }
            } else if title_popup.visible {
                match key.code {
                    KeyCode::Enter => {
                        title_select_popup
                            .titles
                            .clone_from(&scrollable_textarea.titles);
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
                        if let Err(e) = scrollable_textarea.copy_focused_textarea_contents() {
                            eprintln!("Failed to copy to clipboard: {}", e);
                        }
                    }
                    KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        scrollable_textarea.toggle_full_screen();
                    }
                    KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        title_select_popup
                            .titles
                            .clone_from(&scrollable_textarea.titles);
                        title_select_popup.selected_index = 0;
                        title_select_popup.visible = true;
                    }
                    KeyCode::Char('q') => {
                        save_textareas(
                            &scrollable_textarea.textareas,
                            &scrollable_textarea.titles,
                        )?;
                        break;
                    }
                    KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        scrollable_textarea
                            .add_textarea(TextArea::default(), String::from("New Textarea"));
                        scrollable_textarea.adjust_scroll_to_focused();
                    }
                    KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if scrollable_textarea.textareas.len() > 1 {
                            scrollable_textarea.remove_textarea(scrollable_textarea.focused_index);
                        }
                    }
                    KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        title_popup.visible = true;
                        title_popup.title.clone_from(
                            &scrollable_textarea.titles[scrollable_textarea.focused_index],
                        );
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
        } else if let Event::Resize(_, _) = event::read()? {
            // Terminal was resized, redraw the UI
            terminal.clear()?;
        }
    }

    // Cleanup
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn save_textareas(textareas: &[TextArea], titles: &[String]) -> io::Result<()> {
    let mut file = File::create(get_save_file_path())?;
    for (textarea, title) in textareas.iter().zip(titles.iter()) {
        writeln!(file, "# {}", title)?;
        let mut in_code_block = false;
        for line in textarea.lines() {
            if line.trim().starts_with("```") {
                in_code_block = !in_code_block;
            }
            if in_code_block || !line.starts_with('#') {
                writeln!(file, "{}", line)?;
            } else {
                writeln!(file, "\\{}", line)?;
            }
        }
        writeln!(file)?;
    }
    Ok(())
}

fn load_textareas() -> io::Result<(Vec<TextArea<'static>>, Vec<String>)> {
    let file = File::open(get_save_file_path())?;
    let reader = BufReader::new(file);
    let mut textareas = Vec::with_capacity(10);
    let mut titles = Vec::with_capacity(10);
    let mut current_textarea = TextArea::default();
    let mut current_title = String::new();
    let mut in_code_block = false;
    let mut is_first_line = true;

    for line in reader.lines() {
        let line = line?;
        if !in_code_block && line.starts_with("# ") && is_first_line {
            current_title = line[2..].to_string();
            is_first_line = false;
        } else {
            if line.trim().starts_with("```") {
                in_code_block = !in_code_block;
            }
            if in_code_block {
                current_textarea.insert_str(&line);
            } else if let Some(strip) = line.strip_prefix('\\') {
                current_textarea.insert_str(strip);
            } else if line.starts_with("# ") && !is_first_line {
                if !current_title.is_empty() {
                    textareas.push(current_textarea);
                    titles.push(current_title);
                }
                current_textarea = TextArea::default();
                current_title = line[2..].to_string();
                is_first_line = true;
                continue;
            } else {
                current_textarea.insert_str(&line);
            }
            current_textarea.insert_newline();
            is_first_line = false;
        }
    }

    if !current_title.is_empty() {
        textareas.push(current_textarea);
        titles.push(current_title);
    }

    Ok((textareas, titles))
}
