use std::ops::Range;

use ratatui::{
    style::Style,
    text::{Line, Span},
};
use unicode_width::UnicodeWidthChar;

/// A link before wrapping: these spans of this line point at `url`.
pub struct Anchor {
    pub line: usize,
    pub spans: Range<usize>,
    pub url: String,
}

/// A link after wrapping, as the cell ranges it covers on each row.
pub struct Link {
    pub url: String,
    pub cells: Vec<(usize, Range<u16>)>,
    /// The list entry it belongs to, in list views.
    pub entry: Option<usize>,
}

impl Link {
    #[must_use]
    pub fn row(&self) -> usize {
        self.cells.first().map_or(0, |(row, _)| *row)
    }
}

pub struct Wrapped {
    pub lines: Vec<Line<'static>>,
    pub links: Vec<Link>,
}

/// One cell's worth of text and the span it came from.
#[derive(Clone, Copy)]
struct Glyph {
    ch: char,
    width: u16,
    style: Style,
    span: usize,
}

/// Greedy word wrap that keeps leading indentation and drops the whitespace at row breaks.
#[must_use]
pub fn wrap(lines: Vec<Line<'static>>, anchors: &[Anchor], width: u16) -> Wrapped {
    let width = width.max(1);
    let mut out = Wrapped {
        lines: Vec::new(),
        links: Vec::new(),
    };
    for (idx, line) in lines.into_iter().enumerate() {
        let first = out.lines.len();
        let rows = wrap_line(&line, width);
        for anchor in anchors.iter().filter(|anchor| anchor.line == idx) {
            let cells = rows
                .iter()
                .enumerate()
                .filter_map(|(row, glyphs)| {
                    let cols = columns(glyphs, &anchor.spans)?;
                    Some((first + row, cols))
                })
                .collect();
            out.links.push(Link {
                url: anchor.url.clone(),
                cells,
                entry: None,
            });
        }
        out.lines.extend(rows.iter().map(|glyphs| to_line(glyphs)));
    }
    out
}

/// Spans that are a url on their own, as anchors to themselves.
#[must_use]
pub fn url_anchors(lines: &[Line<'static>]) -> Vec<Anchor> {
    lines
        .iter()
        .enumerate()
        .flat_map(|(line, text)| {
            text.spans
                .iter()
                .enumerate()
                .filter_map(move |(span, part)| {
                    let url = part.content.trim();
                    url.starts_with("http").then(|| Anchor {
                        line,
                        spans: span..span + 1,
                        url: url.to_string(),
                    })
                })
        })
        .collect()
}

impl Link {
    /// Moves the link by whole rows and columns, for text placed after other text.
    #[must_use]
    pub fn shifted(self, rows: usize, cols: u16, entry: Option<usize>) -> Self {
        let cells = self
            .cells
            .into_iter()
            .map(|(row, range)| (row + rows, range.start + cols..range.end + cols))
            .collect();
        Self {
            cells,
            entry,
            ..self
        }
    }
}

fn wrap_line(line: &Line<'static>, width: u16) -> Vec<Vec<Glyph>> {
    let glyphs = line.spans.iter().enumerate().flat_map(|(span, part)| {
        let style = line.style.patch(part.style);
        part.content.chars().map(move |ch| Glyph {
            ch,
            width: u16::try_from(ch.width().unwrap_or(0)).unwrap_or(0),
            style,
            span,
        })
    });

    let mut words: Vec<Vec<Glyph>> = Vec::new();
    for glyph in glyphs {
        let space = glyph.ch.is_whitespace();
        match words.last_mut() {
            Some(word)
                if word
                    .last()
                    .is_some_and(|last| last.ch.is_whitespace() == space) =>
            {
                word.push(glyph);
            }
            _ => words.push(vec![glyph]),
        }
    }

    let mut rows = vec![Vec::new()];
    let mut used = 0;
    for word in words {
        let space = word.first().is_some_and(|glyph| glyph.ch.is_whitespace());
        let len: u16 = word.iter().map(|glyph| glyph.width).sum();
        match (space, used + len > width) {
            (true, true) => {
                rows.push(Vec::new());
                used = 0;
            }
            (true, false) if rows.len() > 1 && used == 0 => {}
            (false, true) if used > 0 => {
                rows.push(Vec::new());
                used = 0;
                place(&mut rows, &mut used, word, width);
            }
            _ => place(&mut rows, &mut used, word, width),
        }
    }
    for row in &mut rows {
        while row.last().is_some_and(|glyph| glyph.ch.is_whitespace()) {
            row.pop();
        }
    }
    rows
}

/// Appends glyphs to the last row, hard breaking words wider than a whole row.
fn place(rows: &mut Vec<Vec<Glyph>>, used: &mut u16, glyphs: Vec<Glyph>, width: u16) {
    for glyph in glyphs {
        if *used > 0 && *used + glyph.width > width {
            rows.push(Vec::new());
            *used = 0;
        }
        *used += glyph.width;
        rows.last_mut().into_iter().for_each(|row| row.push(glyph));
    }
}

fn columns(glyphs: &[Glyph], spans: &Range<usize>) -> Option<Range<u16>> {
    let mut col = 0;
    let mut range: Option<Range<u16>> = None;
    for glyph in glyphs {
        if spans.contains(&glyph.span) {
            let start = range.as_ref().map_or(col, |range| range.start);
            range = Some(start..col + glyph.width);
        }
        col += glyph.width;
    }
    range
}

fn to_line(glyphs: &[Glyph]) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut last = None;
    for glyph in glyphs {
        match spans.last_mut() {
            Some(span) if last == Some((glyph.span, glyph.style)) => {
                span.content.to_mut().push(glyph.ch);
            }
            _ => spans.push(Span::styled(glyph.ch.to_string(), glyph.style)),
        }
        last = Some((glyph.span, glyph.style));
    }
    Line::from(spans)
}
