use crate::{ThemeColors, TitlePopup, TitleSelectPopup, BORDER_PADDING_SIZE, DARK_MODE_COLORS};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Tabs},
    Frame,
};
use unicode_width::UnicodeWidthStr;

pub struct EditCommandsPopup {
    pub visible: bool,
}

impl EditCommandsPopup {
    pub fn new() -> Self {
        EditCommandsPopup { visible: false }
    }
}
impl Default for EditCommandsPopup {
    fn default() -> Self {
        Self::new()
    }
}

pub struct UiPopup {
    pub message: String,
    pub popup_title: String,
    pub visible: bool,
    pub percent_x: u16,
    pub percent_y: u16,
}

impl UiPopup {
    pub fn new(popup_title: String, percent_x: u16, percent_y: u16) -> Self {
        UiPopup {
            message: String::new(),
            visible: false,
            popup_title,
            percent_x,
            percent_y,
        }
    }

    pub fn show(&mut self, message: String) {
        self.message = message;
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }
}

impl Default for UiPopup {
    fn default() -> Self {
        Self::new("".to_owned(), 60, 20)
    }
}

pub fn render_edit_commands_popup(f: &mut Frame, theme: &ThemeColors) {
    let area = centered_rect(80, 80, f.size());
    f.render_widget(ratatui::widgets::Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.primary))
        .title("Editing Commands - Esc to exit");

    let header = Row::new(vec![
        Cell::from("MAPPINGS").style(
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from("DESCRIPTIONS").style(
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
    ])
    .height(BORDER_PADDING_SIZE as u16);

    let commands: Vec<Row> = vec![
        Row::new(vec![
            "Ctrl+H, Backspace",
            "Delete one character before cursor",
        ]),
        Row::new(vec!["Ctrl+K", "Delete from cursor until the end of line"]),
        Row::new(vec![
            "Ctrl+W, Alt+Backspace",
            "Delete one word before cursor",
        ]),
        Row::new(vec!["Alt+D, Alt+Delete", "Delete one word next to cursor"]),
        Row::new(vec!["Ctrl+U", "Undo"]),
        Row::new(vec!["Ctrl+R", "Redo"]),
        Row::new(vec!["Ctrl+C, Copy", "Copy selected text"]),
        Row::new(vec!["Ctrl+X, Cut", "Cut selected text"]),
        Row::new(vec!["Ctrl+P, ↑", "Move cursor up by one line"]),
        Row::new(vec!["Ctrl+→", "Move cursor forward by word"]),
        Row::new(vec!["Ctrl+←", "Move cursor backward by word"]),
        Row::new(vec!["Ctrl+↑", "Move cursor up by paragraph"]),
        Row::new(vec!["Ctrl+↓", "Move cursor down by paragraph"]),
        Row::new(vec![
            "Ctrl+E, End, Ctrl+Alt+F, Ctrl+Alt+→",
            "Move cursor to the end of line",
        ]),
        Row::new(vec![
            "Ctrl+A, Home, Ctrl+Alt+B, Ctrl+Alt+←",
            "Move cursor to the head of line",
        ]),
        Row::new(vec!["Ctrl+L", "Toggle between light and dark mode"]),
        Row::new(vec!["Ctrl+K", "Format markdown block"]),
        Row::new(vec!["Ctrl+J", "Format JSON"]),
    ];

    let table = Table::new(commands, [Constraint::Length(5), Constraint::Length(5)])
        .header(header)
        .block(block)
        .widths([Constraint::Percentage(30), Constraint::Percentage(70)])
        .column_spacing(BORDER_PADDING_SIZE as u16)
        .highlight_style(Style::default().fg(theme.accent))
        .highlight_symbol(">> ");

    f.render_widget(table, area);
}

pub fn render_header(f: &mut Frame, area: Rect, is_edit_mode: bool, theme: &ThemeColors) {
    let available_width = area.width as usize;
    let normal_commands = vec![
        "q:Quit",
        "^h:Help",
        "^n:Add",
        "^d:Del",
        "^y:Copy",
        "^v:Paste",
        "Enter:Edit",
        "^f:Focus",
        "Esc:Exit",
        "^t:Title",
        "^s:Select",
        "^l:Toggle Theme",
        "^j:Format JSON",
        "^k:Format Markdown",
    ];
    let edit_commands = vec![
        "Esc:Exit Edit",
        "^g:Move Cursor Top",
        "^b:Copy Sel",
        "Shift+↑↓:Sel",
        "^y:Copy All",
        "^t:Title",
        "^s:Select",
        "^e:External Editor",
        "^h:Help",
        "^l:Toggle Theme",
    ];
    let commands = if is_edit_mode {
        &edit_commands
    } else {
        &normal_commands
    };
    let thoth = "Thoth  ";
    let separator = " | ";

    let thoth_width = thoth.width();
    let separator_width = separator.width();
    let reserved_width = thoth_width + BORDER_PADDING_SIZE; // 2 extra spaces for padding

    let mut display_commands = Vec::new();
    let mut current_width = 0;

    for cmd in commands {
        let cmd_width = cmd.width();
        if current_width + cmd_width + separator_width > available_width - reserved_width {
            break;
        }
        display_commands.push(*cmd);
        current_width += cmd_width + separator_width;
    }

    let command_string = display_commands.join(separator);
    let command_width = command_string.width();

    let padding = " ".repeat(available_width - command_width - thoth_width - BORDER_PADDING_SIZE);

    let header = Line::from(vec![
        Span::styled(command_string, Style::default().fg(theme.accent)),
        Span::styled(padding, Style::default().fg(theme.accent)),
        Span::styled(format!(" {} ", thoth), Style::default().fg(theme.accent)),
    ]);

    let tabs = Tabs::new(vec![header])
        .style(Style::default().bg(theme.header_bg))
        .divider(Span::styled("|", Style::default().fg(theme.accent)));

    f.render_widget(tabs, area);
}

pub fn render_help_popup(f: &mut Frame, popup: &UiPopup, theme: ThemeColors) {
    if !popup.visible {
        return;
    }

    let area = centered_rect(80, 80, f.size());
    f.render_widget(ratatui::widgets::Clear, area);

    let border_color = if theme == *DARK_MODE_COLORS {
        theme.accent
    } else {
        theme.primary
    };

    let text = Paragraph::new(popup.message.as_str())
        .style(Style::default().fg(theme.foreground))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .title(format!("{} - Esc to exit", popup.popup_title)),
        )
        .wrap(ratatui::widgets::Wrap { trim: true });

    f.render_widget(text, area);
}

pub fn render_title_popup(f: &mut Frame, popup: &TitlePopup, theme: &ThemeColors) {
    let area = centered_rect(60, 20, f.size());
    f.render_widget(ratatui::widgets::Clear, area);

    let text = Paragraph::new(popup.title.as_str())
        .style(Style::default().bg(theme.background).fg(theme.foreground))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.primary))
                .title("Change Title"),
        );
    f.render_widget(text, area);
}

pub fn render_title_select_popup(f: &mut Frame, popup: &TitleSelectPopup, theme: &ThemeColors) {
    let area = centered_rect(80, 80, f.size());
    f.render_widget(ratatui::widgets::Clear, area);

    let constraints = vec![Constraint::Min(1), Constraint::Length(3)];

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let main_area = chunks[0];
    let search_box = chunks[1];

    let visible_height = main_area.height.saturating_sub(BORDER_PADDING_SIZE as u16) as usize;

    let start_idx = popup.scroll_offset;
    let end_idx = (popup.scroll_offset + visible_height).min(popup.filtered_titles.len());
    let visible_titles = &popup.filtered_titles[start_idx..end_idx];

    let items: Vec<Line> = visible_titles
        .iter()
        .enumerate()
        .map(|(i, title_match)| {
            let absolute_idx = i + popup.scroll_offset;
            if absolute_idx == popup.selected_index {
                Line::from(vec![Span::styled(
                    format!("> {}", title_match.title),
                    Style::default().fg(theme.accent),
                )])
            } else {
                Line::from(vec![Span::raw(format!("  {}", title_match.title))])
            }
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.primary))
        .title("Select Title");

    let paragraph = Paragraph::new(items)
        .block(block)
        .wrap(ratatui::widgets::Wrap { trim: true });

    f.render_widget(paragraph, main_area);

    let search_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.primary))
        .title("Search");

    let search_text = Paragraph::new(popup.search_query.as_str()).block(search_block);

    f.render_widget(search_text, search_box);
}

pub fn render_ui_popup(f: &mut Frame, popup: &UiPopup, theme: &ThemeColors) {
    if !popup.visible {
        return;
    }

    let area = centered_rect(popup.percent_x, popup.percent_y, f.size());
    f.render_widget(ratatui::widgets::Clear, area);

    let text = Paragraph::new(popup.message.as_str())
        .style(Style::default().fg(theme.error))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.error))
                .title(format!("{} - Esc to exit", popup.popup_title)),
        )
        .wrap(ratatui::widgets::Wrap { trim: true }); // Enable text wrapping

    f.render_widget(text, area);
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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

#[cfg(test)]
mod tests {
    use ratatui::{backend::TestBackend, Terminal};

    use crate::DARK_MODE_COLORS;
    use crate::ORANGE;

    use super::*;

    #[test]
    fn test_centered_rect() {
        let r = Rect::new(0, 0, 100, 100);
        let centered = centered_rect(50, 50, r);
        assert_eq!(centered.width, 50);
        assert_eq!(centered.height, 50);
        assert_eq!(centered.x, 25);
        assert_eq!(centered.y, 25);
    }

    #[test]
    fn test_render_header() {
        let backend = TestBackend::new(100, 1);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let area = f.size();
                render_header(f, area, false, &DARK_MODE_COLORS);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        assert!(buffer
            .content
            .iter()
            .any(|cell| cell.symbol().contains("Q")));
        assert!(buffer
            .content
            .iter()
            .any(|cell| cell.symbol().contains("u")));
        assert!(buffer
            .content
            .iter()
            .any(|cell| cell.symbol().contains("i")));
        assert!(buffer
            .content
            .iter()
            .any(|cell| cell.symbol().contains("t")));

        assert!(buffer.content.iter().any(|cell| cell.fg == ORANGE));
    }

    #[test]
    fn test_render_title_popup() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let popup = TitlePopup {
            title: "Test Title".to_string(),
            visible: true,
        };

        terminal
            .draw(|f| {
                render_title_popup(f, &popup, &DARK_MODE_COLORS);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        assert!(buffer
            .content
            .iter()
            .any(|cell| cell.symbol().contains("T")));

        assert!(buffer
            .content
            .iter()
            .any(|cell| cell.symbol().contains("e")));

        assert!(buffer
            .content
            .iter()
            .any(|cell| cell.symbol().contains("s")));

        assert!(buffer
            .content
            .iter()
            .any(|cell| cell.symbol().contains("t")));

        assert!(buffer
            .content
            .iter()
            .any(|cell| cell.symbol() == "─" || cell.symbol() == "│"));
    }

    #[test]
    fn test_render_title_select_popup() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut popup = TitleSelectPopup {
            titles: Vec::new(),
            selected_index: 0,
            visible: true,
            scroll_offset: 0,
            search_query: "".to_string(),
            filtered_titles: Vec::new(),
        };

        popup.set_titles(vec!["Title1".to_string(), "Title2".to_string()]);

        terminal
            .draw(|f| {
                render_title_select_popup(f, &popup, &DARK_MODE_COLORS);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        assert!(buffer
            .content
            .iter()
            .any(|cell| cell.symbol().contains(">")));
        assert!(buffer
            .content
            .iter()
            .any(|cell| cell.symbol().contains("2")));

        assert!(buffer
            .content
            .iter()
            .any(|cell| cell.symbol().contains("1")));
    }

    #[test]
    fn test_render_edit_commands_popup() {
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                render_edit_commands_popup(f, &DARK_MODE_COLORS);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        assert!(buffer
            .content
            .iter()
            .any(|cell| cell.symbol().contains("E")));

        assert!(buffer
            .content
            .iter()
            .any(|cell| cell.symbol().contains("H")));
        assert!(buffer
            .content
            .iter()
            .any(|cell| cell.symbol().contains("K")));

        assert!(buffer
            .content
            .iter()
            .any(|cell| cell.symbol().contains("I") && cell.fg == ORANGE));
    }
}
