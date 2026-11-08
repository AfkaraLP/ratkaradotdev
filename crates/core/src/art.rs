use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

/// Colored ascii art: each cell's letter in `colors` picks its color from `palette`.
pub struct Art {
    pub width: u16,
    chars: &'static str,
    colors: &'static str,
    palette: &'static [u32],
}

/// The `YouTube` avatar, largest first.
pub const BIRD: [Art; 2] = [
    Art {
        width: 48,
        chars: include_str!("../art/bird48.txt"),
        colors: include_str!("../art/bird48.colors"),
        palette: &[
            0x00de_dbc6,
            0x00d2_ccb3,
            0x00de_bf71,
            0x00be_bfb9,
            0x00d4_b070,
            0x00ba_ae9d,
            0x00ab_acaa,
            0x00c1_9883,
            0x00a5_9992,
            0x0099_9595,
            0x0099_8a85,
            0x00af_7657,
            0x0083_8387,
            0x0072_6f78,
        ],
    },
    Art {
        width: 30,
        chars: include_str!("../art/bird30.txt"),
        colors: include_str!("../art/bird30.colors"),
        palette: &[
            0x00dd_d3a7,
            0x00c6_c8c1,
            0x00df_bc69,
            0x00ba_bebc,
            0x00cc_af79,
            0x00b8_b0a5,
            0x00aa_a9a8,
            0x00be_9a86,
            0x00a4_9895,
            0x0099_9799,
            0x0093_8f8d,
            0x009e_8a81,
            0x00b5_7852,
            0x007f_7c82,
        ],
    },
];

impl Art {
    /// The largest art that fits in `width`.
    #[must_use]
    pub fn fitting(arts: &'static [Art], width: u16) -> Option<&'static Art> {
        arts.iter().find(|art| art.width <= width)
    }

    /// The art centered in `width`.
    #[must_use]
    pub fn lines(&self, width: u16) -> Vec<Line<'static>> {
        let pad = " ".repeat(usize::from(width.saturating_sub(self.width) / 2));
        self.chars
            .lines()
            .zip(self.colors.lines())
            .map(|(chars, colors)| {
                let cells = chars
                    .chars()
                    .zip(colors.chars().chain(std::iter::repeat(' ')));
                let mut spans = vec![Span::raw(pad.clone())];
                for (ch, key) in cells {
                    let style = self.style(key);
                    match spans.last_mut() {
                        Some(span) if span.style == style => span.content.to_mut().push(ch),
                        _ => spans.push(Span::styled(ch.to_string(), style)),
                    }
                }
                Line::from(spans)
            })
            .collect()
    }

    fn style(&self, key: char) -> Style {
        let color = u32::from(key)
            .checked_sub(u32::from('a'))
            .and_then(|idx| self.palette.get(idx as usize));
        color.map_or_else(Style::new, |&rgb| Style::new().fg(Color::from_u32(rgb)))
    }
}
