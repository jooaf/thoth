use once_cell::sync::Lazy;
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use std::collections::HashMap;
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

const CODE_THEME: &str = "base16-eighties.dark";

static LANGUAGE_ALIASES: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("python", "py");
    m.insert("py", "py");
    m.insert("javascript", "js");
    m.insert("js", "js");
    m.insert("typescript", "ts");
    m.insert("ts", "ts");
    m.insert("typescript", "tsx");
    m.insert("tsx", "tsx");
    m.insert("csharp", "cs");
    m.insert("cs", "cs");
    m.insert("cpp", "cpp");
    m.insert("c++", "cpp");
    m.insert("rust", "rs");
    m.insert("rs", "rs");
    m.insert("go", "go");
    m.insert("golang", "go");
    m.insert("ruby", "rb");
    m.insert("rb", "rb");
    m.insert("java", "java");
    m.insert("kotlin", "kt");
    m.insert("kt", "kt");
    m.insert("swift", "swift");
    m.insert("objectivec", "m");
    m.insert("objc", "m");
    m.insert("scala", "scala");
    m.insert("html", "html");
    m.insert("css", "css");
    m.insert("php", "php");
    m.insert("shell", "sh");
    m.insert("bash", "sh");
    m.insert("sh", "sh");
    m.insert("yaml", "yaml");
    m.insert("yml", "yaml");
    m.insert("json", "json");
    m.insert("xml", "xml");
    m.insert("sql", "sql");
    m.insert("markdown", "md");
    m.insert("md", "md");
    // Add more languages and their aliases as needed
    m
});

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
                        pulldown_cmark::CodeBlockKind::Fenced(lang) => lang.to_string(),
                        pulldown_cmark::CodeBlockKind::Indented => String::new(),
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
                    if !current_line.is_empty() {
                        rendered_lines.push(Line::from(std::mem::take(&mut current_line)));
                    }
                    if level == HeadingLevel::H1 {
                        // Convert H1 to H2 within the content
                        current_style = self.header_style(HeadingLevel::H2);
                    } else {
                        current_style = self.header_style(level);
                    }
                }
                Event::End(Tag::Heading(_, _, _)) if !in_code_block => {
                    if !current_line.is_empty() {
                        rendered_lines.push(Line::from(std::mem::take(&mut current_line)));
                    }
                    rendered_lines.push(Line::default()); // Add an empty line after headers
                    current_style = Style::default();
                }
                Event::Start(Tag::List(_)) => {
                    list_level += 1;
                }
                Event::End(Tag::List(_)) => {
                    list_level = (list_level as u64).saturating_sub(1) as usize;
                    if !current_line.is_empty() {
                        rendered_lines.push(Line::from(std::mem::take(&mut current_line)));
                    }
                    rendered_lines.push(Line::default()); // Add an empty line after lists
                }
                Event::Start(Tag::Item) => {
                    if !current_line.is_empty() {
                        rendered_lines.push(Line::from(std::mem::take(&mut current_line)));
                    }
                    let indent = "  ".repeat(list_level - 1);
                    let bullet = format!("{}• ", indent);
                    current_line.push(Span::raw(bullet));
                }
                Event::Text(text) if !in_code_block => {
                    current_line.push(Span::styled(text.to_string(), current_style));
                }
                Event::SoftBreak => {
                    current_line.push(Span::raw(" "));
                }
                Event::HardBreak => {
                    if !current_line.is_empty() {
                        rendered_lines.push(Line::from(std::mem::take(&mut current_line)));
                    }
                }
                Event::Rule => {
                    if !current_line.is_empty() {
                        rendered_lines.push(Line::from(std::mem::take(&mut current_line)));
                    }
                    rendered_lines.push(Line::from(Span::styled(
                        "─".repeat(40),
                        Style::default().fg(Color::DarkGray),
                    )));
                    rendered_lines.push(Line::default()); // Add an empty line after horizontal rules
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
                Event::Start(Tag::Link(_, _url, _)) => {
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
                Event::End(Tag::Paragraph) => {
                    if !current_line.is_empty() {
                        rendered_lines.push(Line::from(std::mem::take(&mut current_line)));
                    }
                    rendered_lines.push(Line::default()); // Add an empty line after paragraphs
                }
                _ => {}
            }
        }

        if !current_line.is_empty() {
            rendered_lines.push(Line::from(current_line));
        }

        Text::from(rendered_lines)
    }

    pub fn format_code_block(&self, content: &str, lang: &str) -> String {
        match lang {
            "rust" => self.format_rust(content),
            "python" | "py" => self.format_python(content),
            "javascript" | "js" => self.format_javascript(content),
            _ => content.to_string(), // Return unformatted for unsupported languages
        }
    }

    fn format_rust(&self, content: &str) -> String {
        // Simple Rust formatting (indentation only)
        content
            .lines()
            .map(|line| {
                let trimmed = line.trim_start();
                let indent = " ".repeat(line.len() - trimmed.len());
                format!("{}{}\n", indent, trimmed)
            })
            .collect()
    }

    fn format_python(&self, content: &str) -> String {
        // Simple Python formatting (indentation only)
        content
            .lines()
            .map(|line| {
                let trimmed = line.trim_start();
                let indent = " ".repeat(line.len() - trimmed.len());
                format!("{}{}\n", indent, trimmed)
            })
            .collect()
    }

    fn format_javascript(&self, content: &str) -> String {
        // Simple JavaScript formatting (indentation only)
        content
            .lines()
            .map(|line| {
                let trimmed = line.trim_start();
                let indent = " ".repeat(line.len() - trimmed.len());
                format!("{}{}\n", indent, trimmed)
            })
            .collect()
    }

    pub fn format_markdown(&self, content: &str) -> String {
        let mut formatted = String::new();
        let mut options = Options::empty();
        options.insert(Options::ENABLE_STRIKETHROUGH);
        let parser = Parser::new_ext(content, options);
        let mut in_code_block = false;
        let mut code_block_lang = String::new();
        let mut code_block_content = String::new();

        for event in parser {
            match event {
                Event::Start(Tag::CodeBlock(kind)) => {
                    in_code_block = true;
                    code_block_lang = match kind {
                        pulldown_cmark::CodeBlockKind::Fenced(lang) => lang.to_string(),
                        pulldown_cmark::CodeBlockKind::Indented => String::new(),
                    };
                    formatted.push_str("```");
                    formatted.push_str(&code_block_lang);
                    formatted.push('\n');
                }
                Event::End(Tag::CodeBlock(_)) => {
                    if !code_block_content.is_empty() {
                        let formatted_code =
                            self.format_code_block(&code_block_content, &code_block_lang);
                        formatted.push_str(&formatted_code);
                    }
                    formatted.push_str("```\n");
                    in_code_block = false;
                    code_block_content.clear();
                }
                Event::Text(text) if in_code_block => {
                    code_block_content.push_str(&text);
                }
                Event::Start(tag) => self.handle_start_tag(&mut formatted, &tag),
                Event::End(tag) => self.handle_end_tag(&mut formatted, &tag),
                Event::Text(text) => formatted.push_str(&text),
                Event::Code(code) => {
                    formatted.push('`');
                    formatted.push_str(&code);
                    formatted.push('`');
                }
                Event::Html(html) => formatted.push_str(&html),
                Event::FootnoteReference(name) => {
                    formatted.push_str("[^");
                    formatted.push_str(&name);
                    formatted.push(']');
                }
                Event::SoftBreak => formatted.push('\n'),
                Event::HardBreak => formatted.push_str("\n\n"),
                Event::Rule => formatted.push_str("\n---\n"),
                Event::TaskListMarker(checked) => {
                    formatted.push_str(if checked { "[x]" } else { "[ ]" });
                }
            }
        }

        formatted
    }

    fn handle_start_tag(&self, formatted: &mut String, tag: &Tag) {
        match tag {
            Tag::Paragraph => formatted.push('\n'),
            Tag::Heading(level, _, _) => {
                formatted.push('\n');
                formatted.push_str(&"#".repeat(*level as usize));
                formatted.push(' ');
            }
            Tag::BlockQuote => formatted.push_str("> "),
            Tag::CodeBlock(_) => {} // Handled in the main loop
            Tag::List(None) => formatted.push_str("\n- "),
            Tag::List(Some(1)) => formatted.push_str("\n1. "),
            Tag::List(Some(start)) => formatted.push_str(&format!("\n{}. ", start)),
            Tag::Item => formatted.push_str("- "),
            Tag::Emphasis => formatted.push('*'),
            Tag::Strong => formatted.push_str("**"),
            Tag::Strikethrough => formatted.push_str("~~"),
            Tag::Link(_, _, _) => formatted.push('['),
            Tag::Image(_, _, _) => formatted.push('!'),
            _ => {}
        }
    }

    fn handle_end_tag(&self, formatted: &mut String, tag: &Tag) {
        match tag {
            Tag::Paragraph => formatted.push_str("\n\n"),
            Tag::Heading(_, _, _) => formatted.push_str("\n\n"),
            Tag::BlockQuote => formatted.push('\n'),
            Tag::CodeBlock(_) => {} // Handled in the main loop
            Tag::List(_) => formatted.push('\n'),
            Tag::Item => formatted.push('\n'),
            Tag::Emphasis => formatted.push('*'),
            Tag::Strong => formatted.push_str("**"),
            Tag::Strikethrough => formatted.push_str("~~"),
            Tag::Link(_, url, title) => {
                formatted.push_str(&format!("]({})", url));
                if !title.is_empty() {
                    formatted.push_str(&format!(" \"{}\"", title));
                }
            }
            Tag::Image(_, url, title) => {
                formatted.push_str(&format!("]({})", url));
                if !title.is_empty() {
                    formatted.push_str(&format!(" \"{}\"", title));
                }
            }
            _ => {}
        }
    }

    fn highlight_code(&self, code: &str, lang: &str) -> Vec<Line> {
        let extension = LANGUAGE_ALIASES
            .get(lang.to_lowercase().as_str())
            .copied()
            .unwrap_or(lang);

        let syntax = self
            .syntax_set
            .find_syntax_by_extension(extension)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
        let theme = &self.theme_set.themes[CODE_THEME];
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
