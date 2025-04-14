use crate::code_block_popup::CodeBlock;
use anyhow::Result;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::{fs::File, io::BufReader};
use tui_textarea::TextArea;

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
    let mut file = File::create(file_path)?;
    for (textarea, title) in textareas.iter().zip(titles.iter()) {
        writeln!(file, "# {}", title)?;
        let content = textarea.lines().join("\n");
        let mut in_code_block = false;
        for line in content.lines() {
            if line.trim().starts_with("```") {
                in_code_block = !in_code_block;
            }
            if in_code_block || !line.starts_with('#') {
                writeln!(file, "{}", line)?;
            } else {
                writeln!(file, "\\{}", line)?;
            }
        }
    }
    Ok(())
}

pub fn load_textareas(file_path: PathBuf) -> Result<(Vec<TextArea<'static>>, Vec<String>)> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let mut textareas = Vec::with_capacity(10);
    let mut titles = Vec::with_capacity(10);
    let mut current_textarea = TextArea::default();
    let mut current_title = String::new();
    let mut in_code_block = false;
    let mut is_first_line = true;

    for line in reader.lines() {
        let line = line?;
        if !in_code_block && line.starts_with("# ") && is_first_line {
            current_title = line[2..].to_string();
            is_first_line = false;
        } else {
            if line.trim().starts_with("```") {
                in_code_block = !in_code_block;
            }
            if in_code_block {
                current_textarea.insert_str(&line);
            } else if let Some(strip) = line.strip_prefix('\\') {
                current_textarea.insert_str(strip);
            } else if line.starts_with("# ") && !is_first_line {
                if !current_title.is_empty() {
                    textareas.push(current_textarea);
                    titles.push(current_title);
                }
                current_textarea = TextArea::default();
                current_title = line[2..].to_string();
                is_first_line = true;
                continue;
            } else {
                current_textarea.insert_str(&line);
            }
            current_textarea.insert_newline();
            is_first_line = false;
        }
    }

    if !current_title.is_empty() {
        textareas.push(current_textarea);
        titles.push(current_title);
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
}
