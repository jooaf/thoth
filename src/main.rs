use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use copypasta::{ClipboardContext, ClipboardProvider};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    fs::File,
    io::{self, BufRead, BufReader, Read, Write},
};
use thoth_cli::{
    format_json, format_markdown, get_save_file_path,
    ui::{
        render_edit_commands_popup, render_header, render_title_popup, render_title_select_popup,
        EditCommandsPopup,
    },
    ScrollableTextArea, TitlePopup, TitleSelectPopup,
};
use tui_textarea::TextArea;

use std::env;
use std::fs;
use std::process::Command;
use tempfile::NamedTempFile;

#[derive(Parser)]
#[command(author = env!("CARGO_PKG_AUTHORS"), version = env!("CARGO_PKG_VERSION"), about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new block to the scratchpad
    Add {
        /// Name of the block to be added
        name: String,
        /// Contents to be associated with the named block
        content: Option<String>,
    },
    /// List all of the blocks within your thoth scratchpad
    List,
    /// Delete a block by name
    Delete {
        /// The name of the block to be deleted
        name: String,
    },
    /// View (STDOUT) the contents of the block by name
    View {
        /// The name of the block to be used
        name: String,
    },
    /// Copy the contents of a block to the system clipboard
    Copy {
        /// The name of the block to be used
        name: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Add { name, content }) => {
            let content = match content {
                Some(c) => c.to_string(),
                None => {
                    let mut buffer = String::new();
                    if atty::is(atty::Stream::Stdin) {
                        bail!(format!("Couldn't create '{}' because nothing was passed in. Either pipe in contents or use `thoth add {} <contents>`", name, name));
                    }
                    io::stdin().read_to_string(&mut buffer)?;
                    if buffer.trim().is_empty() {
                        bail!(format!("Couldn't create '{}' because nothing was passed in. Either pipe in contents or use `thoth add {} <contents>`", name, name));
                    }
                    buffer
                }
            };
            add_block(name, &content)?;
        }
        Some(Commands::List) => {
            list_blocks()?;
        }
        Some(Commands::Delete { name }) => {
            delete_block(name)?;
        }
        Some(Commands::View { name }) => {
            view_block(name)?;
        }
        Some(Commands::Copy { name }) => {
            copy_block(name)?;
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

fn view_block(name: &str) -> Result<()> {
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

    for (block_name, block_content) in blocks {
        if block_name == name {
            for line in block_content {
                println!("{}", line);
            }
        }
    }
    Ok(())
}

fn copy_block(name: &str) -> Result<()> {
    let file = File::open(get_save_file_path())?;
    let reader = BufReader::new(file);
    let mut blocks = Vec::new();
    let mut current_block = Vec::new();
    let mut current_name = String::new();
    let mut matched_name: Option<String> = None;

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

    for (block_name, block_content) in blocks {
        if block_name == name {
            let result_ctx = ClipboardContext::new();

            if result_ctx.is_err() {
                bail!("Failed to create clipboard context for copy block");
            }

            let mut ctx = result_ctx.unwrap();

            let is_success = ctx.set_contents(block_content.join("\n"));

            if is_success.is_err() {
                bail!(format!(
                    "Failed to copy contents of block {} to system clipboard",
                    block_name
                ));
            }
            matched_name = Some(block_name);
        }
    }
    match matched_name {
        Some(name) => println!("Successfully copied contents from block {}", name),
        None => println!("Didn't find the block. Please try again. You can use `thoth list` to find the name of all blocks")
    };

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

fn edit_with_external_editor(content: &str) -> Result<String> {
    let mut temp_file = NamedTempFile::new()?;

    temp_file.write_all(content.as_bytes())?;
    temp_file.flush()?;

    let editor = env::var("VISUAL")
        .or_else(|_| env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_string());

    // suspend the TUI
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;

    let status = Command::new(&editor).arg(temp_file.path()).status()?;

    // resume the TUI
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;

    if !status.success() {
        bail!(format!("Editor '{}' returned non-zero status", editor));
    }

    let edited_content = fs::read_to_string(temp_file.path())?;

    Ok(edited_content)
}

fn run_ui() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut scrollable_textarea = ScrollableTextArea::new();
    let mut title_popup = TitlePopup::new();
    let mut edit_commands_popup = EditCommandsPopup::new();

    if get_save_file_path().exists() {
        let (loaded_textareas, loaded_titles) = load_textareas()?;
        for (textarea, title) in loaded_textareas.into_iter().zip(loaded_titles) {
            scrollable_textarea.add_textarea(textarea, title);
        }
    } else {
        scrollable_textarea.add_textarea(TextArea::default(), String::from("New Textarea"));
    }

    scrollable_textarea.initialize_scroll();
    let mut title_select_popup = TitleSelectPopup::new();
    let mut clipboard = ClipboardContext::new().expect("Failed to initialize clipboard");

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

            render_header(f, chunks[0], scrollable_textarea.edit_mode);
            if scrollable_textarea.full_screen_mode {
                scrollable_textarea
                    .render(f, f.size())
                    .context("Failed to render")
                    .unwrap();
            } else {
                scrollable_textarea.render(f, chunks[1]).unwrap();
            }

            if title_popup.visible {
                render_title_popup(f, &title_popup);
            } else if title_select_popup.visible {
                render_title_select_popup(f, &title_select_popup);
            }

            if edit_commands_popup.visible {
                render_edit_commands_popup(f);
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

                            if key.modifiers.contains(KeyModifiers::SHIFT)
                                && scrollable_textarea.start_sel == 0
                            {
                                let (curr_row, _) = scrollable_textarea.textareas
                                    [scrollable_textarea.focused_index]
                                    .cursor();
                                scrollable_textarea.start_sel = curr_row;
                            }
                        } else {
                            scrollable_textarea.scroll =
                                scrollable_textarea.scroll.saturating_sub(1);
                        }
                    }
                    KeyCode::Down => {
                        if scrollable_textarea.edit_mode {
                            scrollable_textarea.textareas[scrollable_textarea.focused_index]
                                .move_cursor(tui_textarea::CursorMove::Down);

                            if key.modifiers.contains(KeyModifiers::SHIFT)
                                && scrollable_textarea.start_sel == 0
                            {
                                let (curr_row, _) = scrollable_textarea.textareas
                                    [scrollable_textarea.focused_index]
                                    .cursor();
                                scrollable_textarea.start_sel = curr_row;
                            }
                        } else {
                            scrollable_textarea.scroll += 1;
                        }
                    }
                    KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if let Err(e) = scrollable_textarea.copy_focused_textarea_contents() {
                            eprintln!("Failed to copy to clipboard: {}", e);
                        }
                    }
                    KeyCode::Char('s')
                        if key.modifiers.contains(KeyModifiers::ALT)
                            && key.modifiers.contains(KeyModifiers::SHIFT) =>
                    {
                        if scrollable_textarea.edit_mode {
                            scrollable_textarea.textareas[scrollable_textarea.focused_index]
                                .start_selection();
                        }
                    }
                    KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if let Err(e) = scrollable_textarea.copy_selection_contents() {
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
                        #[allow(clippy::assigning_clones)]
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
                        edit_commands_popup.visible = false;
                    }
                    KeyCode::Up => {
                        if title_select_popup.selected_index > 0 {
                            title_select_popup.selected_index -= 1;
                        } else {
                            title_select_popup.selected_index = title_select_popup.titles.len() - 1
                        }
                    }
                    KeyCode::Down => {
                        if title_select_popup.selected_index < title_select_popup.titles.len() - 1 {
                            title_select_popup.selected_index += 1;
                        } else {
                            title_select_popup.selected_index = 0;
                        }
                    }
                    _ => {}
                }
            } else {
                match key.code {
                    KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        let current_content = scrollable_textarea.textareas
                            [scrollable_textarea.focused_index]
                            .lines()
                            .join("\n");
                        match format_markdown(&current_content) {
                            Ok(formatted) => {
                                let mut new_textarea = TextArea::default();
                                for line in formatted.lines() {
                                    new_textarea.insert_str(line);
                                    new_textarea.insert_newline();
                                }
                                scrollable_textarea.textareas[scrollable_textarea.focused_index] =
                                    new_textarea;
                            }
                            Err(e) => {
                                eprintln!("Failed to format Markdown: {}", e);
                            }
                        }
                    }
                    KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        let current_content = scrollable_textarea.textareas
                            [scrollable_textarea.focused_index]
                            .lines()
                            .join("\n");
                        match format_json(&current_content) {
                            Ok(formatted) => {
                                let mut new_textarea = TextArea::default();
                                for line in formatted.lines() {
                                    new_textarea.insert_str(line);
                                    new_textarea.insert_newline();
                                }
                                scrollable_textarea.textareas[scrollable_textarea.focused_index] =
                                    new_textarea;
                            }
                            Err(e) => {
                                eprintln!("Failed to format json: {}", e);
                            }
                        }
                    }
                    // external editor
                    KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        let current_content = scrollable_textarea.textareas
                            [scrollable_textarea.focused_index]
                            .lines()
                            .join("\n");

                        match edit_with_external_editor(&current_content) {
                            Ok(edited_content) => {
                                let mut new_textarea = TextArea::default();
                                for line in edited_content.lines() {
                                    new_textarea.insert_str(line);
                                    new_textarea.insert_newline();
                                }
                                scrollable_textarea.textareas[scrollable_textarea.focused_index] =
                                    new_textarea;

                                // Redraw the terminal after editing
                                terminal.clear()?;
                            }
                            Err(e) => {
                                eprintln!("Failed to edit with external editor: {}", e);
                            }
                        }
                    }
                    KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if let Err(e) = scrollable_textarea.copy_focused_textarea_contents() {
                            eprintln!("Failed to copy to clipboard: {}", e);
                        }
                    }
                    // copy highlighted selection
                    KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if let Err(e) = scrollable_textarea.copy_selection_contents() {
                            eprintln!("Failed to copy to clipboard: {}", e);
                        }
                    }
                    KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        match key.modifiers {
                            KeyModifiers::CONTROL => {
                                if scrollable_textarea.edit_mode {
                                    if let Ok(content) = clipboard.get_contents() {
                                        let textarea = &mut scrollable_textarea.textareas
                                            [scrollable_textarea.focused_index];
                                        for line in content.lines() {
                                            textarea.insert_str(line);
                                            textarea.insert_newline();
                                        }
                                        // Remove the last extra newline
                                        if content.ends_with('\n') {
                                            textarea.delete_char();
                                        }
                                    }
                                }
                            }
                            KeyModifiers::SUPER | KeyModifiers::HYPER | KeyModifiers::META => {}
                            _ => {}
                        }
                    }
                    KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if !scrollable_textarea.edit_mode {
                            scrollable_textarea.toggle_full_screen();
                        }
                    }
                    // edit commands
                    KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        // Toggle the visibility of the edit commands popup
                        if scrollable_textarea.edit_mode {
                            edit_commands_popup.visible = !edit_commands_popup.visible;
                        }
                    }
                    KeyCode::Char('s')
                        if key.modifiers.contains(KeyModifiers::CONTROL)
                            && !key.modifiers.contains(KeyModifiers::SHIFT) =>
                    {
                        #[allow(clippy::assigning_clones)]
                        title_select_popup
                            .titles
                            .clone_from(&scrollable_textarea.titles);
                        title_select_popup.selected_index = 0;
                        title_select_popup.visible = true;
                    }
                    KeyCode::Char('q') => {
                        // allow q in edit mode
                        if !scrollable_textarea.edit_mode {
                            save_textareas(
                                &scrollable_textarea.textareas,
                                &scrollable_textarea.titles,
                            )?;
                            break;
                        }
                        scrollable_textarea.textareas[scrollable_textarea.focused_index].input(key);
                    }
                    KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if !scrollable_textarea.edit_mode {
                            scrollable_textarea
                                .add_textarea(TextArea::default(), String::from("New Textarea"));
                            scrollable_textarea.adjust_scroll_to_focused();
                        }
                    }
                    KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if scrollable_textarea.textareas.len() > 1 && !scrollable_textarea.edit_mode
                        {
                            scrollable_textarea.remove_textarea(scrollable_textarea.focused_index);
                        }
                    }
                    // move cursor to the top
                    KeyCode::Char('g') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if scrollable_textarea.edit_mode {
                            scrollable_textarea.textareas[scrollable_textarea.focused_index]
                                .move_cursor(tui_textarea::CursorMove::Top);
                        }
                    }
                    KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        title_popup.visible = true;
                        #[allow(clippy::assigning_clones)]
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
                        edit_commands_popup.visible = false;
                    }
                    KeyCode::Up => {
                        if scrollable_textarea.edit_mode {
                            let textarea = &mut scrollable_textarea.textareas
                                [scrollable_textarea.focused_index];
                            if key.modifiers.contains(KeyModifiers::SHIFT) {
                                if scrollable_textarea.start_sel == usize::MAX {
                                    let (curr_row, _) = textarea.cursor();
                                    scrollable_textarea.start_sel = curr_row;
                                    textarea.start_selection();
                                }
                                if textarea.cursor().0 > 0 {
                                    textarea.move_cursor(tui_textarea::CursorMove::Up);
                                }
                            } else {
                                textarea.move_cursor(tui_textarea::CursorMove::Up);
                                scrollable_textarea.start_sel = usize::MAX;
                                textarea.cancel_selection();
                            }
                        } else {
                            scrollable_textarea.move_focus(-1);
                        }
                    }
                    KeyCode::Down => {
                        if scrollable_textarea.edit_mode {
                            let textarea = &mut scrollable_textarea.textareas
                                [scrollable_textarea.focused_index];
                            if key.modifiers.contains(KeyModifiers::SHIFT) {
                                if scrollable_textarea.start_sel == usize::MAX {
                                    let (curr_row, _) = textarea.cursor();
                                    scrollable_textarea.start_sel = curr_row;
                                    textarea.start_selection();
                                }
                                if textarea.cursor().0 < textarea.lines().len() - 1 {
                                    textarea.move_cursor(tui_textarea::CursorMove::Down);
                                }
                            } else {
                                textarea.move_cursor(tui_textarea::CursorMove::Down);
                                scrollable_textarea.start_sel = usize::MAX;
                                textarea.cancel_selection();
                            }
                        } else {
                            scrollable_textarea.move_focus(1);
                        }
                    }
                    _ => {
                        if scrollable_textarea.edit_mode {
                            scrollable_textarea.textareas[scrollable_textarea.focused_index]
                                .input(key);
                            scrollable_textarea.start_sel = usize::MAX;
                            scrollable_textarea.textareas[scrollable_textarea.focused_index]
                                .cancel_selection();
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
    let mut file = File::create(get_save_file_path())?;
    for (textarea, title) in textareas.iter().zip(titles.iter()) {
        writeln!(file, "# {}", title)?;
        let content = textarea.lines().join("\n");
        let mut in_code_block = false;
        for line in content.lines() {
            if line.trim().starts_with("```") {
                in_code_block = !in_code_block;
            }
            if in_code_block || !line.starts_with('#') {
                writeln!(file, "{}", line)?;
            } else {
                writeln!(file, "\\{}", line)?;
            }
        }
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
