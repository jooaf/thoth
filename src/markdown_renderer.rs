use std::collections::HashMap;

use anyhow::{anyhow, Result};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};
use syntect::{
    easy::HighlightLines,
    highlighting::{Style as SyntectStyle, ThemeSet},
    parsing::{SyntaxReference, SyntaxSet},
};

pub struct MarkdownRenderer {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    theme: String,
    cache: HashMap<String, Text<'static>>,
}
const HEADER_COLORS: [Color; 6] = [
    Color::Red,
    Color::Green,
    Color::Yellow,
    Color::Blue,
    Color::Magenta,
    Color::Cyan,
];

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
            theme: "base16-mocha.dark".to_string(),
            cache: HashMap::new(),
        }
    }

    pub fn render_markdown(
        &mut self,
        markdown: String,
        title: String,
        width: usize,
    ) -> Result<Text<'static>> {
        if let Some(lines) = self.cache.get(&format!("{}{}", &title, &markdown)) {
            return Ok(lines.clone());
        }

        let md_syntax = self.syntax_set.find_syntax_by_extension("md").unwrap();
        let mut lines = Vec::new();
        let mut in_code_block = false;
        let mut code_block_lang = String::new();
        let mut code_block_content = Vec::new();
        let theme = &self.theme_set.themes[&self.theme];
        let mut h = HighlightLines::new(md_syntax, theme);

        if self.is_json_document(&markdown) {
            let json_syntax = self.syntax_set.find_syntax_by_extension("json").unwrap();
            return Ok(Text::from(self.highlight_code_block(
                &markdown.lines().map(|x| x.to_string()).collect::<Vec<_>>(),
                "json",
                json_syntax,
                theme,
                width,
            )?));
        }

        let mut markdown_lines = markdown.lines().map(|x| x.to_string()).peekable();

        while let Some(line) = markdown_lines.next() {
            // Code block handling
            if line.starts_with("```") {
                if in_code_block {
                    // End of code block
                    lines.extend(self.process_code_block_end(
                        &code_block_content,
                        &code_block_lang,
                        md_syntax,
                        theme,
                        width,
                    )?);
                    code_block_content.clear();
                    in_code_block = false;
                } else {
                    // Start of code block
                    in_code_block = true;
                    code_block_lang = line.trim_start_matches('`').to_string();

                    // Check if it's a one-line code block
                    if let Some(next_line) = markdown_lines.peek() {
                        if next_line.starts_with("```") {
                            lines.extend(self.process_empty_code_block(
                                &code_block_lang,
                                md_syntax,
                                theme,
                                width,
                            )?);
                            in_code_block = false;
                            markdown_lines.next(); // Skip the closing ```
                            continue;
                        }
                    }
                }
            } else if in_code_block {
                code_block_content.push(line.to_string());
            } else {
                let processed_line = self.process_markdown_line(&line, &mut h, theme, width)?;
                lines.push(processed_line);
            }
        }

        let markdown_lines = Text::from(lines);
        let new_key = &format!("{}{}", &title, &markdown);
        self.cache.insert(new_key.clone(), markdown_lines.clone());
        Ok(markdown_lines)
    }

    fn is_json_document(&self, content: &str) -> bool {
        let trimmed = content.trim();
        (trimmed.starts_with('{') || trimmed.starts_with('['))
            && (trimmed.ends_with('}') || trimmed.ends_with(']'))
    }

    fn process_code_block_end(
        &self,
        code_content: &[String],
        lang: &str,
        default_syntax: &SyntaxReference,
        theme: &syntect::highlighting::Theme,
        width: usize,
    ) -> Result<Vec<Line<'static>>> {
        let lang = lang.trim_start_matches('`').trim();
        let syntax = if !lang.is_empty() {
            self.syntax_set
                .find_syntax_by_token(lang)
                .or_else(|| self.syntax_set.find_syntax_by_extension(lang))
                .unwrap_or(default_syntax)
        } else {
            default_syntax
        };

        self.highlight_code_block(code_content, lang, syntax, theme, width)
    }

    fn process_empty_code_block(
        &self,
        lang: &str,
        default_syntax: &SyntaxReference,
        theme: &syntect::highlighting::Theme,
        width: usize,
    ) -> Result<Vec<Line<'static>>> {
        let lang = lang.trim();
        let syntax = if !lang.is_empty() {
            self.syntax_set
                .find_syntax_by_token(lang)
                .or_else(|| self.syntax_set.find_syntax_by_extension(lang))
                .unwrap_or(default_syntax)
        } else {
            default_syntax
        };

        self.highlight_code_block(&["".to_string()], lang, syntax, theme, width)
    }

    fn highlight_code_block(
        &self,
        code: &[String],
        lang: &str,
        syntax: &SyntaxReference,
        theme: &syntect::highlighting::Theme,
        width: usize,
    ) -> Result<Vec<Line<'static>>> {
        let mut h = HighlightLines::new(syntax, theme);
        let mut result = Vec::new();

        let max_line_num = code.len();
        let line_num_width = max_line_num.to_string().len().max(1);

        let lang_name = lang.trim();
        let header_text = if !lang_name.is_empty() {
            format!("▌ {} ", lang_name)
        } else {
            "▌ code ".to_string()
        };

        let border_width = width.saturating_sub(header_text.len());
        let header = Span::styled(
            format!("{}{}", header_text, "─".repeat(border_width)),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );

        if lang != "json" {
            result.push(Line::from(vec![header]));
        }

        for (line_number, line) in code.iter().enumerate() {
            let highlighted = h
                .highlight_line(line, &self.syntax_set)
                .map_err(|e| anyhow!("Highlight error: {}", e))?;

            let mut spans = if lang == "json" {
                vec![Span::styled(
                    format!("{:>width$} ", line_number + 1, width = line_num_width),
                    Style::default().fg(Color::DarkGray),
                )]
            } else {
                vec![Span::styled(
                    format!("{:>width$} │ ", line_number + 1, width = line_num_width),
                    Style::default().fg(Color::DarkGray),
                )]
            };
            spans.extend(self.process_syntect_highlights(highlighted));

            let line_content: String = spans.iter().map(|span| span.content.clone()).collect();
            let padding_width = width.saturating_sub(line_content.len());
            if padding_width > 0 {
                spans.push(Span::styled(" ".repeat(padding_width), Style::default()));
            }

            result.push(Line::from(spans));
        }

        if lang != "json" {
            result.push(Line::from(Span::styled(
                "─".repeat(width),
                Style::default().fg(Color::DarkGray),
            )));
        }

        Ok(result)
    }

    fn process_markdown_line(
        &self,
        line: &str,
        h: &mut HighlightLines,
        _theme: &syntect::highlighting::Theme,
        width: usize,
    ) -> Result<Line<'static>> {
        let mut spans: Vec<Span<'static>>;

        // Handle header
        if let Some((is_header, level)) = self.is_header(line) {
            if is_header {
                let header_color = if level <= 6 {
                    HEADER_COLORS[level.saturating_sub(1)]
                } else {
                    HEADER_COLORS[0]
                };

                spans = vec![Span::styled(
                    line.to_string(),
                    Style::default()
                        .fg(header_color)
                        .add_modifier(Modifier::BOLD),
                )];
                return Ok(Line::from(spans));
            }
        }

        let (content, is_blockquote) = self.process_blockquote(line);

        if let Some((content, is_checked)) = self.is_checkbox_list_item(&content) {
            return self.format_checkbox_item(line, content, is_checked, h, width);
        }

        let (content, is_list, is_ordered, order_num) = self.process_list_item(&content);

        let highlighted = h
            .highlight_line(&content, &self.syntax_set)
            .map_err(|e| anyhow!("Highlight error: {}", e))?;

        spans = self.process_syntect_highlights(highlighted);

        if is_blockquote {
            spans = self.apply_blockquote_styling(spans);
        }

        if is_list {
            spans = self.apply_list_styling(line, spans, is_ordered, order_num);
        } else {
            let whitespace_prefix = line
                .chars()
                .take_while(|c| c.is_whitespace())
                .collect::<String>();

            if !whitespace_prefix.is_empty() {
                spans.insert(0, Span::styled(whitespace_prefix, Style::default()));
            }
        }

        let line_content: String = spans.iter().map(|span| span.content.clone()).collect();
        let padding_width = width.saturating_sub(line_content.len());
        if padding_width > 0 {
            spans.push(Span::styled(" ".repeat(padding_width), Style::default()));
        }

        Ok(Line::from(spans))
    }

    fn is_header(&self, line: &str) -> Option<(bool, usize)> {
        if let Some(header_level) = line.bytes().position(|b| b != b'#') {
            if header_level > 0
                && header_level <= 6
                && line.as_bytes().get(header_level) == Some(&b' ')
            {
                return Some((true, header_level));
            }
        }
        None
    }

    fn process_blockquote(&self, line: &str) -> (String, bool) {
        if line.starts_with('>') {
            let content = line.trim_start_matches('>').trim_start().to_string();
            (content, true)
        } else {
            (line.to_string(), false)
        }
    }

    fn is_checkbox_list_item(&self, line: &str) -> Option<(String, bool)> {
        let trimmed = line.trim_start();

        if trimmed.starts_with("- [ ]")
            || trimmed.starts_with("+ [ ]")
            || trimmed.starts_with("* [ ]")
        {
            let content = trimmed[5..].to_string();
            return Some((content, false)); // Unchecked
        } else if trimmed.starts_with("- [x]")
            || trimmed.starts_with("- [X]")
            || trimmed.starts_with("+ [x]")
            || trimmed.starts_with("+ [X]")
            || trimmed.starts_with("* [x]")
            || trimmed.starts_with("* [X]")
        {
            let content = trimmed[5..].to_string();
            return Some((content, true)); // Checked
        }

        // Also match "- [ x ]" or "- [  ]" style with extra spaces
        if let Some(list_marker_pos) = ["- [", "+ [", "* ["].iter().find_map(|marker| {
            if trimmed.starts_with(marker) {
                Some(marker.len())
            } else {
                None
            }
        }) {
            if trimmed.len() > list_marker_pos {
                let remaining = &trimmed[list_marker_pos..];
                if remaining.starts_with("  ]") || remaining.starts_with(" ]") {
                    let content_start = remaining
                        .find(']')
                        .map(|pos| list_marker_pos + pos + 1)
                        .unwrap_or(list_marker_pos);

                    if content_start < trimmed.len() {
                        let content = trimmed[content_start + 1..].to_string();
                        return Some((content, false));
                    }
                } else if remaining.starts_with(" x ]")
                    || remaining.starts_with(" X ]")
                    || remaining.starts_with("x ]")
                    || remaining.starts_with("X ]")
                {
                    let content_start = remaining
                        .find(']')
                        .map(|pos| list_marker_pos + pos + 1)
                        .unwrap_or(list_marker_pos);

                    if content_start < trimmed.len() {
                        let content = trimmed[content_start + 1..].to_string();
                        return Some((content, true));
                    }
                }
            }
        }

        None
    }

    fn format_checkbox_item(
        &self,
        line: &str,
        content: String,
        is_checked: bool,
        h: &mut HighlightLines,
        width: usize,
    ) -> Result<Line<'static>> {
        let whitespace_prefix = line
            .chars()
            .take_while(|c| c.is_whitespace())
            .collect::<String>();

        let checkbox = if is_checked {
            Span::styled("[X] ".to_string(), Style::default().fg(Color::Green))
        } else {
            Span::styled("[ ] ".to_string(), Style::default().fg(Color::Gray))
        };

        let highlighted = h
            .highlight_line(&content, &self.syntax_set)
            .map_err(|e| anyhow!("Highlight error: {}", e))?;

        let mut content_spans = self.process_syntect_highlights(highlighted);

        let mut spans = vec![Span::styled(whitespace_prefix, Style::default()), checkbox];
        spans.append(&mut content_spans);

        let line_content: String = spans.iter().map(|span| span.content.clone()).collect();
        let padding_width = width.saturating_sub(line_content.len());
        if padding_width > 0 {
            spans.push(Span::styled(" ".repeat(padding_width), Style::default()));
        }

        Ok(Line::from(spans))
    }

    fn process_list_item(&self, line: &str) -> (String, bool, bool, usize) {
        let trimmed = line.trim_start();

        if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ") {
            let content = trimmed[2..].to_string();
            return (content, true, false, 0);
        }

        if let Some(dot_pos) = trimmed.find(". ") {
            if dot_pos > 0 && trimmed[..dot_pos].chars().all(|c| c.is_ascii_digit()) {
                let order_num = trimmed[..dot_pos].parse::<usize>().unwrap_or(1);
                let content = trimmed[(dot_pos + 2)..].to_string();
                return (content, true, true, order_num);
            }
        }

        (line.to_string(), false, false, 0)
    }

    fn apply_blockquote_styling<'a>(&self, spans: Vec<Span<'a>>) -> Vec<Span<'a>> {
        let mut result = vec![Span::styled(
            "▎ ".to_string(),
            Style::default().fg(Color::Blue),
        )];

        for span in spans {
            result.push(Span::styled(span.content, Style::default().fg(Color::Gray)));
        }

        result
    }

    fn apply_list_styling<'a>(
        &self,
        original_line: &str,
        spans: Vec<Span<'a>>,
        is_ordered: bool,
        order_num: usize,
    ) -> Vec<Span<'a>> {
        let whitespace_prefix = original_line
            .chars()
            .take_while(|c| c.is_whitespace())
            .collect::<String>();

        let list_marker = if is_ordered {
            format!("{}. ", order_num)
        } else {
            "• ".to_string()
        };

        let prefix = Span::styled(
            format!("{}{}", whitespace_prefix, list_marker),
            Style::default().fg(Color::Yellow),
        );

        let mut result = vec![prefix];
        result.extend(spans);
        result
    }

    fn process_syntect_highlights(
        &self,
        highlighted: Vec<(SyntectStyle, &str)>,
    ) -> Vec<Span<'static>> {
        let mut spans = Vec::new();

        for (style, text) in highlighted {
            let text_owned = text.to_string();

            if text_owned.contains("~~") && text_owned.matches("~~").count() >= 2 {
                self.process_strikethrough(&text_owned, style, &mut spans);
                continue;
            }

            if text_owned.contains('`') && !text_owned.contains("```") {
                self.process_inline_code(&text_owned, style, &mut spans);
                continue;
            }

            if text_owned.contains('[')
                && text_owned.contains(']')
                && text_owned.contains('(')
                && text_owned.contains(')')
            {
                self.process_links(&text_owned, style, &mut spans);
                continue;
            }

            spans.push(Span::styled(
                text_owned,
                syntect_style_to_ratatui_style(style),
            ));
        }

        spans
    }

    fn process_strikethrough(
        &self,
        text: &str,
        style: SyntectStyle,
        spans: &mut Vec<Span<'static>>,
    ) {
        let parts: Vec<&str> = text.split("~~").collect();
        let mut in_strikethrough = false;

        for (i, part) in parts.iter().enumerate() {
            if !part.is_empty() {
                if in_strikethrough {
                    spans.push(Span::styled(
                        part.to_string(),
                        syntect_style_to_ratatui_style(style).add_modifier(Modifier::CROSSED_OUT),
                    ));
                } else {
                    spans.push(Span::styled(
                        part.to_string(),
                        syntect_style_to_ratatui_style(style),
                    ));
                }
            }

            if i < parts.len() - 1 {
                in_strikethrough = !in_strikethrough;
            }
        }
    }

    fn process_inline_code(&self, text: &str, style: SyntectStyle, spans: &mut Vec<Span<'static>>) {
        let parts: Vec<&str> = text.split('`').collect();
        let mut in_code = false;

        for (i, part) in parts.iter().enumerate() {
            if !part.is_empty() {
                if in_code {
                    spans.push(Span::styled(
                        part.to_string(),
                        Style::default().fg(Color::White).bg(Color::DarkGray),
                    ));
                } else {
                    spans.push(Span::styled(
                        part.to_string(),
                        syntect_style_to_ratatui_style(style),
                    ));
                }
            }

            if i < parts.len() - 1 {
                in_code = !in_code;
            }
        }
    }

    fn process_links(&self, text: &str, style: SyntectStyle, spans: &mut Vec<Span<'static>>) {
        let mut in_link = false;
        let mut in_url = false;
        let mut current_text = String::new();
        let mut link_text = String::new();

        let mut i = 0;
        let chars: Vec<char> = text.chars().collect();

        while i < chars.len() {
            match chars[i] {
                '[' => {
                    if !in_link && !in_url {
                        // Add any text before the link
                        if !current_text.is_empty() {
                            spans.push(Span::styled(
                                current_text.clone(),
                                syntect_style_to_ratatui_style(style),
                            ));
                            current_text.clear();
                        }
                        in_link = true;
                    } else {
                        current_text.push('[');
                    }
                }
                ']' => {
                    if in_link && !in_url {
                        link_text = current_text.clone();
                        current_text.clear();
                        in_link = false;

                        // Check if next char is '('
                        if i + 1 < chars.len() && chars[i + 1] == '(' {
                            in_url = true;
                            i += 1; // Skip the opening paren
                        } else {
                            // Not a proper link, just show the text with brackets
                            spans.push(Span::styled(
                                format!("[{}]", link_text),
                                syntect_style_to_ratatui_style(style),
                            ));
                            link_text.clear();
                        }
                    } else {
                        current_text.push(']');
                    }
                }
                ')' => {
                    if in_url {
                        // URL part is in current_text, link text is in link_text
                        in_url = false;

                        spans.push(Span::styled(
                            link_text.clone(),
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::UNDERLINED),
                        ));

                        link_text.clear();
                        current_text.clear();
                    } else {
                        current_text.push(')');
                    }
                }
                _ => {
                    current_text.push(chars[i]);
                }
            }

            i += 1;
        }

        if !current_text.is_empty() {
            spans.push(Span::styled(
                current_text,
                syntect_style_to_ratatui_style(style),
            ));
        }
    }
}

fn syntect_style_to_ratatui_style(style: SyntectStyle) -> Style {
    let mut ratatui_style = Style::default().fg(Color::Rgb(
        style.foreground.r,
        style.foreground.g,
        style.foreground.b,
    ));

    if style
        .font_style
        .contains(syntect::highlighting::FontStyle::BOLD)
    {
        ratatui_style = ratatui_style.add_modifier(Modifier::BOLD);
    }
    if style
        .font_style
        .contains(syntect::highlighting::FontStyle::ITALIC)
    {
        ratatui_style = ratatui_style.add_modifier(Modifier::ITALIC);
    }
    if style
        .font_style
        .contains(syntect::highlighting::FontStyle::UNDERLINE)
    {
        ratatui_style = ratatui_style.add_modifier(Modifier::UNDERLINED);
    }

    ratatui_style
}

#[cfg(test)]
mod tests {
    use crate::MIN_TEXTAREA_HEIGHT;

    use super::*;

    #[test]
    fn test_render_markdown() {
        let mut renderer = MarkdownRenderer::new();
        let markdown = "# Header\n\nThis is **bold** and *italic* text.";
        let rendered = renderer
            .render_markdown(markdown.to_string(), "".to_string(), 40)
            .unwrap();

        assert!(rendered.lines.len() >= MIN_TEXTAREA_HEIGHT);
        assert!(rendered.lines[0]
            .spans
            .iter()
            .any(|span| span.content.contains("Header")));
        assert!(rendered.lines[2]
            .spans
            .iter()
            .any(|span| span.content.contains("This is")));
    }

    #[test]
    fn test_render_markdown_with_code_block() {
        let mut renderer = MarkdownRenderer::new();
        let markdown = "# Header\n\n```rust\nfn main() {\n    println!(\"Hello, world!\");\n}\n```";

        let rendered = renderer
            .render_markdown(markdown.to_string(), "".to_string(), 40)
            .unwrap();
        assert!(rendered.lines.len() > 5);
        assert!(rendered.lines[0]
            .spans
            .iter()
            .any(|span| span.content.contains("Header")));
        assert!(rendered
            .lines
            .iter()
            .any(|line| line.spans.iter().any(|span| span.content.contains("main"))));
    }

    #[test]
    fn test_render_json() {
        let mut renderer = MarkdownRenderer::new();
        let json = r#"{
  "name": "John Doe",
  "age": 30,
  "city": "New York"
}"#;

        let rendered = renderer
            .render_markdown(json.to_string(), "".to_string(), 40)
            .unwrap();

        assert!(rendered.lines.len() == 5);
        assert!(rendered.lines[0]
            .spans
            .iter()
            .any(|span| span.content.contains("{")));
        assert!(rendered.lines[4]
            .spans
            .iter()
            .any(|span| span.content.contains("}")));
    }

    #[test]
    fn test_render_markdown_with_lists() {
        let mut renderer = MarkdownRenderer::new();
        let markdown =
            "# List Test\n\n- Item 1\n- Item 2\n  - Nested item\n\n1. First item\n2. Second item";
        let rendered = renderer
            .render_markdown(markdown.to_string(), "".to_string(), 40)
            .unwrap();

        assert!(rendered
            .lines
            .iter()
            .any(|line| line.spans.iter().any(|span| span.content.contains("•"))));
        assert!(rendered
            .lines
            .iter()
            .any(|line| line.spans.iter().any(|span| span.content.contains("1."))));
    }

    #[test]
    fn test_render_markdown_with_links() {
        let mut renderer = MarkdownRenderer::new();
        let markdown = "Visit [Google](https://google.com) for search";
        let rendered = renderer
            .render_markdown(markdown.to_string(), "".to_string(), 40)
            .unwrap();

        assert!(rendered.lines.iter().any(|line| line
            .spans
            .iter()
            .any(|span| span.content.contains("Google"))));
    }

    #[test]
    fn test_render_markdown_with_blockquotes() {
        let mut renderer = MarkdownRenderer::new();
        let markdown = "> This is a blockquote\n> Another line";
        let rendered = renderer
            .render_markdown(markdown.to_string(), "".to_string(), 40)
            .unwrap();

        assert!(rendered
            .lines
            .iter()
            .any(|line| line.spans.iter().any(|span| span.content.contains("▎"))));
    }

    #[test]
    fn test_render_markdown_with_task_lists() {
        let mut renderer = MarkdownRenderer::new();
        let markdown = "- [ ] Unchecked task\n- [x] Checked task\n- [ x ] Also checked task\n- [  ] Another unchecked task";
        let rendered = renderer
            .render_markdown(markdown.to_string(), "".to_string(), 40)
            .unwrap();

        assert!(rendered
            .lines
            .iter()
            .any(|line| line.spans.iter().any(|span| span.content.contains("[ ]"))));
        assert!(rendered
            .lines
            .iter()
            .any(|line| line.spans.iter().any(|span| span.content.contains("[X]"))));
    }

    #[test]
    fn test_render_markdown_with_inline_code() {
        let mut renderer = MarkdownRenderer::new();
        let markdown = "Some `inline code` here";
        let rendered = renderer
            .render_markdown(markdown.to_string(), "".to_string(), 40)
            .unwrap();

        assert!(rendered.lines.iter().any(|line| line
            .spans
            .iter()
            .any(|span| span.content.contains("inline code"))));
    }

    #[test]
    fn test_render_markdown_with_strikethrough() {
        let mut renderer = MarkdownRenderer::new();
        let markdown = "This is ~~strikethrough~~ text";
        let rendered = renderer
            .render_markdown(markdown.to_string(), "".to_string(), 40)
            .unwrap();

        let has_strikethrough = rendered.lines.iter().any(|line| {
            line.spans.iter().any(|span| {
                let modifiers = span.style.add_modifier;
                return modifiers.contains(Modifier::CROSSED_OUT);
            })
        });

        assert!(has_strikethrough);
    }

    #[test]
    fn test_render_markdown_with_one_line_code_block() {
        let mut renderer = MarkdownRenderer::new();
        let markdown = "# Header\n\n```rust\n```\n\nText after.".to_string();
        let rendered = renderer
            .render_markdown(markdown, "".to_string(), 40)
            .unwrap();

        assert!(rendered.lines.len() > MIN_TEXTAREA_HEIGHT);
        assert!(rendered.lines[0]
            .spans
            .iter()
            .any(|span| span.content.contains("Header")));
        assert!(rendered
            .lines
            .iter()
            .any(|line| line.spans.iter().any(|span| span.content.contains("1 │"))));
        assert!(rendered
            .lines
            .last()
            .unwrap()
            .spans
            .iter()
            .any(|span| span.content.contains("Text after.")));
    }

    #[test]
    fn test_indentation_preservation() {
        let mut renderer = MarkdownRenderer::new();
        let markdown = "Regular text\n    Indented text\n        Double indented text";
        let rendered = renderer
            .render_markdown(markdown.to_string(), "".to_string(), 50)
            .unwrap();

        assert_eq!(rendered.lines.len(), 3);

        assert!(rendered.lines[1]
            .spans
            .iter()
            .any(|span| span.content.starts_with("    ")));

        assert!(rendered.lines[2]
            .spans
            .iter()
            .any(|span| span.content.starts_with("        ")));
    }
}
