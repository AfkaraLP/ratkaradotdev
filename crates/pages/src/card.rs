// pixel math: every cast here is between small, positive card coordinates
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

use std::path::Path;

use ab_glyph::{Font, FontArc, PxScale, ScaleFont, point};
use anyhow::Context;
use ratkara_core::{
    art::BIRD,
    content,
    ratatui::style::Color,
    theme::ferra::{BARK, BLUSH, CORAL, MIST, NIGHT, UMBER},
};
use tiny_skia::{Paint, PathBuilder, Pixmap, PremultipliedColorU8, Stroke, Transform};

const WIDTH: u32 = 1200;
const HEIGHT: u32 = 630;
/// Where the text column starts, right of the bird.
const TEXT_X: f32 = 560.0;
const TEXT_WIDTH: f32 = 580.0;

pub struct Fonts {
    regular: FontArc,
    bold: FontArc,
}

impl Fonts {
    pub fn load(dir: &Path) -> anyhow::Result<Self> {
        let load = |name: &str| -> anyhow::Result<FontArc> {
            let path = dir.join(name);
            let bytes =
                std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
            Ok(FontArc::try_from_vec(bytes)?)
        };
        Ok(Self {
            regular: load("FiraMono-Regular.ttf")?,
            bold: load("FiraMono-Bold.ttf")?,
        })
    }
}

pub enum Card<'a> {
    Site,
    Post {
        title: &'a str,
        date: &'a str,
        excerpt: &'a str,
    },
}

/// One run of text: font, size in px, color, and the lines to draw.
struct Block<'a> {
    font: &'a FontArc,
    size: f32,
    color: Color,
    lines: Vec<String>,
}

impl Card<'_> {
    pub fn png(&self, fonts: &Fonts) -> anyhow::Result<Vec<u8>> {
        let mut pixmap = Pixmap::new(WIDTH, HEIGHT).context("allocating the card")?;
        pixmap.fill(skia(NIGHT));
        border(&mut pixmap)?;
        bird(&mut pixmap, &fonts.regular);

        let blocks = match self {
            Card::Site => vec![
                Block::new(&fonts.bold, 84.0, CORAL, vec![content::NAME.into()]),
                Block::new(&fonts.regular, 28.0, MIST, vec![content::TAGLINE.into()]),
                Block::new(&fonts.regular, 28.0, BLUSH, vec![content::HEADLINE.into()]),
            ],
            Card::Post {
                title,
                date,
                excerpt,
            } => vec![
                Block::wrapped(&fonts.bold, 46.0, CORAL, title, 3),
                Block::new(&fonts.regular, 24.0, BARK, vec![(*date).into()]),
                Block::wrapped(&fonts.regular, 24.0, MIST, excerpt, 5),
            ],
        };
        let footer = Block::new(&fonts.regular, 24.0, BARK, vec!["afkara.dev".into()]);

        // the blocks sit centered in the column, the footer at the bottom
        let gap = 26.0;
        let height = blocks.iter().map(|block| block.height() + gap).sum::<f32>() - gap;
        let mut y = (HEIGHT as f32 - 80.0 - height) / 2.0;
        for block in &blocks {
            block.draw(&mut pixmap, TEXT_X, y);
            y += block.height() + gap;
        }
        footer.draw(&mut pixmap, TEXT_X, HEIGHT as f32 - 110.0);

        Ok(pixmap.encode_png()?)
    }
}

impl<'a> Block<'a> {
    fn new(font: &'a FontArc, size: f32, color: Color, lines: Vec<String>) -> Self {
        Self {
            font,
            size,
            color,
            lines,
        }
    }

    /// Word wrapped to the text column, at most `max` lines, the last one cut with "…".
    fn wrapped(font: &'a FontArc, size: f32, color: Color, text: &str, max: usize) -> Self {
        let advance = font
            .as_scaled(PxScale::from(size))
            .h_advance(font.glyph_id('M'));
        let per_line = (TEXT_WIDTH / advance) as usize;
        let mut lines: Vec<String> = Vec::new();
        for word in text.split_whitespace() {
            match lines.last_mut() {
                Some(line) if line.chars().count() + 1 + word.chars().count() <= per_line => {
                    line.push(' ');
                    line.push_str(word);
                }
                _ => lines.push(word.to_string()),
            }
        }
        if lines.len() > max {
            lines.truncate(max);
            if let Some(last) = lines.last_mut() {
                let keep = per_line.saturating_sub(1).min(last.chars().count());
                *last = format!(
                    "{}…",
                    last.chars().take(keep).collect::<String>().trim_end()
                );
            }
        }
        Self::new(font, size, color, lines)
    }

    fn line_height(&self) -> f32 {
        self.size * 1.25
    }

    fn height(&self) -> f32 {
        self.line_height() * self.lines.len() as f32
    }

    fn draw(&self, pixmap: &mut Pixmap, x: f32, y: f32) {
        for (idx, line) in self.lines.iter().enumerate() {
            text(
                pixmap,
                self.font,
                self.size,
                x,
                y + idx as f32 * self.line_height(),
                line,
                self.color,
            );
        }
    }
}

fn border(pixmap: &mut Pixmap) -> anyhow::Result<()> {
    let (x0, y0, x1, y1, r) = (24.0, 24.0, WIDTH as f32 - 24.0, HEIGHT as f32 - 24.0, 18.0);
    let mut path = PathBuilder::new();
    path.move_to(x0 + r, y0);
    path.line_to(x1 - r, y0);
    path.quad_to(x1, y0, x1, y0 + r);
    path.line_to(x1, y1 - r);
    path.quad_to(x1, y1, x1 - r, y1);
    path.line_to(x0 + r, y1);
    path.quad_to(x0, y1, x0, y1 - r);
    path.line_to(x0, y0 + r);
    path.quad_to(x0, y0, x0 + r, y0);
    path.close();
    let path = path.finish().context("building the border")?;
    let mut paint = Paint::default();
    paint.set_color(skia(UMBER));
    paint.anti_alias = true;
    let stroke = Stroke {
        width: 3.0,
        ..Stroke::default()
    };
    pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    Ok(())
}

fn bird(pixmap: &mut Pixmap, font: &FontArc) {
    let art = &BIRD[0];
    let size = 15.0;
    let cell = font
        .as_scaled(PxScale::from(size))
        .h_advance(font.glyph_id('M'));
    let line = 17.2;
    let rows = art
        .cells()
        .map(|(row, ..)| row + 1)
        .max()
        .unwrap_or_default();
    let top = (HEIGHT as f32 - rows as f32 * line) / 2.0;
    for (row, col, ch, rgb) in art.cells() {
        let (x, y) = (70.0 + col as f32 * cell, top + row as f32 * line);
        text(
            pixmap,
            font,
            size,
            x,
            y,
            &ch.to_string(),
            Color::from_u32(rgb),
        );
    }
}

/// Draws one line of text with its top at `y`.
fn text(
    pixmap: &mut Pixmap,
    font: &FontArc,
    size: f32,
    left: f32,
    top: f32,
    line: &str,
    color: Color,
) {
    let scaled = font.as_scaled(PxScale::from(size));
    let paint = rgb(color);
    let mut caret = left;
    for ch in line.chars() {
        let id = font.glyph_id(ch);
        let glyph =
            id.with_scale_and_position(PxScale::from(size), point(caret, top + scaled.ascent()));
        caret += scaled.h_advance(id);
        let Some(outline) = font.outline_glyph(glyph) else {
            continue;
        };
        let bounds = outline.px_bounds();
        outline.draw(|gx, gy, coverage| {
            let (px, py) = (
                bounds.min.x as i32 + gx as i32,
                bounds.min.y as i32 + gy as i32,
            );
            blend(pixmap, px, py, paint, coverage);
        });
    }
}

fn blend(pixmap: &mut Pixmap, x: i32, y: i32, color: [u8; 3], coverage: f32) {
    let (Ok(x), Ok(y)) = (u32::try_from(x), u32::try_from(y)) else {
        return;
    };
    if x >= WIDTH || y >= HEIGHT {
        return;
    }
    let idx = (y * WIDTH + x) as usize;
    let pixels = pixmap.pixels_mut();
    let under = pixels[idx];
    let mix = |src: u8, dst: u8| {
        (f32::from(src) * coverage + f32::from(dst) * (1.0 - coverage)).round() as u8
    };
    let mixed = [
        mix(color[0], under.red()),
        mix(color[1], under.green()),
        mix(color[2], under.blue()),
    ];
    if let Some(px) = PremultipliedColorU8::from_rgba(mixed[0], mixed[1], mixed[2], 255) {
        pixels[idx] = px;
    }
}

fn rgb(color: Color) -> [u8; 3] {
    match color {
        Color::Rgb(r, g, b) => [r, g, b],
        _ => [255, 255, 255],
    }
}

fn skia(color: Color) -> tiny_skia::Color {
    let [r, g, b] = rgb(color);
    tiny_skia::Color::from_rgba8(r, g, b, 255)
}
