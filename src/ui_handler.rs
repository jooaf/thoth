use crate::EditorClipboard;
use anyhow::{bail, Result};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, KeyCode, KeyModifiers},
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
        render_edit_commands_popup, render_error_popup, render_header, render_title_popup,
        render_title_select_popup, EditCommandsPopup, ErrorPopup,
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
    pub error_popup: ErrorPopup,
    pub edit_commands_popup: EditCommandsPopup,
    pub clipboard: Option<EditorClipboard>,
    pub last_draw: Instant,
}

impl UIState {
    pub fn new() -> Result<Self> {
        let mut scrollable_textarea = ScrollableTextArea::new();
        if get_save_file_path().exists() {
            let (loaded_textareas, loaded_titles) = load_textareas()?;
            for (textarea, title) in loaded_textareas.into_iter().zip(loaded_titles) {
                scrollable_textarea.add_textarea(textarea, title);
            }
        } else {
            scrollable_textarea.add_textarea(TextArea::default(), String::from("New Textarea"));
        }
        scrollable_textarea.initialize_scroll();

        Ok(UIState {
            scrollable_textarea,
            title_popup: TitlePopup::new(),
            title_select_popup: TitleSelectPopup::new(),
            error_popup: ErrorPopup::new(),
            edit_commands_popup: EditCommandsPopup::new(),
            clipboard: EditorClipboard::try_new(),
            last_draw: Instant::now(),
        })
    }
}

pub fn draw_ui(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut UIState,
) -> Result<()> {
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

        render_header(f, chunks[0], state.scrollable_textarea.edit_mode);
        if state.scrollable_textarea.full_screen_mode {
            state.scrollable_textarea.render(f, f.size()).unwrap();
        } else {
            state.scrollable_textarea.render(f, chunks[1]).unwrap();
        }

        if state.title_popup.visible {
            render_title_popup(f, &state.title_popup);
        } else if state.title_select_popup.visible {
            render_title_select_popup(f, &state.title_select_popup);
        }

        if state.edit_commands_popup.visible {
            render_edit_commands_popup(f);
        }

        if state.error_popup.visible {
            render_error_popup(f, &state.error_popup);
        }
    })?;
    Ok(())
}

pub fn handle_input(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut UIState,
    key: event::KeyEvent,
) -> Result<bool> {
    if state.scrollable_textarea.full_screen_mode {
        handle_full_screen_input(state, key)
    } else if state.title_popup.visible {
        handle_title_popup_input(state, key)
    } else if state.title_select_popup.visible {
        handle_title_select_popup_input(state, key)
    } else {
        handle_normal_input(terminal, state, key)
    }
}

fn handle_full_screen_input(state: &mut UIState, key: event::KeyEvent) -> Result<bool> {
    match key.code {
        KeyCode::Esc => {
            if state.scrollable_textarea.edit_mode {
                state.scrollable_textarea.edit_mode = false;
            } else {
                state.scrollable_textarea.toggle_full_screen();
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
        KeyCode::Up => handle_up_key(state, key),
        KeyCode::Down => handle_down_key(state, key),
        KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if let Err(e) = state.scrollable_textarea.copy_focused_textarea_contents() {
                state
                    .error_popup
                    .show(format!("Failed to copy to clipboard: {}", e));
            }
        }
        KeyCode::Char('s')
            if key.modifiers.contains(KeyModifiers::ALT)
                && key.modifiers.contains(KeyModifiers::SHIFT) =>
        {
            if state.scrollable_textarea.edit_mode {
                state.scrollable_textarea.textareas[state.scrollable_textarea.focused_index]
                    .start_selection();
            }
        }
        KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if let Err(e) = state.scrollable_textarea.copy_selection_contents() {
                state
                    .error_popup
                    .show(format!("Failed to copy to clipboard: {}", e));
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
    match key.code {
        KeyCode::Enter => {
            state
                .scrollable_textarea
                .jump_to_textarea(state.title_select_popup.selected_index);
            state.title_select_popup.visible = false;
        }
        KeyCode::Esc => {
            state.title_select_popup.visible = false;
            state.edit_commands_popup.visible = false;
        }
        KeyCode::Up => {
            if state.title_select_popup.selected_index > 0 {
                state.title_select_popup.selected_index -= 1;
            } else {
                state.title_select_popup.selected_index = state.title_select_popup.titles.len() - 1
            }
        }
        KeyCode::Down => {
            if state.title_select_popup.selected_index < state.title_select_popup.titles.len() - 1 {
                state.title_select_popup.selected_index += 1;
            } else {
                state.title_select_popup.selected_index = 0;
            }
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
            // edit_with_external_editor(state)?;
        }
        KeyCode::Char('y') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if let Err(e) = state.scrollable_textarea.copy_focused_textarea_contents() {
                state
                    .error_popup
                    .show(format!("Failed to copy to clipboard: {}", e));
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
            state.title_select_popup.titles = state.scrollable_textarea.titles.clone();
            state.title_select_popup.selected_index = 0;
            state.title_select_popup.visible = true;
        }
        KeyCode::Char('q') => {
            if !state.scrollable_textarea.edit_mode {
                save_textareas(
                    &state.scrollable_textarea.textareas,
                    &state.scrollable_textarea.titles,
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
            state.scrollable_textarea.edit_mode = false;
            state.edit_commands_popup.visible = false;

            if state.error_popup.visible {
                state.error_popup.hide();
            }
        }
        KeyCode::Up => handle_up_key(state, key),
        KeyCode::Down => handle_down_key(state, key),
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
