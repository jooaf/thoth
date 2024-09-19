use std::sync::{Arc, Mutex};
#[cfg(target_os = "linux")]
use std::thread;

#[cfg(target_os = "linux")]
use arboard::SetExtLinux;
use arboard::{Clipboard, Error};

const DAEMONIZE_ARG: &str = "98b2d50a-2152-463e-986b-bc4b8280452e";

pub struct EditorClipboard {
    clipboard: Arc<Mutex<Clipboard>>,
}

impl EditorClipboard {
    pub fn new() -> Result<EditorClipboard, Error> {
        #[cfg(target_os = "linux")]
        if env::args().nth(1).as_deref() == Some(DAEMONIZE_ARG) {
            let mut clipboard = Clipboard::new()?;
            clipboard.set().wait().text("")?;
            std::thread::park();
            return Err(Error::ClipboardError("Daemon mode".into()));
        }

        Clipboard::new().map(|c| EditorClipboard {
            clipboard: Arc::new(Mutex::new(c)),
        })
    }

    pub fn try_new() -> Option<EditorClipboard> {
        Self::new().ok()
    }

    #[cfg(not(target_os = "linux"))]
    pub fn set_contents(&mut self, content: String) -> Result<(), Error> {
        let mut clipboard = self.clipboard.lock().unwrap();
        clipboard.set_text(content)
    }

    #[cfg(target_os = "linux")]
    pub fn set_contents(&mut self, content: String) -> Result<(), Error> {
        let daemon = process::Command::new(env::current_exe()?)
            .arg(DAEMONIZE_ARG)
            .stdin(process::Stdio::null())
            .stdout(process::Stdio::null())
            .stderr(process::Stdio::null())
            .current_dir("/")
            .spawn()
            .map_err(|e| Error::ClipboardError(e.to_string()))?;

        let mut clipboard = self.clipboard.lock().unwrap();
        clipboard.set().wait().text(content)?;

        Ok(())
    }

    pub fn get_content(&mut self) -> Result<String, Error> {
        let mut clipboard = self.clipboard.lock().unwrap();
        clipboard.get_text()
    }
}
