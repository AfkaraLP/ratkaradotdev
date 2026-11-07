use ratatui::style::{Color, Modifier, Style};

pub struct Theme {
    pub base: Style,
    pub accent: Style,
    pub heading: Style,
    pub muted: Style,
    pub code: Style,
    pub link: Style,
    pub tag: Style,
    pub selected: Style,
    pub border: Style,
    pub border_focus: Style,
}

pub const ANSI: Theme = Theme {
    base: Style::new(),
    accent: Style::new().fg(Color::Yellow),
    heading: Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    muted: Style::new().fg(Color::DarkGray),
    code: Style::new().fg(Color::Green),
    link: Style::new()
        .fg(Color::Cyan)
        .add_modifier(Modifier::UNDERLINED),
    tag: Style::new().fg(Color::Magenta),
    selected: Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    border: Style::new().fg(Color::DarkGray),
    border_focus: Style::new().fg(Color::Yellow),
};

/// <https://github.com/casperstorm/ferra>
pub mod ferra {
    use ratatui::style::Color;

    pub const NIGHT: Color = Color::from_u32(0x002b_292d);
    pub const UMBER: Color = Color::from_u32(0x004d_424b);
    pub const BARK: Color = Color::from_u32(0x006f_5d63);
    pub const MIST: Color = Color::from_u32(0x00d1_d1e0);
    pub const SAGE: Color = Color::from_u32(0x00b1_b695);
    pub const BLUSH: Color = Color::from_u32(0x00fe_cdb2);
    pub const CORAL: Color = Color::from_u32(0x00ff_a07a);
    pub const ROSE: Color = Color::from_u32(0x00f6_b6c9);
    pub const HONEY: Color = Color::from_u32(0x00f5_d76e);
}

pub const FERRA: Theme = Theme {
    base: Style::new().fg(ferra::MIST).bg(ferra::NIGHT),
    accent: Style::new().fg(ferra::BLUSH),
    heading: Style::new().fg(ferra::CORAL).add_modifier(Modifier::BOLD),
    muted: Style::new().fg(ferra::BARK),
    code: Style::new().fg(ferra::SAGE),
    link: Style::new()
        .fg(ferra::ROSE)
        .add_modifier(Modifier::UNDERLINED),
    tag: Style::new().fg(ferra::HONEY),
    selected: Style::new().fg(ferra::CORAL).add_modifier(Modifier::BOLD),
    border: Style::new().fg(ferra::UMBER),
    border_focus: Style::new().fg(ferra::BARK),
};
