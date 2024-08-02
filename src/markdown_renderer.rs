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
        if let Some(lines) = self.cache.get(&title) {
            return Ok(lines.clone());
        }

        let md_syntax = self.syntax_set.find_syntax_by_extension("md").unwrap();
        let mut lines = Vec::new();
        let mut in_code_block = false;
        let mut code_block_lang = String::new();
        let mut code_block_content = Vec::new();
        let theme = &self.theme_set.themes[&self.theme];
        let mut h = HighlightLines::new(md_syntax, theme);

        const HEADER_COLORS: [Color; 6] = [
            Color::Red,
            Color::Green,
            Color::Yellow,
            Color::Blue,
            Color::Magenta,
            Color::Cyan,
        ];

        // Check if the entire markdown is JSON
        if (markdown.trim_start().starts_with('{') || markdown.trim_start().starts_with('['))
            && (markdown.trim_end().ends_with('}') || markdown.trim_end().ends_with(']'))
        {
            let json_syntax = self.syntax_set.find_syntax_by_extension("json").unwrap();
            return Ok(Text::from(self.highlight_code_block(
                &markdown.lines().map(|x| x.to_string()).collect::<Vec<_>>(),
                "json",
                json_syntax,
                theme,
                width,
            )?));
        }

        let updated_markdown = markdown.clone();
        let mut markdown_lines = updated_markdown.lines().map(|x| x.to_string()).peekable();
        while let Some(line) = markdown_lines.next() {
            if line.starts_with("```") {
                if in_code_block {
                    // End of code block
                    let syntax = self
                        .syntax_set
                        .find_syntax_by_token(&code_block_lang)
                        .unwrap_or(md_syntax);
                    lines.extend(self.highlight_code_block(
                        &code_block_content.clone(),
                        &code_block_lang,
                        syntax,
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
                            // It's a one-line code block
                            let syntax = self
                                .syntax_set
                                .find_syntax_by_token(&code_block_lang)
                                .unwrap_or(md_syntax);
                            lines.extend(self.highlight_code_block(
                                &["".to_string()],
                                &code_block_lang,
                                syntax,
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
                let highlighted = h
                    .highlight_line(&line, &self.syntax_set)
                    .map_err(|e| anyhow!("Highlight error: {}", e))?;
                let mut spans: Vec<Span> = highlighted.into_iter().map(into_span).collect();

                // Optimized header handling
                if let Some(header_level) = line.bytes().position(|b| b != b'#') {
                    if header_level > 0
                        && header_level <= 6
                        && line.as_bytes().get(header_level) == Some(&b' ')
                    {
                        let header_color = HEADER_COLORS[header_level.saturating_sub(1)];
                        spans = vec![Span::styled(
                            line,
                            Style::default()
                                .fg(header_color)
                                .add_modifier(Modifier::BOLD),
                        )];
                    }
                }

                // Pad regular Markdown lines to full width
                let line_content: String =
                    spans.iter().map(|span| span.content.to_string()).collect();
                let padding_width = width.saturating_sub(line_content.len());
                if padding_width > 0 {
                    spans.push(Span::styled(" ".repeat(padding_width), Style::default()));
                }

                lines.push(Line::from(spans));
            }
        }

        let markdown_lines = Text::from(lines);
        self.cache.insert(title.clone(), markdown_lines.clone());
        Ok(markdown_lines)
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
        let line_num_width = max_line_num.to_string().len();

        if lang != "json" {
            result.push(Line::from(Span::styled(
                "─".repeat(width),
                Style::default().fg(Color::White),
            )));
        }

        for (line_number, line) in code.iter().enumerate() {
            let highlighted = h
                .highlight_line(line, &self.syntax_set)
                .map_err(|e| anyhow!("Highlight error: {}", e))?;

            let mut spans = if lang == "json" {
                vec![Span::styled(
                    format!("{:>width$} ", line_number + 1, width = line_num_width),
                    Style::default().fg(Color::White),
                )]
            } else {
                vec![Span::styled(
                    format!("{:>width$} │ ", line_number + 1, width = line_num_width),
                    Style::default().fg(Color::White),
                )]
            };
            spans.extend(highlighted.into_iter().map(into_span));

            // Pad the line to full width
            let line_content: String = spans.iter().map(|span| span.content.to_string()).collect();
            let padding_width = width.saturating_sub(line_content.len());
            if padding_width > 0 {
                spans.push(Span::styled(" ".repeat(padding_width), Style::default()));
            }

            result.push(Line::from(spans));
        }

        if lang != "json" {
            result.push(Line::from(Span::styled(
                "─".repeat(width),
                Style::default().fg(Color::White),
            )));
        }

        Ok(result)
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

fn into_span((style, text): (SyntectStyle, &str)) -> Span<'static> {
    Span::styled(text.to_string(), syntect_style_to_ratatui_style(style))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_markdown() {
        let mut renderer = MarkdownRenderer::new();
        let markdown = "# Header\n\nThis is **bold** and *italic* text.";
        let rendered = renderer
            .render_markdown(markdown.to_string(), "".to_string(), 40)
            .unwrap();

        assert!(rendered.lines.len() >= 3);
        assert!(rendered.lines[0]
            .spans
            .iter()
            .any(|span| span.content.contains("Header")));
        assert!(rendered.lines[2]
            .spans
            .iter()
            .any(|span| span.content.contains("This is")));
        assert!(rendered.lines[2]
            .spans
            .iter()
            .any(|span| span.content.contains("bold")));
        assert!(rendered.lines[2]
            .spans
            .iter()
            .any(|span| span.content.contains("italic")));
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
    fn test_render_markdown_with_one_line_code_block() {
        let mut renderer = MarkdownRenderer::new();
        let markdown = "# Header\n\n```rust\n```\n\nText after.".to_string();
        let rendered = renderer
            .render_markdown(markdown, "".to_string(), 40)
            .unwrap();

        assert!(rendered.lines.len() > 3);
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
}
