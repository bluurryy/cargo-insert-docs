use anstyle::{AnsiColor, Color, Effects, Style};

const fn label(color: AnsiColor) -> Style {
    Style::new().fg_color(Some(Color::Ansi(color))).effects(Effects::BOLD)
}

pub const INFO: Style = label(AnsiColor::Green);
pub const WARNING: Style = label(AnsiColor::Yellow);
pub const ERROR: Style = label(AnsiColor::Red);

pub const BOLD: Style = Style::new().effects(Effects::BOLD);
