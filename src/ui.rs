use crate::{TitlePopup, TitleSelectPopup, ORANGE};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs, Widget},
    Frame,
};

pub fn render_header(f: &mut Frame, area: Rect) {
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

pub fn render_title_popup(f: &mut Frame, popup: &TitlePopup) {
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

pub fn render_title_select_popup(f: &mut Frame, popup: &TitleSelectPopup) {
    let area = centered_rect(60, 60, f.size());
    f.render_widget(ratatui::widgets::Clear, area);

    let items: Vec<Line> = popup
        .titles
        .iter()
        .enumerate()
        .map(|(i, title)| {
            if i == popup.selected_index {
                Line::from(vec![Span::styled(
                    format!("‚Äö√±‚àÇ {}", title),
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
}
