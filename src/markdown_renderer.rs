use anyhow::{anyhow, Result};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};
use syntect::{easy::HighlightLines, parsing::SyntaxSet, util::LinesWithEndings};
use syntect::{
    highlighting::{Style as SyntectStyle, ThemeSet},
    parsing::SyntaxReference,
};
// use syntect_tui::into_span;

pub struct MarkdownRenderer;

impl Default for MarkdownRenderer {
    fn default() -> Self {
        Self::new()
    }
}

fn highlight_code_block(
    code: &str,
    syntax: &SyntaxReference,
    ps: &SyntaxSet,
    theme: &syntect::highlighting::Theme,
    add_top_border: bool,
    width: usize,
) -> Result<Vec<Line<'static>>> {
    let mut h = HighlightLines::new(syntax, theme);
    let mut line_number = 1;
    let mut result = Vec::new();

    let max_line_num = code.lines().count();
    let line_num_width = max_line_num.to_string().len();

    // Add top border if needed
    if add_top_border {
        result.push(Line::from(Span::styled(
            "─".repeat(width),
            Style::default().fg(Color::White),
        )));
    }

    // Highlight code lines
    for line in LinesWithEndings::from(code) {
        let highlighted = h
            .highlight_line(line, ps)
            .map_err(|e| anyhow!("Highlight error: {}", e))?;
        let mut spans = vec![Span::styled(
            format!("{:>width$} │ ", line_number, width = line_num_width),
            Style::default().fg(Color::White),
        )];
        spans.extend(highlighted.into_iter().map(into_span));

        // Pad the line to full width
        let line_content: String = spans.iter().map(|span| span.content.clone()).collect();
        let padding_width = width.saturating_sub(line_content.len());
        if padding_width > 0 {
            spans.push(Span::styled(" ".repeat(padding_width), Style::default()));
        }

        result.push(Line::from(spans));
        line_number += 1;
    }

    // Add bottom border
    result.push(Line::from(Span::styled(
        "─".repeat(width),
        Style::default().fg(Color::White),
    )));

    Ok(result)
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

impl MarkdownRenderer {
    pub fn new() -> Self {
        MarkdownRenderer
    }

    pub fn render_markdown(&self, markdown: String, width: usize) -> Result<Text<'static>> {
        let ps = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();
        let md_syntax = ps.find_syntax_by_extension("md").unwrap();
        let mut lines = Vec::new();
        let mut in_code_block = false;
        let mut code_block_lang = String::new();
        let mut code_block_content = String::new();
        let mut is_first_code_block = true;
        let theme = &ts.themes["base16-mocha.dark"];
        // TODO make this a config option
        // Themes: `base16-ocean.dark`,`base16-eighties.dark`,`base16-mocha.dark`,`base16-ocean.light`
        let mut h = HighlightLines::new(md_syntax, theme);

        for line in markdown.lines() {
            if line.starts_with("```") {
                if in_code_block {
                    // End of code block
                    if let Some(syntax) = ps.find_syntax_by_token(&code_block_lang) {
                        lines.extend(highlight_code_block(
                            &code_block_content,
                            syntax,
                            &ps,
                            theme,
                            !is_first_code_block,
                            width,
                        )?);
                    } else {
                        // Fallback to plain text if language not recognized
                        lines.extend(highlight_code_block(
                            &code_block_content,
                            md_syntax,
                            &ps,
                            theme,
                            !is_first_code_block,
                            width,
                        )?);
                    }
                    code_block_content.clear();
                    in_code_block = false;
                    is_first_code_block = false;
                } else {
                    // Start of code block
                    in_code_block = true;
                    code_block_lang = line.trim_start_matches('`').to_string();
                }
            } else if in_code_block {
                code_block_content.push_str(line);
                code_block_content.push('\n');
            } else {
                let highlighted = h
                    .highlight_line(line, &ps)
                    .map_err(|e| anyhow!("Highlight error: {}", e))?;
                let mut spans: Vec<Span<'static>> =
                    highlighted.into_iter().map(into_span).collect();

                // Pad regular Markdown lines to full width
                let line_content: String = spans.iter().map(|span| span.content.clone()).collect();
                let padding_width = width.saturating_sub(line_content.len());
                if padding_width > 0 {
                    spans.push(Span::styled(" ".repeat(padding_width), Style::default()));
                }

                lines.push(Line::from(spans));
            }
        }

        Ok(Text::from(lines))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_markdown() {
        let renderer = MarkdownRenderer::new();
        let markdown = "# Header\n\nThis is **bold** and *italic* text.".to_string();
        let rendered = renderer.render_markdown(markdown, 40).unwrap();

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
        let renderer = MarkdownRenderer::new();
        let markdown = "# Header\n\n```rust\nfn main() {\n    println!(\"Hello, world!\");\n}\n```"
            .to_string();
        let rendered = renderer.render_markdown(markdown, 40).unwrap();
        println!("{:?}", rendered);

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
}
