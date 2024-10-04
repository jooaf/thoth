use crate::get_save_file_path;
use anyhow::Result;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::{fs::File, io::BufReader};
use tui_textarea::TextArea;

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

pub fn load_textareas() -> Result<(Vec<TextArea<'static>>, Vec<String>)> {
    let file = File::open(get_save_file_path())?;
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
