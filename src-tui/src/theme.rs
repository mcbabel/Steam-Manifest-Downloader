use std::sync::RwLock;

use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
    Terminal,
}

impl ThemeMode {
    pub const ALL: [ThemeMode; 3] = [ThemeMode::Dark, ThemeMode::Light, ThemeMode::Terminal];

    pub fn label_key(self) -> &'static str {
        match self {
            ThemeMode::Dark => "tui.settings.themeDark",
            ThemeMode::Light => "tui.settings.themeLight",
            ThemeMode::Terminal => "tui.settings.themeTerminal",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub truecolor: bool,
    pub bg: Color,
    pub surface: Color,
    pub surface_hi: Color,
    pub border: Color,
    pub border_focus: Color,
    pub text: Color,
    pub text_dim: Color,
    pub text_muted: Color,
    pub accent: Color,
    pub purple: Color,
    pub success: Color,
    pub error: Color,
    pub warning: Color,
    pub on_accent: Color,
    pub selection: Color,
}

const fn rgb(hex: u32) -> Color {
    Color::Rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

const DARK: Theme = Theme {
    truecolor: true,
    bg: rgb(0x0d1117),
    surface: rgb(0x161b22),
    surface_hi: rgb(0x1e2530),
    border: rgb(0x30363d),
    border_focus: rgb(0x58a6ff),
    text: rgb(0xe6edf3),
    text_dim: rgb(0x8b949e),
    text_muted: rgb(0x6e7681),
    accent: rgb(0x58a6ff),
    purple: rgb(0xbc8cff),
    success: rgb(0x3fb950),
    error: rgb(0xf85149),
    warning: rgb(0xd29922),
    on_accent: rgb(0x0d1117),
    selection: rgb(0x1f3a5f),
};

const LIGHT: Theme = Theme {
    truecolor: true,
    bg: rgb(0xf6f8fa),
    surface: rgb(0xffffff),
    surface_hi: rgb(0xeef1f5),
    border: rgb(0xd0d7de),
    border_focus: rgb(0x0969da),
    text: rgb(0x1f2328),
    text_dim: rgb(0x656d76),
    text_muted: rgb(0x8b949e),
    accent: rgb(0x0969da),
    purple: rgb(0x8250df),
    success: rgb(0x1a7f37),
    error: rgb(0xcf222e),
    warning: rgb(0x9a6700),
    on_accent: rgb(0xffffff),
    selection: rgb(0xddf4ff),
};

const TERMINAL: Theme = Theme {
    truecolor: false,
    bg: Color::Reset,
    surface: Color::Reset,
    surface_hi: Color::Reset,
    border: Color::DarkGray,
    border_focus: Color::LightBlue,
    text: Color::Reset,
    text_dim: Color::Gray,
    text_muted: Color::DarkGray,
    accent: Color::LightBlue,
    purple: Color::LightMagenta,
    success: Color::LightGreen,
    error: Color::LightRed,
    warning: Color::Yellow,
    on_accent: Color::Black,
    selection: Color::DarkGray,
};

const MONO: Theme = Theme {
    truecolor: false,
    bg: Color::Reset,
    surface: Color::Reset,
    surface_hi: Color::Reset,
    border: Color::Reset,
    border_focus: Color::Reset,
    text: Color::Reset,
    text_dim: Color::Reset,
    text_muted: Color::Reset,
    accent: Color::Reset,
    purple: Color::Reset,
    success: Color::Reset,
    error: Color::Reset,
    warning: Color::Reset,
    on_accent: Color::Reset,
    selection: Color::Reset,
};

static CURRENT: RwLock<Theme> = RwLock::new(DARK);
static MONOCHROME: RwLock<bool> = RwLock::new(false);

pub fn supports_truecolor() -> bool {
    #[cfg(windows)]
    if crossterm::ansi_support::supports_ansi() {
        return true;
    }
    if let Ok(v) = std::env::var("COLORTERM") {
        let v = v.to_ascii_lowercase();
        if v.contains("truecolor") || v.contains("24bit") {
            return true;
        }
    }
    if std::env::var_os("WT_SESSION").is_some() {
        return true;
    }
    matches!(
        std::env::var("TERM_PROGRAM").as_deref(),
        Ok("iTerm.app") | Ok("WezTerm") | Ok("vscode") | Ok("ghostty") | Ok("rio")
    ) || std::env::var("TERM")
        .map(|t| t.contains("direct") || t == "xterm-kitty")
        .unwrap_or(false)
}

pub fn no_color() -> bool {
    std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty())
}

pub fn apply(mode: ThemeMode) {
    let mono = no_color();
    let theme = if mono {
        MONO
    } else {
        match mode {
            ThemeMode::Dark if supports_truecolor() => DARK,
            ThemeMode::Light if supports_truecolor() => LIGHT,
            _ => TERMINAL,
        }
    };
    if let Ok(mut t) = CURRENT.write() {
        *t = theme;
    }
    if let Ok(mut m) = MONOCHROME.write() {
        *m = mono;
    }
}

pub fn get() -> Theme {
    CURRENT.read().map(|t| *t).unwrap_or(DARK)
}

pub fn monochrome() -> bool {
    MONOCHROME.read().map(|m| *m).unwrap_or(false)
}

pub fn base() -> Style {
    let t = get();
    Style::default().fg(t.text).bg(t.bg)
}

pub fn text() -> Style {
    Style::default().fg(get().text)
}

pub fn dim() -> Style {
    Style::default().fg(get().text_dim)
}

pub fn muted() -> Style {
    Style::default().fg(get().text_muted)
}

pub fn accent() -> Style {
    Style::default().fg(get().accent)
}

pub fn accent_bold() -> Style {
    accent().add_modifier(Modifier::BOLD)
}

pub fn success() -> Style {
    Style::default().fg(get().success)
}

pub fn error() -> Style {
    Style::default().fg(get().error)
}

pub fn warning() -> Style {
    Style::default().fg(get().warning)
}

pub fn border(focused: bool) -> Style {
    let t = get();
    Style::default().fg(if focused { t.border_focus } else { t.border })
}

pub fn surface() -> Style {
    let t = get();
    Style::default().bg(t.surface).fg(t.text)
}

pub fn selection() -> Style {
    let t = get();
    if monochrome() {
        Style::default().add_modifier(Modifier::REVERSED)
    } else {
        Style::default().bg(t.selection).fg(t.text)
    }
}

pub fn gradient(pos: f32) -> Color {
    let t = get();
    match (t.accent, t.purple) {
        (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => {
            let p = pos.clamp(0.0, 1.0);
            let lerp = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * p).round() as u8;
            Color::Rgb(lerp(r1, r2), lerp(g1, g2), lerp(b1, b2))
        }
        _ => {
            if pos < 0.5 {
                t.accent
            } else {
                t.purple
            }
        }
    }
}
