use anyhow::Result;
use arboard::{Clipboard, Error};
use std::env;
use std::process;
use std::sync::{Arc, Mutex};

#[cfg(target_os = "linux")]
use arboard::SetExtLinux;

use crate::DAEMONIZE_ARG;

pub trait ClipboardTrait {
    fn set_contents(&mut self, content: String) -> anyhow::Result<()>;
    fn get_content(&self) -> anyhow::Result<String>;
}

pub struct EditorClipboard {
    clipboard: Arc<Mutex<Clipboard>>,
}

impl EditorClipboard {
    pub fn new() -> Result<EditorClipboard, Error> {
        Clipboard::new().map(|c| EditorClipboard {
            clipboard: Arc::new(Mutex::new(c)),
        })
    }

    pub fn try_new() -> Option<EditorClipboard> {
        Self::new().ok()
    }

    pub fn set_contents(&mut self, content: String) -> Result<(), Error> {
        #[cfg(target_os = "linux")]
        {
            let is_wayland = std::env::var("WAYLAND_DISPLAY").is_ok();

            if is_wayland {
                let mut clipboard = self
                    .clipboard
                    .lock()
                    .map_err(|_e| arboard::Error::ContentNotAvailable)?;
                clipboard.set().wait().text(content);
            } else if env::args().nth(1).as_deref() == Some(DAEMONIZE_ARG) {
                let mut clipboard = self
                    .clipboard
                    .lock()
                    .map_err(|_e| arboard::Error::ContentNotAvailable)?;
                clipboard.set().wait().text(content);
            } else {
                process::Command::new(env::current_exe().unwrap())
                    .arg(DAEMONIZE_ARG)
                    .arg(content)
                    .stdin(process::Stdio::null())
                    .stdout(process::Stdio::null())
                    .stderr(process::Stdio::null())
                    .current_dir("/")
                    .spawn()
                    .map_err(|_e| arboard::Error::ContentNotAvailable)?;
                Ok(());
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            let mut clipboard = self
                .clipboard
                .lock()
                .map_err(|_e| arboard::Error::ContentNotAvailable)?;
            clipboard.set_text(content)
        }
    }

    pub fn get_content(&mut self) -> Result<String, Error> {
        let mut clipboard = self.clipboard.lock().unwrap();
        clipboard.get_text()
    }

    #[cfg(target_os = "linux")]
    pub fn handle_daemon_args() -> Result<(), Error> {
        if let Some(content) = env::args().nth(2) {
            if env::args().nth(1).as_deref() == Some(DAEMONIZE_ARG) {
                let mut clipboard = Self::new()?;
                clipboard.set_contents(content)?;
                std::process::exit(0);
            }
        }
        Ok(())
    }
}
