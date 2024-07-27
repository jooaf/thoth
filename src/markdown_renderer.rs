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

pub struct MarkdownRenderer;

impl Default for MarkdownRenderer {
    fn default() -> Self {
        Self::new()
    }
}

fn highlight_code_block(
    code: &str,
    lang: &str,
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
    // done to create the proper spacing after the line number
    let line_num_width = max_line_num.to_string().len();
    // add top border if needed
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

        let mut spans = if lang != "json" {
            vec![Span::styled(
                format!("{:>width$} │ ", line_number, width = line_num_width),
                Style::default().fg(Color::White),
            )]
        // if json no need to render lines
        // TODO: find a way of making lines flush for syntax highlighting of json. not critical tho
        } else {
            vec![Span::styled(
                format!("{:>width$}  ", line_number, width = line_num_width),
                Style::default().fg(Color::White),
            )]
        };
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

    pub fn render_markdown<'a>(&self, markdown: &'a str, width: usize) -> Result<Text<'a>> {
        let ps = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();
        let md_syntax = ps.find_syntax_by_extension("md").unwrap();
        let mut lines = Vec::new();
        let mut in_code_block = false;
        let mut code_block_lang = String::new();
        let mut code_block_content = String::new();
        let mut is_first_code_block = true;
        let mut json_start = false;
        let mut start_del = "".to_string();
        let theme = &ts.themes["base16-mocha.dark"];
        // TODO make this a config option
        // Themes: `base16-ocean.dark`,`base16-eighties.dark`,`base16-mocha.dark`,`base16-ocean.light`
        let mut h = HighlightLines::new(md_syntax, theme);

        const HEADER_COLORS: [Color; 6] = [
            Color::Red,
            Color::Green,
            Color::Yellow,
            Color::Blue,
            Color::Magenta,
            Color::Cyan,
        ];

        let max_num_lines = markdown.lines().count() - 1; // first line will be used for code line

        for (index, line) in markdown.lines().enumerate() {
            // when the index finally has reached the last line, finishing the highlighting
            let reached_end = index == max_num_lines;

            // TODO: Assumption here is that this will be rendered if JSON is inputted.
            // might want to change to be more flexible
            // this is for the json rendering
            if line.starts_with('{') || line.starts_with('[') {
                start_del = line.chars().next().unwrap().to_string();
                json_start = true;
                in_code_block = true;

                // when the json is only one line
                if max_num_lines == 0 {
                    code_block_content.push_str(line);
                    code_block_content.push('\n');
                }
            }

            if json_start && in_code_block && reached_end {
                // TODO: ugly clean up
                // TODO: You will need to take into badly formatted json that has a ton of spaces in between keys
                if reached_end && index != 0 {
                    let end_del = if start_del == *"{" {
                        "}".to_string()
                    } else {
                        "]".to_string()
                    };

                    code_block_content.push_str(&end_del);
                    code_block_content.push('\n');
                }

                let syntax = ps.find_syntax_by_extension("json").unwrap();
                lines.extend(highlight_code_block(
                    &code_block_content,
                    "json",
                    syntax,
                    &ps,
                    theme,
                    false,
                    width,
                )?);

                json_start = false;
                in_code_block = false;
                code_block_content.clear();
                is_first_code_block = false;
            }

            if line.starts_with("```") {
                if in_code_block {
                    // End of code block
                    if let Some(syntax) = ps.find_syntax_by_token(&code_block_lang) {
                        lines.extend(highlight_code_block(
                            &code_block_content,
                            &code_block_lang,
                            syntax,
                            &ps,
                            theme,
                            !is_first_code_block || index != 0,
                            width,
                        )?);
                    } else {
                        // Fallback to plain text if language not recognized
                        lines.extend(highlight_code_block(
                            &code_block_content,
                            &code_block_lang,
                            md_syntax,
                            &ps,
                            theme,
                            !is_first_code_block || index != 0,
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
                let mut spans = highlighted.into_iter().map(into_span).collect();

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
        let rendered = renderer.render_markdown(&markdown, 40).unwrap();

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
        let rendered = renderer.render_markdown(&markdown, 40).unwrap();
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
