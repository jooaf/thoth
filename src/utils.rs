use crate::code_block_popup::CodeBlock;
use anyhow::Result;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use tui_textarea::TextArea;

/// Notes file parsed into blocks of `(title, lines)`.
/// `preamble_lines` holds any content found before the first block header
/// so it can be preserved when the file is rewritten.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ParsedNotes {
    pub preamble_lines: Vec<String>,
    pub blocks: Vec<(String, Vec<String>)>,
}

fn unescape_line(line: &str) -> String {
    match line.strip_prefix("\\#") {
        Some(rest) => format!("#{}", rest),
        None => line.to_string(),
    }
}

fn escape_line(line: &str, in_code_block: bool) -> String {
    if in_code_block || !line.starts_with('#') {
        line.to_string()
    } else {
        format!("\\{}", line)
    }
}

/// Parse the notes file format into blocks. Lines starting with `# ` (outside
/// code blocks) start a new block; the escaped form `\# ` is kept as content.
pub fn parse_notes(content: &str) -> ParsedNotes {
    let mut parsed = ParsedNotes::default();
    let mut current_title = String::new();
    let mut current_lines: Vec<String> = Vec::new();
    let mut in_code_block = false;
    let mut seen_title = false;

    for line in content.lines() {
        if line.trim().starts_with("```") {
            in_code_block = !in_code_block;
        }
        if !in_code_block && line.starts_with("# ") {
            if seen_title && !current_title.is_empty() {
                parsed
                    .blocks
                    .push((current_title.clone(), std::mem::take(&mut current_lines)));
            }
            seen_title = true;
            current_title = line[2..].to_string();
        } else if seen_title {
            current_lines.push(unescape_line(line));
        } else {
            parsed.preamble_lines.push(line.to_string());
        }
    }

    if seen_title && !current_title.is_empty() {
        parsed.blocks.push((current_title, current_lines));
    }

    parsed
}

/// Write a `ParsedNotes` back to a writer, escaping content lines that start
/// with `#` so they are not mistaken for block headers on reload.
pub fn write_notes<W: Write>(writer: &mut W, parsed: &ParsedNotes) -> Result<()> {
    for line in &parsed.preamble_lines {
        writeln!(writer, "{}", line)?;
    }
    let mut in_code_block = false;
    for (title, lines) in &parsed.blocks {
        writeln!(writer, "# {}", title)?;
        for line in lines {
            writeln!(writer, "{}", escape_line(line, in_code_block))?;
            if line.trim().starts_with("```") {
                in_code_block = !in_code_block;
            }
        }
    }
    Ok(())
}

pub fn extract_code_blocks(content: &str) -> Vec<CodeBlock> {
    let mut code_blocks = Vec::new();
    let mut in_code_block = false;
    let mut current_block = String::new();
    let mut current_language = String::new();
    let mut start_line = 0;

    for (i, line) in content.lines().enumerate() {
        if line.trim().starts_with("```") {
            if in_code_block {
                // End of code block
                code_blocks.push(CodeBlock::new(
                    current_block.trim_end().to_string(),
                    current_language.clone(),
                    start_line,
                    i,
                ));
                current_block.clear();
                current_language.clear();
                in_code_block = false;
            } else {
                // Start of code block
                in_code_block = true;
                start_line = i;

                let lang_part = line.trim_start_matches('`').trim();
                current_language = lang_part.to_string();
            }
        } else if in_code_block {
            current_block.push_str(line);
            current_block.push('\n');
        }
    }

    if in_code_block && !current_block.is_empty() {
        code_blocks.push(CodeBlock::new(
            current_block.trim_end().to_string(),
            current_language,
            start_line,
            content.lines().count() - 1,
        ));
    }

    code_blocks
}

pub fn save_textareas(textareas: &[TextArea], titles: &[String], file_path: PathBuf) -> Result<()> {
    let parsed = ParsedNotes {
        preamble_lines: Vec::new(),
        blocks: textareas
            .iter()
            .zip(titles.iter())
            .map(|(textarea, title)| (title.clone(), textarea.lines().to_vec()))
            .collect(),
    };
    let mut file = File::create(file_path)?;
    write_notes(&mut file, &parsed)
}

pub fn load_textareas(file_path: PathBuf) -> Result<(Vec<TextArea<'static>>, Vec<String>)> {
    let content = std::fs::read_to_string(file_path)?;
    let parsed = parse_notes(&content);
    let mut textareas = Vec::with_capacity(parsed.blocks.len().max(1));
    let mut titles = Vec::with_capacity(parsed.blocks.len().max(1));

    // Preserve content that appears before the first block header instead of
    // silently discarding it.
    if !parsed.preamble_lines.is_empty() {
        let mut preamble_textarea = TextArea::default();
        for line in &parsed.preamble_lines {
            preamble_textarea.insert_str(line);
            preamble_textarea.insert_newline();
        }
        textareas.push(preamble_textarea);
        titles.push(String::from("Untitled"));
    }

    for (title, lines) in parsed.blocks {
        let mut textarea = TextArea::default();
        for line in &lines {
            textarea.insert_str(line);
            textarea.insert_newline();
        }
        textareas.push(textarea);
        titles.push(title);
    }

    Ok((textareas, titles))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_code_blocks() {
        let content = r#"# Test Document
        
```rust
fn main() {
    println!("Hello, world!");
}
```

Some text here

```python
def hello():
    print("Hello")
```"#;

        let blocks = extract_code_blocks(content);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].language, "rust");
        assert_eq!(
            blocks[0].content,
            "fn main() {\n    println!(\"Hello, world!\");\n}"
        );
        assert_eq!(blocks[1].language, "python");
        assert_eq!(blocks[1].content, "def hello():\n    print(\"Hello\")");
    }

    #[test]
    fn test_extract_empty_code_blocks() {
        let content = "```rust\n```";
        let blocks = extract_code_blocks(content);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].language, "rust");
        assert_eq!(blocks[0].content, "");
    }

    #[test]
    fn test_unclosed_code_block() {
        let content = "```js\nlet x = 1;";
        let blocks = extract_code_blocks(content);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].language, "js");
        assert_eq!(blocks[0].content, "let x = 1;");
    }

    #[test]
    fn test_parse_notes_multi_block() {
        let content = "# First\nline one\n# Second\nline two\nline three";
        let parsed = parse_notes(content);
        assert_eq!(parsed.preamble_lines.len(), 0);
        assert_eq!(parsed.blocks.len(), 2);
        assert_eq!(
            parsed.blocks[0],
            ("First".to_string(), vec!["line one".to_string()])
        );
        assert_eq!(
            parsed.blocks[1],
            (
                "Second".to_string(),
                vec!["line two".to_string(), "line three".to_string()]
            )
        );
    }

    #[test]
    fn test_write_roundtrip_with_escapes() {
        let content = "# Notes\nplain text\n\\# not a header\n```rust\n# this is code\n```";
        let parsed = parse_notes(content);
        assert_eq!(parsed.blocks.len(), 1);
        let (title, lines) = &parsed.blocks[0];
        assert_eq!(title, "Notes");
        assert_eq!(
            lines,
            &vec![
                "plain text".to_string(),
                "# not a header".to_string(),
                "```rust".to_string(),
                "# this is code".to_string(),
                "```".to_string(),
            ]
        );

        let mut buffer = Vec::new();
        write_notes(&mut buffer, &parsed).unwrap();
        // Rewritten files always end with a trailing newline
        assert_eq!(String::from_utf8(buffer).unwrap(), format!("{}\n", content));
    }

    #[test]
    fn test_preamble_preserved() {
        let content = "some preamble text\n# Only block\ncontent\n";
        let parsed = parse_notes(content);
        assert_eq!(
            parsed.preamble_lines,
            vec!["some preamble text".to_string()]
        );
        assert_eq!(parsed.blocks.len(), 1);

        let mut buffer = Vec::new();
        write_notes(&mut buffer, &parsed).unwrap();
        assert_eq!(String::from_utf8(buffer).unwrap(), content);
    }

    #[test]
    fn test_headings_inside_code_blocks_are_content() {
        let content = "# Block\n```md\n# Header inside code\n```";
        let parsed = parse_notes(content);
        assert_eq!(parsed.blocks.len(), 1);
        assert_eq!(parsed.blocks[0].1.len(), 3);
    }
}
