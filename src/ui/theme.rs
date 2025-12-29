use ratatui::style::Color;
use serde::{Deserialize, Serialize};

/// Color palette for a theme - defines all semantic colors used in the UI
#[derive(Debug, Clone)]
pub struct ThemePalette {
    // Primary UI colors
    pub primary: Color,
    pub secondary: Color,
    pub success: Color,
    pub error: Color,
    pub warning: Color,
    pub info: Color,

    // Text colors
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_dim: Color,

    // UI element colors
    pub border: Color,
    pub border_focused: Color,
    pub highlight: Color,
    pub background: Color,

    // Special colors
    pub tab_active: Color,
    pub tab_inactive: Color,
    pub preview_border: Color,
    pub help_section: Color,

    // ASCII art gradient colors (for home view)
    pub ascii_art_1: Color,
    pub ascii_art_2: Color,
    pub ascii_art_3: Color,
    pub ascii_art_4: Color,
    pub ascii_art_5: Color,
}

/// Available themes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    Default,
    Nord,
    Dracula,
    Dark,
    White,
}

impl Theme {
    /// Get the color palette for this theme
    pub fn palette(&self) -> ThemePalette {
        match self {
            Theme::Default => ThemePalette {
                // Primary UI colors
                primary: Color::Cyan,
                secondary: Color::Yellow,
                success: Color::Green,
                error: Color::Red,
                warning: Color::Magenta,
                info: Color::Cyan,

                // Text colors
                text_primary: Color::White,
                text_secondary: Color::Gray,
                text_dim: Color::DarkGray,

                // UI element colors
                border: Color::White,
                border_focused: Color::Cyan,
                highlight: Color::Cyan,
                background: Color::Black,

                // Special colors
                tab_active: Color::Cyan,
                tab_inactive: Color::White,
                preview_border: Color::Green,
                help_section: Color::Yellow,

                // ASCII art gradient (current colors from render.rs)
                ascii_art_1: Color::Rgb(210, 215, 255),
                ascii_art_2: Color::Rgb(200, 205, 245),
                ascii_art_3: Color::Rgb(190, 195, 235),
                ascii_art_4: Color::Rgb(175, 180, 220),
                ascii_art_5: Color::Rgb(165, 170, 210),
            },

            Theme::Nord => ThemePalette {
                // Nord color palette (https://www.nordtheme.com/)
                primary: Color::Rgb(136, 192, 208),    // Nord Frost - #88C0D0
                secondary: Color::Rgb(235, 203, 139),  // Nord Aurora Yellow - #EBCB8B
                success: Color::Rgb(163, 190, 140),    // Nord Aurora Green - #A3BE8C
                error: Color::Rgb(191, 97, 106),       // Nord Aurora Red - #BF616A
                warning: Color::Rgb(208, 135, 112),    // Nord Aurora Orange - #D08770
                info: Color::Rgb(129, 161, 193),       // Nord Frost Blue - #81A1C1

                // Text colors
                text_primary: Color::Rgb(236, 239, 244),  // Nord Snow Storm - #ECEFF4
                text_secondary: Color::Rgb(216, 222, 233), // Nord Snow Storm - #D8DEE9
                text_dim: Color::Rgb(76, 86, 106),        // Nord Polar Night - #4C566A

                // UI element colors
                border: Color::Rgb(76, 86, 106),          // Nord Polar Night - #4C566A
                border_focused: Color::Rgb(136, 192, 208), // Nord Frost - #88C0D0
                highlight: Color::Rgb(136, 192, 208),     // Nord Frost - #88C0D0
                background: Color::Rgb(46, 52, 64),       // Nord Polar Night - #2E3440

                // Special colors
                tab_active: Color::Rgb(136, 192, 208),    // Nord Frost - #88C0D0
                tab_inactive: Color::Rgb(216, 222, 233),  // Nord Snow Storm - #D8DEE9
                preview_border: Color::Rgb(163, 190, 140), // Nord Aurora Green - #A3BE8C
                help_section: Color::Rgb(235, 203, 139),  // Nord Aurora Yellow - #EBCB8B

                // ASCII art gradient (Nord blues and purples)
                ascii_art_1: Color::Rgb(143, 188, 187),  // Lighter Nord Frost
                ascii_art_2: Color::Rgb(136, 192, 208),  // Nord Frost
                ascii_art_3: Color::Rgb(129, 161, 193),  // Nord Frost Blue
                ascii_art_4: Color::Rgb(94, 129, 172),   // Nord Frost Dark Blue
                ascii_art_5: Color::Rgb(81, 92, 117),    // Nord Polar Night
            },

            Theme::Dracula => ThemePalette {
                // Dracula color palette (https://draculatheme.com/)
                primary: Color::Rgb(189, 147, 249),    // Purple - #BD93F9
                secondary: Color::Rgb(139, 233, 253),  // Cyan - #8BE9FD
                success: Color::Rgb(80, 250, 123),     // Green - #50FA7B
                error: Color::Rgb(255, 85, 85),        // Red - #FF5555
                warning: Color::Rgb(241, 250, 140),    // Yellow - #F1FA8C
                info: Color::Rgb(139, 233, 253),       // Cyan - #8BE9FD

                // Text colors
                text_primary: Color::Rgb(248, 248, 242),  // Foreground - #F8F8F2
                text_secondary: Color::Rgb(98, 114, 164), // Comment - #6272A4
                text_dim: Color::Rgb(68, 71, 90),         // Current Line - #44475A

                // UI element colors
                border: Color::Rgb(68, 71, 90),           // Current Line - #44475A
                border_focused: Color::Rgb(189, 147, 249), // Purple - #BD93F9
                highlight: Color::Rgb(189, 147, 249),     // Purple - #BD93F9
                background: Color::Rgb(40, 42, 54),       // Background - #282A36

                // Special colors
                tab_active: Color::Rgb(189, 147, 249),    // Purple - #BD93F9
                tab_inactive: Color::Rgb(248, 248, 242),  // Foreground - #F8F8F2
                preview_border: Color::Rgb(80, 250, 123), // Green - #50FA7B
                help_section: Color::Rgb(241, 250, 140),  // Yellow - #F1FA8C

                // ASCII art gradient (Dracula purples and pinks)
                ascii_art_1: Color::Rgb(255, 121, 198),  // Pink - #FF79C6
                ascii_art_2: Color::Rgb(189, 147, 249),  // Purple - #BD93F9
                ascii_art_3: Color::Rgb(139, 233, 253),  // Cyan - #8BE9FD
                ascii_art_4: Color::Rgb(98, 114, 164),   // Comment - #6272A4
                ascii_art_5: Color::Rgb(68, 71, 90),     // Current Line - #44475A
            },

            Theme::Dark => ThemePalette {
                // Material-inspired dark theme
                primary: Color::Rgb(100, 149, 237),    // Cornflower Blue
                secondary: Color::Rgb(255, 200, 87),   // Amber
                success: Color::Rgb(76, 175, 80),      // Material Green
                error: Color::Rgb(244, 67, 54),        // Material Red
                warning: Color::Rgb(255, 152, 0),      // Material Orange
                info: Color::Rgb(33, 150, 243),        // Material Blue

                // Text colors
                text_primary: Color::Rgb(224, 224, 224),  // Light Gray
                text_secondary: Color::Rgb(158, 158, 158), // Medium Gray
                text_dim: Color::Rgb(97, 97, 97),         // Dark Gray

                // UI element colors
                border: Color::Rgb(66, 66, 66),           // Dark Gray Border
                border_focused: Color::Rgb(100, 149, 237), // Cornflower Blue
                highlight: Color::Rgb(100, 149, 237),     // Cornflower Blue
                background: Color::Rgb(18, 18, 18),       // Very Dark Gray

                // Special colors
                tab_active: Color::Rgb(100, 149, 237),    // Cornflower Blue
                tab_inactive: Color::Rgb(158, 158, 158),  // Medium Gray
                preview_border: Color::Rgb(76, 175, 80),  // Material Green
                help_section: Color::Rgb(255, 200, 87),   // Amber

                // ASCII art gradient (Blue to purple)
                ascii_art_1: Color::Rgb(100, 181, 246),  // Light Blue
                ascii_art_2: Color::Rgb(100, 149, 237),  // Cornflower Blue
                ascii_art_3: Color::Rgb(92, 107, 192),   // Indigo
                ascii_art_4: Color::Rgb(103, 58, 183),   // Deep Purple
                ascii_art_5: Color::Rgb(81, 45, 168),    // Darker Purple
            },

            Theme::White => ThemePalette {
                // Light theme with high contrast
                primary: Color::Rgb(25, 118, 210),     // Blue
                secondary: Color::Rgb(255, 143, 0),    // Orange
                success: Color::Rgb(56, 142, 60),      // Dark Green
                error: Color::Rgb(211, 47, 47),        // Dark Red
                warning: Color::Rgb(245, 124, 0),      // Dark Orange
                info: Color::Rgb(2, 136, 209),         // Dark Cyan

                // Text colors
                text_primary: Color::Rgb(33, 33, 33),     // Near Black
                text_secondary: Color::Rgb(97, 97, 97),   // Dark Gray
                text_dim: Color::Rgb(158, 158, 158),      // Medium Gray

                // UI element colors
                border: Color::Rgb(189, 189, 189),        // Light Gray
                border_focused: Color::Rgb(25, 118, 210), // Blue
                highlight: Color::Rgb(25, 118, 210),      // Blue
                background: Color::Rgb(245, 245, 245),    // Off-White

                // Special colors
                tab_active: Color::Rgb(25, 118, 210),     // Blue
                tab_inactive: Color::Rgb(97, 97, 97),     // Dark Gray
                preview_border: Color::Rgb(56, 142, 60),  // Dark Green
                help_section: Color::Rgb(255, 143, 0),    // Orange

                // ASCII art gradient (Blue shades)
                ascii_art_1: Color::Rgb(66, 165, 245),   // Light Blue
                ascii_art_2: Color::Rgb(25, 118, 210),   // Blue
                ascii_art_3: Color::Rgb(21, 101, 192),   // Darker Blue
                ascii_art_4: Color::Rgb(13, 71, 161),    // Dark Blue
                ascii_art_5: Color::Rgb(10, 57, 129),    // Very Dark Blue
            },
        }
    }

    /// Get the display name of this theme
    pub fn name(&self) -> &str {
        match self {
            Theme::Default => "Default",
            Theme::Nord => "Nord",
            Theme::Dracula => "Dracula",
            Theme::Dark => "Dark",
            Theme::White => "White (Light)",
        }
    }

    /// Get all available themes
    pub fn all() -> Vec<Theme> {
        vec![
            Theme::Default,
            Theme::Nord,
            Theme::Dracula,
            Theme::Dark,
            Theme::White,
        ]
    }
}

impl Default for Theme {
    fn default() -> Self {
        Theme::Default
    }
}
