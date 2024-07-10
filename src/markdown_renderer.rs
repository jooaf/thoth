use std::mem;

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

pub struct MarkdownRenderer {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl Default for MarkdownRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownRenderer {
    pub fn new() -> Self {
        MarkdownRenderer {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    pub fn render_markdown(&self, markdown: String) -> Text {
        let mut rendered_lines = Vec::new();
        let mut options = Options::empty();
        options.insert(Options::ENABLE_STRIKETHROUGH);

        let parser = Parser::new_ext(&markdown, options);
        let mut current_line = Vec::new();
        let mut in_code_block = false;
        let mut code_block_lang = String::new();
        let mut code_block_content = String::new();
        let mut list_level = 0;
        let mut current_style = Style::default();

        for event in parser {
            match event {
                Event::Start(Tag::CodeBlock(kind)) => {
                    in_code_block = true;
                    code_block_lang = match kind {
                        CodeBlockKind::Fenced(lang) => lang.to_string(),
                        CodeBlockKind::Indented => String::new(),
                    };
                }
                Event::End(Tag::CodeBlock(_)) => {
                    let highlighted = self.highlight_code(&code_block_content, &code_block_lang);
                    rendered_lines.extend(highlighted);
                    in_code_block = false;
                    code_block_content.clear();
                }
                Event::Text(text) if in_code_block => {
                    code_block_content.push_str(&text);
                }
                Event::Start(Tag::Heading(level, _, _)) if !in_code_block => {
                    if level == HeadingLevel::H1 {
                        // Convert H1 to H2 within the content
                        current_style = self.header_style(HeadingLevel::H2);
                    } else {
                        current_style = self.header_style(level);
                    }
                }
                Event::End(Tag::Heading(_, _, _)) if !in_code_block => {
                    if !current_line.is_empty() {
                        rendered_lines.push(Line::from(mem::take(&mut current_line)));
                    }
                    current_style = Style::default();
                }
                Event::Start(Tag::List(_)) => {
                    list_level += 1;
                }
                Event::End(Tag::List(_)) => {
                    list_level = (list_level as u64).saturating_sub(1) as usize;
                }
                Event::Start(Tag::Item) => {
                    let indent = "  ".repeat(list_level - 1);
                    let bullet = format!("{}• ", indent);
                    current_line.push(Span::raw(bullet));
                }
                Event::Text(text) if !in_code_block => {
                    current_line.push(Span::styled(text.to_string(), current_style));
                }
                Event::SoftBreak | Event::HardBreak => {
                    if !current_line.is_empty() {
                        rendered_lines.push(Line::from(mem::take(&mut current_line)));
                    }
                }
                Event::Rule => {
                    rendered_lines.push(Line::from(Span::styled(
                        "─".repeat(40),
                        Style::default().fg(Color::DarkGray),
                    )));
                }
                Event::Start(Tag::Emphasis) => {
                    current_style = current_style.add_modifier(Modifier::ITALIC);
                }
                Event::End(Tag::Emphasis) => {
                    current_style = current_style.remove_modifier(Modifier::ITALIC);
                }
                Event::Start(Tag::Strong) => {
                    current_style = current_style.add_modifier(Modifier::BOLD);
                }
                Event::End(Tag::Strong) => {
                    current_style = current_style.remove_modifier(Modifier::BOLD);
                }
                Event::Start(Tag::Link(_, _, _)) => {
                    current_style = current_style
                        .fg(Color::Blue)
                        .add_modifier(Modifier::UNDERLINED);
                    current_line.push(Span::styled("[", current_style));
                }
                Event::End(Tag::Link(_, url, _)) => {
                    current_line.push(Span::styled("]", current_style));
                    current_line.push(Span::styled(
                        format!("({})", url),
                        Style::default().fg(Color::DarkGray),
                    ));
                    current_style = Style::default();
                }
                _ => {}
            }
        }

        if !current_line.is_empty() {
            rendered_lines.push(Line::from(current_line));
        }

        Text::from(rendered_lines)
    }

    fn highlight_code(&self, code: &str, lang: &str) -> Vec<Line> {
        let syntax = self
            .syntax_set
            .find_syntax_by_extension(lang)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
        let theme = &self.theme_set.themes["base16-ocean.dark"];
        let mut h = HighlightLines::new(syntax, theme);

        let mut lines = Vec::new();
        for (idx, line) in LinesWithEndings::from(code).enumerate() {
            let highlighted = h.highlight_line(line, &self.syntax_set).unwrap();
            let mut spans = Vec::new();

            // Add line number
            spans.push(Span::styled(
                format!("{:4} ", idx + 1),
                Style::default().fg(Color::DarkGray),
            ));

            // Add highlighted code
            spans.extend(highlighted.iter().map(|(style, text)| {
                let mut s = Style::default().fg(Color::Rgb(
                    style.foreground.r,
                    style.foreground.g,
                    style.foreground.b,
                ));
                if style.font_style.contains(FontStyle::BOLD) {
                    s = s.add_modifier(Modifier::BOLD);
                }
                if style.font_style.contains(FontStyle::ITALIC) {
                    s = s.add_modifier(Modifier::ITALIC);
                }
                if style.font_style.contains(FontStyle::UNDERLINE) {
                    s = s.add_modifier(Modifier::UNDERLINED);
                }
                Span::styled(text.to_string(), s)
            }));

            lines.push(Line::from(spans));
        }

        // Add a border around the code block
        let width = lines.iter().map(|l| l.width()).max().unwrap_or(0);
        let top_bottom_border = Line::from(Span::styled(
            "─".repeat(width + 2),
            Style::default().fg(Color::DarkGray),
        ));
        lines.insert(0, top_bottom_border.clone());
        lines.push(top_bottom_border);

        lines
    }

    fn header_style(&self, level: HeadingLevel) -> Style {
        match level {
            HeadingLevel::H1 => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            HeadingLevel::H2 => Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
            HeadingLevel::H3 => Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            HeadingLevel::H4 => Style::default()
                .fg(Color::Blue)
                .add_modifier(Modifier::BOLD),
            HeadingLevel::H5 => Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
            HeadingLevel::H6 => Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        }
    }
}
