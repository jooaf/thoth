use lazy_static::lazy_static;
use ratatui::style::Color;

#[derive(Clone)]
pub struct ThemeColors {
    pub primary: Color,
    pub background: Color,
    pub foreground: Color,
    pub selection: Color,
    pub accent: Color,
    pub error: Color,
    pub success: Color,
    pub border: Color,
    pub header_bg: Color,
    pub header_fg: Color,
}

impl PartialEq for ThemeColors {
    fn eq(&self, other: &Self) -> bool {
        self.primary == other.primary
            && self.background == other.background
            && self.foreground == other.foreground
            && self.selection == other.selection
            && self.accent == other.accent
            && self.error == other.error
            && self.success == other.success
            && self.border == other.border
            && self.header_bg == other.header_bg
            && self.header_fg == other.header_fg
    }
}

lazy_static! {
    pub static ref LIGHT_MODE_COLORS: ThemeColors = ThemeColors {
        primary: Color::Rgb(25, 118, 210),    // Blue
        background: Color::Rgb(248, 248, 248), // Light Grey
        foreground: Color::Rgb(33, 33, 33),    // Dark Grey (near black)
        selection: Color::Rgb(207, 232, 252),  // Light Blue
        accent: Color::Rgb(230, 81, 0),        // Orange
        error: Color::Rgb(211, 47, 47),        // Red
        success: Color::Rgb(56, 142, 60),      // Green
        border: Color::Rgb(189, 189, 189),     // Mid Grey
        header_bg: Color::Rgb(232, 232, 232),  // Lighter Grey
        header_fg: Color::Rgb(66, 66, 66),     // Dark Grey
    };

    pub static ref DARK_MODE_COLORS: ThemeColors = ThemeColors {
        primary: Color::Rgb(255, 165, 0),      // Orange (same as original ORANGE constant)
        background: Color::Black,              // Black background (original)
        foreground: Color::White,              // White text (original)
        selection: Color::DarkGray,            // Dark gray for selection (original)
        accent: Color::Rgb(255, 165, 0),       // Orange (same as original ORANGE constant)
        error: Color::Red,                     // Red for errors (original)
        success: Color::Green,                 // Green for success (original)
        border: Color::Rgb(255, 165, 0),       // Orange borders (original)
        header_bg: Color::Black,               // Black header background (original)
        header_fg: Color::Rgb(255, 165, 0),    // Orange header text (original)
    };
}
