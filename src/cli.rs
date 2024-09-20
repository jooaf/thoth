use crate::EditorClipboard;
use anyhow::{bail, Result};
use std::{
    fs::File,
    io::{BufRead, BufReader, Write},
};

use std::env;

use clap::{Parser, Subcommand};

use crate::get_save_file_path;
#[derive(Parser)]
#[command(author = env!("CARGO_PKG_AUTHORS"), version = env!("CARGO_PKG_VERSION"), about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Add a new block to the scratchpad
    Add {
        /// Name of the block to be added
        name: String,
        /// Contents to be associated with the named block
        content: Option<String>,
    },
    /// List all of the blocks within your thoth scratchpad
    List,
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
}

pub fn add_block(name: &str, content: &str) -> Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(get_save_file_path())?;

    writeln!(file, "# {}", name)?;
    writeln!(file, "{}", content)?;
    writeln!(file)?;

    println!("Block '{}' added successfully.", name);
    Ok(())
}

pub fn list_blocks() -> Result<()> {
    let file = File::open(get_save_file_path())?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line?;

        if let Some(strip) = line.strip_prefix("# ") {
            println!("{}", strip);
        }
    }

    Ok(())
}

pub fn view_block(name: &str) -> Result<()> {
    let file = File::open(get_save_file_path())?;
    let reader = BufReader::new(file);
    let mut blocks = Vec::new();
    let mut current_block = Vec::new();
    let mut current_name = String::new();

    for line in reader.lines() {
        let line = line?;
        if let Some(strip) = line.strip_prefix("# ") {
            if !current_name.is_empty() {
                blocks.push((current_name, current_block));
                current_block = Vec::new();
            }
            current_name = strip.to_string();
        } else {
            current_block.push(line);
        }
    }

    if !current_name.is_empty() {
        blocks.push((current_name, current_block));
    }

    for (block_name, block_content) in blocks {
        if block_name == name {
            for line in block_content {
                println!("{}", line);
            }
        }
    }
    Ok(())
}

pub fn copy_block(name: &str) -> Result<()> {
    let file = File::open(get_save_file_path())?;
    let reader = BufReader::new(file);
    let mut blocks = Vec::new();
    let mut current_block = Vec::new();
    let mut current_name = String::new();
    let mut matched_name: Option<String> = None;

    for line in reader.lines() {
        let line = line?;
        if let Some(strip) = line.strip_prefix("# ") {
            if !current_name.is_empty() {
                blocks.push((current_name, current_block));
                current_block = Vec::new();
            }
            current_name = strip.to_string();
        } else {
            current_block.push(line);
        }
    }

    if !current_name.is_empty() {
        blocks.push((current_name, current_block));
    }

    for (block_name, block_content) in blocks {
        if block_name == name {
            let result_ctx = EditorClipboard::new();

            if result_ctx.is_err() {
                bail!("Failed to create clipboard context for copy block");
            }

            let mut ctx = result_ctx.unwrap();

            let is_success = ctx.set_contents(block_content.join("\n"));

            if is_success.is_err() {
                bail!(format!(
                    "Failed to copy contents of block {} to system clipboard",
                    block_name
                ));
            }
            matched_name = Some(block_name);
            break;
        }
    }
    match matched_name {
        Some(name) => println!("Successfully copied contents from block {}", name),
        None => println!("Didn't find the block. Please try again. You can use `thoth list` to find the name of all blocks")
    };

    Ok(())
}

pub fn delete_block(name: &str) -> Result<()> {
    let file = File::open(get_save_file_path())?;
    let reader = BufReader::new(file);
    let mut blocks = Vec::new();
    let mut current_block = Vec::new();
    let mut current_name = String::new();

    for line in reader.lines() {
        let line = line?;
        if let Some(strip) = line.strip_prefix("# ") {
            if !current_name.is_empty() {
                blocks.push((current_name, current_block));
                current_block = Vec::new();
            }
            current_name = strip.to_string();
        } else {
            current_block.push(line);
        }
    }

    if !current_name.is_empty() {
        blocks.push((current_name, current_block));
    }

    let mut file = File::create(get_save_file_path())?;
    let mut deleted = false;

    for (block_name, block_content) in blocks {
        if block_name != name {
            writeln!(file, "# {}", block_name)?;
            for line in block_content {
                writeln!(file, "{}", line)?;
            }
            writeln!(file)?;
        } else {
            deleted = true;
        }
    }

    if deleted {
        println!("Block '{}' deleted successfully.", name);
    } else {
        println!("Block '{}' not found.", name);
    }

    Ok(())
}
