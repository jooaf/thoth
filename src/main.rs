use anyhow::{bail, Result};
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, Read};
use thoth_cli::cli::{add_block, copy_block, delete_block, list_blocks, view_block};
use thoth_cli::{
    cli::{Cli, Commands},
    ui_handler::{draw_ui, handle_input, UIState},
};

use std::time::Duration;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Add { name, content }) => {
            let content = match content {
                Some(c) => c.to_string(),
                None => {
                    let mut buffer = String::new();
                    if atty::is(atty::Stream::Stdin) {
                        bail!(format!("Couldn't create '{}' because nothing was passed in. Either pipe in contents or use `thoth add {} <contents>`", name, name));
                    }
                    io::stdin().read_to_string(&mut buffer)?;
                    if buffer.trim().is_empty() {
                        bail!(format!("Couldn't create '{}' because nothing was passed in. Either pipe in contents or use `thoth add {} <contents>`", name, name));
                    }
                    buffer
                }
            };
            add_block(name, &content)?;
        }
        Some(Commands::List) => {
            list_blocks()?;
        }
        Some(Commands::Delete { name }) => {
            delete_block(name)?;
        }
        Some(Commands::View { name }) => {
            view_block(name)?;
        }
        Some(Commands::Copy { name }) => {
            copy_block(name)?;
        }
        None => {
            run_ui()?;
        }
    }

    Ok(())
}

pub fn run_ui() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = UIState::new()?;

    let draw_interval = Duration::from_millis(33);

    loop {
        let should_draw = state.last_draw.elapsed() >= draw_interval;
        if should_draw {
            draw_ui(&mut terminal, &mut state)?;
            state.last_draw = std::time::Instant::now();
        }

        if event::poll(Duration::from_millis(1))? {
            if let Event::Key(key) = event::read()? {
                if handle_input(&mut terminal, &mut state, key)? {
                    break;
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    Ok(())
}
