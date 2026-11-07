use ratatui::{
    buffer::Buffer,
    style::{Color, Modifier},
};

const MODIFIERS: [(Modifier, u8); 6] = [
    (Modifier::BOLD, 1),
    (Modifier::DIM, 2),
    (Modifier::ITALIC, 3),
    (Modifier::UNDERLINED, 4),
    (Modifier::REVERSED, 7),
    (Modifier::CROSSED_OUT, 9),
];

const COLORS: [Color; 16] = [
    Color::Black,
    Color::Red,
    Color::Green,
    Color::Yellow,
    Color::Blue,
    Color::Magenta,
    Color::Cyan,
    Color::Gray,
    Color::DarkGray,
    Color::LightRed,
    Color::LightGreen,
    Color::LightYellow,
    Color::LightBlue,
    Color::LightMagenta,
    Color::LightCyan,
    Color::White,
];

#[must_use]
pub fn to_ansi(buf: &Buffer) -> String {
    let area = buf.area;
    let mut out = String::new();
    for y in area.top()..area.bottom() {
        let cells: Vec<_> = (area.left()..area.right()).map(|x| &buf[(x, y)]).collect();
        let end = cells
            .iter()
            .rposition(|cell| cell.symbol() != " ")
            .map_or(0, |idx| idx + 1);
        let mut current = None;
        for cell in &cells[..end] {
            let style = (cell.fg, cell.bg, cell.modifier);
            if current != Some(style) {
                out.push_str(&sgr(style));
                current = Some(style);
            }
            out.push_str(cell.symbol());
        }
        out.push_str("\x1b[0m\n");
    }
    out
}

fn sgr((fg, bg, modifier): (Color, Color, Modifier)) -> String {
    let codes: Vec<String> = std::iter::once("0".to_string())
        .chain(
            MODIFIERS
                .iter()
                .filter(|(m, _)| modifier.contains(*m))
                .map(|(_, code)| code.to_string()),
        )
        .chain(color(fg, 30))
        .chain(color(bg, 40))
        .collect();
    format!("\x1b[{}m", codes.join(";"))
}

fn color(color: Color, base: u8) -> Option<String> {
    match color {
        Color::Reset => None,
        Color::Rgb(r, g, b) => Some(format!("{};2;{r};{g};{b}", base + 8)),
        Color::Indexed(idx) => Some(format!("{};5;{idx}", base + 8)),
        named => COLORS
            .iter()
            .position(|&c| c == named)
            .and_then(|idx| u8::try_from(idx).ok())
            .map(|idx| (base + idx % 8 + 60 * (idx / 8)).to_string()),
    }
}
