use crate::{get_save_file_path, EditorClipboard, ThemeMode, ThothConfig};
use anyhow::{bail, Result};
use std::{fs::File, io::Write};

use std::env;
use std::path::Path;
use std::process::Command;
use tempfile::NamedTempFile;

use clap::{Parser, Subcommand};

use crate::utils::{parse_notes, write_notes, ParsedNotes};
#[derive(Parser)]
#[command(author = env!("CARGO_PKG_AUTHORS"), version = env!("CARGO_PKG_VERSION"), about, long_about = None, rename_all = "snake_case")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
#[command(rename_all = "snake_case")]
pub enum Commands {
    /// Add a new block to the scratchpad.
    /// If a block with the same name already exists, the content is appended to it.
    Add {
        /// Name of the block to be added
        name: String,
        /// Contents to be associated with the named block
        content: Option<String>,
    },
    /// List all of the blocks within your thoth scratchpad
    List {
        /// Show additional details such as line counts per block
        #[arg(short = 'l', long)]
        long: bool,
    },
    /// Load backup file as the main thoth markdown file
    LoadBackup,
    /// Read the contents of the clipboard backup file
    ReadClipboard,
    /// Delete a block by name
    Delete {
        /// The name of the block to be deleted
        name: String,
    },
    /// View (STDOUT) the contents of the block by name
    View {
        /// The name of the block to be used
        name: String,
    },
    /// Copy the contents of a block to the system clipboard
    Copy {
        /// The name of the block to be used
        name: String,
    },
    /// Rename a block
    Rename {
        /// Current name of the block
        old_name: String,
        /// New name for the block
        new_name: String,
    },
    /// Edit the contents of a block with your $EDITOR/$VISUAL editor
    Edit {
        /// The name of the block to be edited
        name: String,
    },
    /// Set the theme to light or dark mode
    Theme {
        /// The theme to set: 'light' or 'dark'
        mode: String,
    },
    /// Get the current theme
    GetTheme,
}

pub fn set_theme(mode: &str) -> Result<()> {
    let mode_lowercase = mode.to_lowercase();

    let mut config = ThothConfig::load()?;

    match mode_lowercase.as_str() {
        "light" => {
            config.set_theme(ThemeMode::Light)?;
            println!("Theme set to light mode");
        }
        "dark" => {
            config.set_theme(ThemeMode::Dark)?;
            println!("Theme set to dark mode");
        }
        _ => {
            bail!("Invalid theme mode. Use 'light' or 'dark'");
        }
    }

    Ok(())
}

pub fn get_theme() -> Result<()> {
    let config = ThothConfig::load()?;

    match config.theme {
        ThemeMode::Light => println!("Current theme: light"),
        ThemeMode::Dark => println!("Current theme: dark"),
    }

    Ok(())
}

pub fn read_clipboard_backup() -> Result<()> {
    let file_path = crate::get_clipboard_backup_file_path();
    if !file_path.exists() {
        println!("No clipboard backup file found at {}", file_path.display());
        return Ok(());
    }

    let content = std::fs::read_to_string(&file_path)?;
    if content.is_empty() {
        println!("Clipboard backup file exists but is empty.");
    } else {
        println!("{}", content);
    }
    Ok(())
}

/// Load the notes file as parsed blocks. Returns an empty result when the file
/// does not exist yet.
fn load_parsed_notes(file_path: &Path) -> Result<ParsedNotes> {
    if !file_path.exists() {
        return Ok(ParsedNotes::default());
    }
    let content = std::fs::read_to_string(file_path)?;
    Ok(parse_notes(&content))
}

/// Write parsed blocks to the notes file, creating parent directories as needed.
fn save_parsed_notes(file_path: &Path, parsed: &ParsedNotes) -> Result<()> {
    if let Some(parent) = file_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let mut file = File::create(file_path)?;
    write_notes(&mut file, parsed)
}

fn find_block_index(parsed: &ParsedNotes, name: &str) -> Option<usize> {
    parsed.blocks.iter().position(|(title, _)| title == name)
}

pub fn add_block(name: &str, content: &str) -> Result<()> {
    let file_path = get_save_file_path();
    let mut parsed = load_parsed_notes(&file_path)?;
    let content_lines: Vec<String> = content.lines().map(str::to_string).collect();

    if let Some(index) = find_block_index(&parsed, name) {
        let lines = &mut parsed.blocks[index].1;
        // Separate appended content from existing content with a blank line
        if let Some(last) = lines.last() {
            if !last.is_empty() {
                lines.push(String::new());
            }
        }
        lines.extend(content_lines);
        save_parsed_notes(&file_path, &parsed)?;
        println!("Appended content to existing block '{}'.", name);
    } else {
        parsed.blocks.push((name.to_string(), content_lines));
        save_parsed_notes(&file_path, &parsed)?;
        println!("Block '{}' added successfully.", name);
    }
    Ok(())
}

pub fn list_blocks() -> Result<()> {
    list_blocks_with_options(false)
}

pub fn list_blocks_with_options(long: bool) -> Result<()> {
    let file_path = get_save_file_path();
    if !file_path.exists() {
        println!(
            "No notes file found at {}. Add one with `thoth add <name> <content>` or start the TUI with `thoth`.",
            file_path.display()
        );
        return Ok(());
    }

    let parsed = load_parsed_notes(&file_path)?;
    for (title, lines) in &parsed.blocks {
        if long {
            println!("{} ({} lines)", title, lines.len());
        } else {
            println!("{}", title);
        }
    }

    Ok(())
}

pub fn replace_from_backup() -> Result<()> {
    let (backup_textareas, backup_textareas_titles) =
        crate::load_textareas(crate::get_save_backup_file_path())?;
    crate::save_textareas(
        &backup_textareas,
        &backup_textareas_titles,
        get_save_file_path(),
    )
}

pub fn view_block(name: &str) -> Result<()> {
    let parsed = load_parsed_notes(&get_save_file_path())?;

    match find_block_index(&parsed, name) {
        Some(index) => {
            for line in &parsed.blocks[index].1 {
                println!("{}", line);
            }
        }
        None => bail!(
            "Block '{}' not found. Use `thoth list` to see available blocks.",
            name
        ),
    }
    Ok(())
}

pub fn copy_block(name: &str) -> Result<()> {
    let parsed = load_parsed_notes(&get_save_file_path())?;

    match find_block_index(&parsed, name) {
        Some(index) => {
            let content = parsed.blocks[index].1.join("\n");
            let mut ctx = EditorClipboard::new()
                .map_err(|e| anyhow::anyhow!("Failed to create clipboard context: {}", e))?;
            ctx.set_contents(content).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to copy contents of block {} to system clipboard: {}",
                    name,
                    e
                )
            })?;
            println!("Successfully copied contents from block {}", name);
        }
        None => bail!(
            "Didn't find the block. Please try again. You can use `thoth list` to find the name of all blocks"
        ),
    }
    Ok(())
}

pub fn delete_block(name: &str) -> Result<()> {
    let file_path = get_save_file_path();
    let parsed = load_parsed_notes(&file_path)?;

    match find_block_index(&parsed, name) {
        Some(index) => {
            let mut remaining = parsed;
            remaining.blocks.remove(index);
            save_parsed_notes(&file_path, &remaining)?;
            println!("Block '{}' deleted successfully.", name);
        }
        None => bail!("Block '{}' not found.", name),
    }
    Ok(())
}

pub fn rename_block(old_name: &str, new_name: &str) -> Result<()> {
    let file_path = get_save_file_path();
    let mut parsed = load_parsed_notes(&file_path)?;
    // Renaming to an existing name would create a duplicate block
    let new_index = find_block_index(&parsed, new_name);
    if let Some(index) = new_index {
        if parsed.blocks[index].0 == new_name && Some(index) != find_block_index(&parsed, old_name)
        {
            bail!(
                "A block named '{}' already exists. Choose a different name.",
                new_name
            );
        }
    }

    match find_block_index(&parsed, old_name) {
        Some(index) => {
            parsed.blocks[index].0 = new_name.to_string();
            save_parsed_notes(&file_path, &parsed)?;
            println!("Block '{}' renamed to '{}'.", old_name, new_name);
        }
        None => bail!(
            "Block '{}' not found. Use `thoth list` to see available blocks.",
            old_name
        ),
    }
    Ok(())
}

pub fn edit_block(name: &str) -> Result<()> {
    let file_path = get_save_file_path();
    let mut parsed = load_parsed_notes(&file_path)?;
    let index = find_block_index(&parsed, name).ok_or_else(|| {
        anyhow::anyhow!(
            "Block '{}' not found. Use `thoth list` to see available blocks.",
            name
        )
    })?;

    let content = parsed.blocks[index].1.join("\n");
    let mut temp_file = NamedTempFile::new()?;
    temp_file.write_all(content.as_bytes())?;
    temp_file.flush()?;

    let editor = env::var("VISUAL")
        .or_else(|_| env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_string());

    let status = Command::new(&editor).arg(temp_file.path()).status()?;
    if !status.success() {
        bail!(format!("Editor '{}' returned non-zero status", editor));
    }

    let edited_content = std::fs::read_to_string(temp_file.path())?;
    parsed.blocks[index].1 = edited_content.lines().map(str::to_string).collect();
    save_parsed_notes(&file_path, &parsed)?;
    println!("Block '{}' updated successfully.", name);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Mutex;

    // The notes location is process-global, so serialize tests that touch it.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_notes_dir<F: FnOnce()>(f: F) {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempfile::TempDir::new().unwrap();
        std::env::set_var("THOTH_NOTES_DIR", dir.path());
        f();
        std::env::remove_var("THOTH_NOTES_DIR");
    }

    fn notes_file() -> PathBuf {
        get_save_file_path()
    }

    fn file_content() -> String {
        std::fs::read_to_string(notes_file()).unwrap()
    }

    #[test]
    fn test_add_creates_new_block() {
        with_notes_dir(|| {
            add_block("groceries", "bananas\nmilk").unwrap();
            assert_eq!(file_content(), "# groceries\nbananas\nmilk\n");
        });
    }

    #[test]
    fn test_add_existing_block_appends() {
        with_notes_dir(|| {
            add_block("groceries", "bananas").unwrap();
            add_block("groceries", "milk").unwrap();
            assert_eq!(file_content(), "# groceries\nbananas\n\nmilk\n");
        });
    }

    #[test]
    fn test_add_preserves_code_fences() {
        with_notes_dir(|| {
            add_block("code", "```rust\n# comment\n```\nheading after code").unwrap();
            let content = file_content();
            // Content inside a fenced block is kept verbatim (unescaped)
            assert_eq!(
                content,
                "# code\n```rust\n# comment\n```\nheading after code\n"
            );
        });
    }

    #[test]
    fn test_view_missing_block_errors() {
        with_notes_dir(|| {
            add_block("real", "content").unwrap();
            assert!(view_block("missing").is_err());
        });
    }

    #[test]
    fn test_delete_missing_block_preserves_file() {
        with_notes_dir(|| {
            add_block("keeper", "content").unwrap();
            let before = file_content();
            assert!(delete_block("nope").is_err());
            assert_eq!(file_content(), before);
        });
    }

    #[test]
    fn test_delete_existing_block() {
        with_notes_dir(|| {
            add_block("keeper", "content").unwrap();
            add_block("doomed", "bye").unwrap();
            delete_block("doomed").unwrap();
            let content = file_content();
            assert!(content.contains("# keeper"));
            assert!(!content.contains("doomed"));
        });
    }

    #[test]
    fn test_rename_block() {
        with_notes_dir(|| {
            add_block("old", "content").unwrap();
            rename_block("old", "new").unwrap();
            let content = file_content();
            assert!(content.contains("# new"));
            assert!(!content.contains("# old"));

            // Renaming to an existing name is rejected
            add_block("other", "x").unwrap();
            assert!(rename_block("other", "new").is_err());
        });
    }

    #[test]
    fn test_preamble_preserved_on_rewrite() {
        with_notes_dir(|| {
            let dir = notes_file();
            std::fs::create_dir_all(dir.parent().unwrap()).unwrap();
            std::fs::write(&dir, "loose notes\n# block\ncontent\n").unwrap();
            delete_block("block").unwrap();
            assert_eq!(file_content(), "loose notes\n");
        });
    }
}
