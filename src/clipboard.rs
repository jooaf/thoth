use std::sync::{Arc, Mutex};
#[cfg(target_os = "linux")]
use std::thread;

#[cfg(target_os = "linux")]
use arboard::SetExtLinux;
use arboard::{Clipboard, Error};

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

    #[cfg(not(target_os = "linux"))]
    pub fn set_contents(&mut self, content: String) -> Result<(), Error> {
        let mut clipboard = self.clipboard.lock().unwrap();
        clipboard.set_text(content)
    }

    #[cfg(target_os = "linux")]
    pub fn set_contents(&mut self, content: String) -> Result<(), Error> {
        let clipboard = Arc::clone(&self.clipboard);
        thread::spawn(move || {
            let mut clipboard = clipboard.lock().unwrap();
            clipboard.set().wait().text(content).unwrap();
        });
        Ok(())
    }

    pub fn get_content(&mut self) -> Result<String, Error> {
        let mut clipboard = self.clipboard.lock().unwrap();
        clipboard.get_text()
    }
}
