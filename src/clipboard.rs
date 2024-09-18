#[cfg(target_os = "linux")]
use std::thread;

#[cfg(target_os = "linux")]
use arboard::SetExtLinux;
use arboard::{Clipboard, Error};
pub struct EditorClipboard {
    clipboard: Clipboard,
}

impl EditorClipboard {
    pub fn new() -> Result<EditorClipboard, Error> {
        Clipboard::new().map(|c| EditorClipboard { clipboard: c })
    }
    pub fn try_new() -> Option<EditorClipboard> {
        Self::new().ok()
    }
    #[cfg(not(target_os = "linux"))]
    pub fn set_contents(self: &mut Self, content: String) -> Result<(), Error> {
        self.clipboard.set_text(content)
    }
    #[cfg(target_os = "linux")]
    pub fn set_content(self: &mut Self, content: String) -> Result<(), Error> {
        thread::spawn(|| self.clipboard.set().wait().text(content));
        Ok(())
    }

    pub fn get_content(self: &mut Self) -> Result<String, Error> {
        self.clipboard.get_text()
    }
}
