use crate::{
    get_save_backup_file_path, utils::extract_code_blocks, CodeBlockPopup, EditorClipboard,
    ThemeMode, ThothConfig,
};
use anyhow::{bail, Result};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    io::{self, Write},
    time::Instant,
};
use tui_textarea::TextArea;

use crate::{
    format_json, format_markdown, get_save_file_path, load_textareas, save_textareas,
    ui::{
        render_code_block_popup, render_edit_commands_popup, render_header, render_title_popup,
        render_title_select_popup, render_ui_popup, EditCommandsPopup, UiPopup,
    },
    ScrollableTextArea, TitlePopup, TitleSelectPopup,
};

use std::env;
use std::fs;
use std::process::Command;
use tempfile::NamedTempFile;

pub struct UIState {
    pub scrollable_textarea: ScrollableTextArea,
    pub title_popup: TitlePopup,
    pub title_select_popup: TitleSelectPopup,
    pub error_popup: UiPopup,
    pub help_popup: UiPopup,
    pub copy_popup: UiPopup,
    pub edit_commands_popup: EditCommandsPopup,
    pub code_block_popup: CodeBlockPopup,
    pub clipboard: Option<EditorClipboard>,
    pub last_draw: Instant,
    pub config: ThothConfig,
}

impl UIState {
    pub fn new() -> Result<Self> {
        let mut scrollable_textarea = ScrollableTextArea::new();
        let main_save_path = get_save_file_path();
        if main_save_path.exists() {
            let (loaded_textareas, loaded_titles) = load_textareas(main_save_path)?;
            for (textarea, title) in loaded_textareas.into_iter().zip(loaded_titles) {
                scrollable_textarea.add_textarea(textarea, title);
            }
        } else {
            scrollable_textarea.add_textarea(TextArea::default(), String::from("New Textarea"));
        }
        scrollable_textarea.initialize_scroll();

        let config = ThothConfig::load()?;

        Ok(UIState {
            scrollable_textarea,
            title_popup: TitlePopup::new(),
            title_select_popup: TitleSelectPopup::new(),
            error_popup: UiPopup::new("Error".to_string(), 60, 20),
            copy_popup: UiPopup::new("Block Copied".to_string(), 60, 20),
            help_popup: UiPopup::new("Keyboard Shortcuts".to_string(), 60, 80),
            edit_commands_popup: EditCommandsPopup::new(),
            code_block_popup: CodeBlockPopup::new(),
            clipboard: EditorClipboard::try_new(),
            last_draw: Instant::now(),
            config,
        })
    }
}

pub fn draw_ui(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut UIState,
) -> Result<()> {
    terminal.draw(|f| {
        let theme = state.config.get_theme_colors();

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

        render_header(f, chunks[0], state.scrollable_textarea.edit_mode, theme);
        if state.scrollable_textarea.full_screen_mode {
            state
                .scrollable_textarea
                .render(f, f.size(), theme)
                .unwrap();
        } else {
            state
                .scrollable_textarea
                .render(f, chunks[1], theme)
                .unwrap();
        }

        if state.copy_popup.visible {
            render_ui_popup(f, &state.copy_popup, theme);
        }
        if state.title_popup.visible {
            render_title_popup(f, &state.title_popup, theme);
        } else if state.title_select_popup.visible {
            render_title_select_popup(f, &state.title_select_popup, theme);
        } else if state.code_block_popup.visible {
            render_code_block_popup(f, &state.code_block_popup, theme);
        }

        if state.edit_commands_popup.visible {
            render_edit_commands_popup(f, theme);
        }

        if state.error_popup.visible {
            render_ui_popup(f, &state.error_popup, theme);
        }
        if state.help_popup.visible {
            render_ui_popup(f, &state.help_popup, theme);
        }

        if state.help_popup.visible {
            render_ui_popup(f, &state.help_popup, theme);
        }
    })?;
    Ok(())
}

fn handle_code_block_popup_input(state: &mut UIState, key: event::KeyEvent) -> Result<bool> {
    let visible_items =
        (state.scrollable_textarea.viewport_height as f32 * 0.8).floor() as usize - 4;

    match key.code {
        KeyCode::Enter => {
            if !state.code_block_popup.filtered_blocks.is_empty() {
                let selected_index = state.code_block_popup.selected_index;
                let content = state.code_block_popup.filtered_blocks[selected_index]
                    .content
                    .clone();
                let language = state.code_block_popup.filtered_blocks[selected_index]
                    .language
                    .clone();

                if let Err(e) = copy_code_block_content_to_clipboard(state, &content, &language) {
                    state.error_popup.show(format!("{}", e));
                }

                state.code_block_popup.visible = false;
            }
        }
        KeyCode::Esc => {
            state.code_block_popup.visible = false;
        }
        KeyCode::Up => {
            state.code_block_popup.move_selection_up(visible_items);
        }
        KeyCode::Down => {
            state.code_block_popup.move_selection_down(visible_items);
        }
        _ => {}
    }
    Ok(false)
}

fn extract_and_show_code_blocks(state: &mut UIState) -> Result<()> {
    let content = state.scrollable_textarea.textareas[state.scrollable_textarea.focused_index]
        .lines()
        .join("\n");

    let code_blocks = extract_code_blocks(&content);

    if code_blocks.is_empty() {
        state
            .error_popup
            .show("No code blocks found in the current note.".to_string());
        return Ok(());
    }

    state.code_block_popup.set_code_blocks(code_blocks);
    state.code_block_popup.visible = true;
    Ok(())
}

fn copy_code_block_content_to_clipboard(
    state: &mut UIState,
    content: &str,
    language: &str,
) -> Result<()> {
    match &mut state.clipboard {
        Some(clip) => {
            if let Err(e) = clip.set_contents(content.to_string()) {
                let backup_path = crate::get_clipboard_backup_file_path();
                std::fs::write(&backup_path, content)?;

                return Err(anyhow::anyhow!(
                    "Clipboard error: {}.\nContent saved to: {}\nPlease use 'thoth read_clipboard' to read the contents from STDOUT.",
                    e.to_string().split('\n').next().unwrap_or("Unknown error"),
                    backup_path.display()
                ));
            }

            state.copy_popup.show(format!(
                "Copied code block [{}] to clipboard",
                if language.is_empty() {
                    "no language"
                } else {
                    language
                }
            ));
        }
        None => {
            let backup_path = crate::get_clipboard_backup_file_path();
            std::fs::write(&backup_path, content)?;

            return Err(anyhow::anyhow!(
                "Clipboard unavailable.\nContent saved to: {}\nPlease use 'thoth read_clipboard' to read the contents from STDOUT.",
                backup_path.display()
            ));
        }
    }
    Ok(())
}

pub fn handle_input(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut UIState,
    key: event::KeyEvent,
) -> Result<bool> {
    if key.kind != KeyEventKind::Press {
        return Ok(false);
    }

    if state.code_block_popup.visible {
        handle_code_block_popup_input(state, key)
    } else if state.scrollable_textarea.full_screen_mode {
        handle_full_screen_input(terminal, state, key)
    } else if state.title_popup.visible {
        handle_title_popup_input(state, key)
    } else if state.title_select_popup.visible {
        handle_title_select_popup_input(state, key)
    } else {
        handle_normal_input(terminal, state, key)
    }
}

fn handle_full_screen_input(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut UIState,
    key: event::KeyEvent,
) -> Result<bool> {
    match key.code {
        KeyCode::Esc => {
            if state.copy_popup.visible {
                state.copy_popup.hide();
            } else if state.error_popup.visible {
                state.error_popup.hide();
            } else if state.help_popup.visible {
                state.help_popup.hide();
            } else if state.edit_commands_popup.visible {
                state.edit_commands_popup.visible = false;
            } else if state.scrollable_textarea.edit_mode {
                state.scrollable_textarea.edit_mode = false;
            } else {
                state.scrollable_textarea.toggle_full_screen();
                state
                    .scrollable_textarea
                    .jump_to_textarea(state.scrollable_textarea.focused_index);
            }
        }
        KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if state.scrollable_textarea.edit_mode {
                match edit_with_external_editor(state) {
                    Ok(edited_content) => {
                        let mut new_textarea = TextArea::default();
                        for line in edited_content.lines() {
                            new_textarea.insert_str(line);
                            new_textarea.insert_newline();
                        }
                        state.scrollable_textarea.textareas
                            [state.scrollable_textarea.focused_index] = new_textarea;

                        terminal.clear()?;
                    }
                    Err(e) => {
                        state
                            .error_popup
                            .show(format!("Failed to edit with external editor: {}", e));
                    }
                }
            }
        }
        KeyCode::Enter => {
            if !state.scrollable_textarea.edit_mode {
                state.scrollable_textarea.edit_mode = true;
            } else {
                state.scrollable_textarea.textareas[state.scrollable_textarea.focused_index]
                    .insert_newline();
            }
        }
        KeyCode::Up => {
            if state.scrollable_textarea.edit_mode {
                handle_up_key(state, key);
            } else {
                state.scrollable_textarea.handle_scroll(-1);
            }
        }
        KeyCode::Down => {
            if state.scrollable_textarea.edit_mode {
                handle_down_key(state, key);
            } else {
                state.scrollable_textarea.handle_scroll(1);
            }
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if !state.scrollable_textarea.edit_mode {
                if let Err(e) = extract_and_show_code_blocks(state) {
                    state
                        .error_popup
                        .show(format!("Error extracting code blocks: {}", e));
                }
            } else {
                state.scrollable_textarea.textareas[state.scrollable_textarea.focused_index]
                    .input(key);
            }
        }
        KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            match state.scrollable_textarea.copy_focused_textarea_contents() {
                Ok(_) => {
                    let curr_focused_index = state.scrollable_textarea.focused_index;
                    let curr_title_option =
                        state.scrollable_textarea.titles.get(curr_focused_index);

                    match curr_title_option {
                        Some(curr_title) => {
                            state
                                .copy_popup
                                .show(format!("Copied block {}", curr_title));
                        }
                        None => {
                            state
                                .error_popup
                                .show("Failed to copy selection with title".to_string());
                        }
                    }
                }
                Err(e) => {
                    state.error_popup.show(format!("{}", e));
                }
            }
        }
        _ => {
            if state.scrollable_textarea.edit_mode {
                state.scrollable_textarea.textareas[state.scrollable_textarea.focused_index]
                    .input(key);
            }
        }
    }
    Ok(false)
}

fn handle_title_popup_input(state: &mut UIState, key: event::KeyEvent) -> Result<bool> {
    match key.code {
        KeyCode::Enter => {
            #[allow(clippy::assigning_clones)]
            state
                .scrollable_textarea
                .change_title(state.title_popup.title.clone());
            state.title_popup.visible = false;
            state.title_popup.title.clear();
        }
        KeyCode::Esc => {
            state.title_popup.visible = false;
            state.title_popup.title.clear();
        }
        KeyCode::Char(c) => {
            state.title_popup.title.push(c);
        }
        KeyCode::Backspace => {
            state.title_popup.title.pop();
        }
        _ => {}
    }
    Ok(false)
}

fn handle_title_select_popup_input(state: &mut UIState, key: event::KeyEvent) -> Result<bool> {
    let visible_items =
        (state.scrollable_textarea.viewport_height as f32 * 0.8).floor() as usize - 10;

    match key.code {
        KeyCode::Enter => {
            if !state.title_select_popup.filtered_titles.is_empty() {
                let selected_title_match = &state.title_select_popup.filtered_titles
                    [state.title_select_popup.selected_index];
                state
                    .scrollable_textarea
                    .jump_to_textarea(selected_title_match.index);
                state.title_select_popup.visible = false;
                if !state.title_select_popup.search_query.is_empty() {
                    state.title_select_popup.search_query.clear();
                    state.title_select_popup.reset_filtered_titles();
                }
            }
        }
        KeyCode::Esc => {
            state.title_select_popup.visible = false;
            state.edit_commands_popup.visible = false;
            if !state.title_select_popup.search_query.is_empty() {
                state.title_select_popup.search_query.clear();
                state.title_select_popup.reset_filtered_titles();
            }
        }
        KeyCode::Up => {
            state.title_select_popup.move_selection_up(visible_items);
        }
        KeyCode::Down => {
            state.title_select_popup.move_selection_down(visible_items);
        }
        KeyCode::Char(c) => {
            state.title_select_popup.search_query.push(c);
            state.title_select_popup.update_search();
        }
        KeyCode::Backspace => {
            state.title_select_popup.search_query.pop();
            state.title_select_popup.update_search();
        }

        _ => {}
    }
    Ok(false)
}

fn handle_normal_input(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut UIState,
    key: event::KeyEvent,
) -> Result<bool> {
    match key.code {
        KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            format_current_textarea(state, format_markdown)?;
        }
        KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            format_current_textarea(state, format_json)?;
        }
        KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if state.scrollable_textarea.edit_mode {
                match edit_with_external_editor(state) {
                    Ok(edited_content) => {
                        let mut new_textarea = TextArea::default();
                        for line in edited_content.lines() {
                            new_textarea.insert_str(line);
                            new_textarea.insert_newline();
                        }
                        state.scrollable_textarea.textareas
                            [state.scrollable_textarea.focused_index] = new_textarea;

                        // Redraw the terminal after editing
                        terminal.clear()?;
                    }
                    Err(e) => {
                        state
                            .error_popup
                            .show(format!("Failed to edit with external editor: {}", e));
                    }
                }
            }
        }
        KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let new_theme = match state.config.theme {
                ThemeMode::Light => ThemeMode::Dark,
                ThemeMode::Dark => ThemeMode::Light,
            };
            state.config.set_theme(new_theme.clone())?;
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if !state.scrollable_textarea.edit_mode {
                if let Err(e) = extract_and_show_code_blocks(state) {
                    state
                        .error_popup
                        .show(format!("Error extracting code blocks: {}", e));
                }
            } else {
                state.scrollable_textarea.textareas[state.scrollable_textarea.focused_index]
                    .input(key);
            }
        }
        KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            match state.scrollable_textarea.copy_focused_textarea_contents() {
                Ok(_) => {
                    let curr_focused_index = state.scrollable_textarea.focused_index;
                    let curr_title_option =
                        state.scrollable_textarea.titles.get(curr_focused_index);

                    match curr_title_option {
                        Some(curr_title) => {
                            state
                                .copy_popup
                                .show(format!("Copied block {}", curr_title));
                        }
                        None => {
                            state
                                .error_popup
                                .show("Failed to copy selection with title".to_string());
                        }
                    }
                }
                Err(e) => {
                    state.error_popup.show(format!("{}", e));
                }
            }
        }
        KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if let Err(e) = state.scrollable_textarea.copy_selection_contents() {
                state
                    .error_popup
                    .show(format!("Failed to copy to clipboard: {}", e));
            }
        }
        KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            handle_paste(state)?;
        }
        KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let help_message = "\
NAVIGATION:
  • ↑/↓ or j/k: Navigate between blocks
  • Enter: Enter edit mode
  • Esc: Exit current mode

BLOCKS:
  • ^n: Add a new block
  • ^d: Delete current block
  • ^t: Change block title
  • ^s: Select block by title
  • ^f: Toggle fullscreen mode

CLIPBOARD:
  • ^y: Copy current block
  • ^v: Paste from clipboard
  • ^b: Copy selection (in edit mode)
  • ^c: Copy code block from current note

FORMATTING:
  • ^j: Format as JSON
  • ^k: Format as Markdown

OTHER:
  • ^l: Toggle light/dark theme
  • ^e: Edit with external editor (in edit mode)
  • q: Quit application
  • ^h: Show this help";

            state.help_popup.show(help_message.to_string());
        }
        KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if !state.scrollable_textarea.edit_mode {
                state.scrollable_textarea.toggle_full_screen();
            }
        }
        KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if state.scrollable_textarea.edit_mode {
                state.edit_commands_popup.visible = !state.edit_commands_popup.visible;
            }
        }
        #[allow(clippy::assigning_clones)]
        KeyCode::Char('s')
            if key.modifiers.contains(KeyModifiers::CONTROL)
                && !key.modifiers.contains(KeyModifiers::SHIFT) =>
        {
            // populate title_select_popup with the current titles from the textareas
            state
                .title_select_popup
                .set_titles(state.scrollable_textarea.titles.clone());
            state.title_select_popup.selected_index = 0;
            state.title_select_popup.visible = true;
        }
        KeyCode::Char('q') => {
            if !state.scrollable_textarea.edit_mode {
                save_textareas(
                    &state.scrollable_textarea.textareas,
                    &state.scrollable_textarea.titles,
                    get_save_file_path(),
                )?;
                save_textareas(
                    &state.scrollable_textarea.textareas,
                    &state.scrollable_textarea.titles,
                    get_save_backup_file_path(),
                )?;
                return Ok(true);
            }
            state.scrollable_textarea.textareas[state.scrollable_textarea.focused_index].input(key);
        }
        KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if !state.scrollable_textarea.edit_mode {
                state
                    .scrollable_textarea
                    .add_textarea(TextArea::default(), String::from("New Textarea"));
                state.scrollable_textarea.adjust_scroll_to_focused();
            }
        }
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if state.scrollable_textarea.textareas.len() > 1 && !state.scrollable_textarea.edit_mode
            {
                state
                    .scrollable_textarea
                    .remove_textarea(state.scrollable_textarea.focused_index);
            }
        }
        KeyCode::Char('g') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if state.scrollable_textarea.edit_mode {
                state.scrollable_textarea.textareas[state.scrollable_textarea.focused_index]
                    .move_cursor(tui_textarea::CursorMove::Top);
            }
        }
        #[allow(clippy::assigning_clones)]
        KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.title_popup.visible = true;
            state.title_popup.title =
                state.scrollable_textarea.titles[state.scrollable_textarea.focused_index].clone();
        }
        KeyCode::Enter => {
            if state.scrollable_textarea.edit_mode {
                state.scrollable_textarea.textareas[state.scrollable_textarea.focused_index]
                    .insert_newline();
            } else {
                state.scrollable_textarea.edit_mode = true;
            }
        }
        KeyCode::Esc => {
            if state.edit_commands_popup.visible {
                state.edit_commands_popup.visible = false;
            } else {
                state.scrollable_textarea.edit_mode = false;
                state.edit_commands_popup.visible = false;
            }

            if state.error_popup.visible {
                state.error_popup.hide();
            }
            if state.help_popup.visible {
                state.help_popup.hide();
            }
            if state.copy_popup.visible {
                state.copy_popup.hide();
            }
        }
        KeyCode::Up => handle_up_key(state, key),
        KeyCode::Down => handle_down_key(state, key),
        KeyCode::Char('k') if !state.scrollable_textarea.edit_mode => handle_up_key(state, key),
        KeyCode::Char('j') if !state.scrollable_textarea.edit_mode => handle_down_key(state, key),
        _ => {
            if state.scrollable_textarea.edit_mode {
                state.scrollable_textarea.textareas[state.scrollable_textarea.focused_index]
                    .input(key);
                state.scrollable_textarea.start_sel = usize::MAX;
                state.scrollable_textarea.textareas[state.scrollable_textarea.focused_index]
                    .cancel_selection();
            }
        }
    }
    Ok(false)
}

fn handle_up_key(state: &mut UIState, key: event::KeyEvent) {
    if state.scrollable_textarea.edit_mode {
        let textarea =
            &mut state.scrollable_textarea.textareas[state.scrollable_textarea.focused_index];
        if key.modifiers.contains(KeyModifiers::SHIFT) {
            if state.scrollable_textarea.start_sel == usize::MAX {
                let (curr_row, _) = textarea.cursor();
                state.scrollable_textarea.start_sel = curr_row;
                textarea.start_selection();
            }
            if textarea.cursor().0 > 0 {
                textarea.move_cursor(tui_textarea::CursorMove::Up);
            }
        } else {
            textarea.move_cursor(tui_textarea::CursorMove::Up);
            state.scrollable_textarea.start_sel = usize::MAX;
            textarea.cancel_selection();
        }
    } else {
        state.scrollable_textarea.move_focus(-1);
    }
}

fn handle_down_key(state: &mut UIState, key: event::KeyEvent) {
    if state.scrollable_textarea.edit_mode {
        let textarea =
            &mut state.scrollable_textarea.textareas[state.scrollable_textarea.focused_index];
        if key.modifiers.contains(KeyModifiers::SHIFT) {
            if state.scrollable_textarea.start_sel == usize::MAX {
                let (curr_row, _) = textarea.cursor();
                state.scrollable_textarea.start_sel = curr_row;
                textarea.start_selection();
            }
            if textarea.cursor().0 < textarea.lines().len() - 1 {
                textarea.move_cursor(tui_textarea::CursorMove::Down);
            }
        } else {
            textarea.move_cursor(tui_textarea::CursorMove::Down);
            state.scrollable_textarea.start_sel = usize::MAX;
            textarea.cancel_selection();
        }
    } else {
        state.scrollable_textarea.move_focus(1);
    }
}

fn format_current_textarea<F>(state: &mut UIState, formatter: F) -> Result<()>
where
    F: Fn(&str) -> Result<String>,
{
    let current_content = state.scrollable_textarea.textareas
        [state.scrollable_textarea.focused_index]
        .lines()
        .join("\n");
    match formatter(&current_content) {
        Ok(formatted) => {
            let mut new_textarea = TextArea::default();
            for line in formatted.lines() {
                new_textarea.insert_str(line);
                new_textarea.insert_newline();
            }
            state.scrollable_textarea.textareas[state.scrollable_textarea.focused_index] =
                new_textarea;
            Ok(())
        }
        Err(e) => {
            state
                .error_popup
                .show(format!("Failed to format block: {}", e));
            Ok(())
        }
    }
}

fn edit_with_external_editor(state: &mut UIState) -> Result<String> {
    let content = state.scrollable_textarea.textareas[state.scrollable_textarea.focused_index]
        .lines()
        .join("\n");
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

fn handle_paste(state: &mut UIState) -> Result<()> {
    if state.scrollable_textarea.edit_mode {
        match &mut state.clipboard {
            Some(clip) => {
                if let Ok(content) = clip.get_content() {
                    let textarea = &mut state.scrollable_textarea.textareas
                        [state.scrollable_textarea.focused_index];
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
            None => {
                state
                    .error_popup
                    .show("Failed to create clipboard".to_string());
            }
        }
    }
    Ok(())
}
