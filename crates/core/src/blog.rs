use std::{collections::HashMap, ops::Range};

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use crate::{theme::Theme, wrap::Anchor};

include!(concat!(env!("OUT_DIR"), "/posts.rs"));

pub struct Post {
    pub slug: &'static str,
    pub title: &'static str,
    pub date: &'static str,
    pub description: String,
    pub body: &'static str,
}

#[must_use]
pub fn load_posts() -> Vec<Post> {
    let mut posts: Vec<Post> = RAW_POSTS
        .iter()
        .map(|(slug, raw)| parse_post(slug, raw))
        .collect();
    posts.sort_by(|a, b| b.date.cmp(a.date));
    posts
}

fn parse_post(slug: &'static str, raw: &'static str) -> Post {
    let (frontmatter, body) = raw
        .strip_prefix("---")
        .and_then(|rest| rest.split_once("---"))
        .unwrap_or(("", raw));
    let meta: HashMap<&str, &str> = frontmatter
        .lines()
        .filter_map(|line| line.split_once(':'))
        .map(|(key, value)| (key.trim(), value.trim().trim_matches(['"', '\''])))
        .collect();
    let body = body.trim();
    Post {
        slug,
        title: meta.get("title").copied().unwrap_or("Untitled"),
        date: meta.get("date").copied().unwrap_or("Unknown"),
        description: meta
            .get("description")
            .map_or_else(|| excerpt(body, 200), ToString::to_string),
        body,
    }
}

#[must_use]
pub fn excerpt(body: &str, max: usize) -> String {
    let mut text = String::new();
    for event in Parser::new(body) {
        match event {
            Event::Text(t) | Event::Code(t) => text.push_str(&t),
            Event::SoftBreak | Event::HardBreak | Event::End(TagEnd::Paragraph) => text.push(' '),
            _ => {}
        }
        if text.chars().count() > max {
            break;
        }
    }
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.chars().count() <= max {
        return text;
    }
    let mut cut = String::new();
    for word in text.split(' ') {
        if cut.chars().count() + word.chars().count() + 1 > max {
            break;
        }
        if !cut.is_empty() {
            cut.push(' ');
        }
        cut.push_str(word);
    }
    format!("{}…", cut.trim_end_matches([',', '.', ':', ';']))
}

pub struct Markdown {
    pub lines: Vec<Line<'static>>,
    pub anchors: Vec<Anchor>,
}

/// `expand` writes each link's url after its text, for output that can't be clicked.
#[must_use]
pub fn render_markdown(src: &str, theme: &'static Theme, expand: bool) -> Markdown {
    Renderer {
        theme,
        expand,
        lines: Vec::new(),
        anchors: Vec::new(),
        pending: Vec::new(),
        spans: Vec::new(),
        styles: Vec::new(),
        lists: Vec::new(),
        quote_depth: 0,
        code_block: false,
        link: None,
    }
    .run(src)
}

struct Renderer {
    theme: &'static Theme,
    expand: bool,
    lines: Vec<Line<'static>>,
    anchors: Vec<Anchor>,
    /// Finished links on the current line, by span.
    pending: Vec<(Range<usize>, String)>,
    spans: Vec<Span<'static>>,
    styles: Vec<Style>,
    /// `None` for bullet lists, `Some(n)` for the next number of an ordered list.
    lists: Vec<Option<u64>>,
    quote_depth: usize,
    code_block: bool,
    /// The link being written: its first span and url.
    link: Option<(usize, String)>,
}

impl Renderer {
    fn run(mut self, src: &str) -> Markdown {
        let options = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES;
        for event in Parser::new_ext(src, options) {
            self.event(event);
        }
        self.flush();
        while self.lines.last().is_some_and(|line| line.spans.is_empty()) {
            self.lines.pop();
        }
        Markdown {
            lines: self.lines,
            anchors: self.anchors,
        }
    }

    fn style(&self) -> Style {
        self.styles.last().copied().unwrap_or_default()
    }

    fn push_style(&mut self, f: impl FnOnce(Style) -> Style) {
        self.styles.push(f(self.style()));
    }

    fn flush(&mut self) {
        if self.spans.is_empty() {
            return;
        }
        let mut spans = Vec::new();
        if self.quote_depth > 0 {
            spans.push(Span::styled(
                "▌ ".repeat(self.quote_depth),
                self.theme.muted,
            ));
        }
        let (line, shift) = (self.lines.len(), spans.len());
        self.anchors
            .extend(self.pending.drain(..).map(|(spans, url)| Anchor {
                line,
                spans: spans.start + shift..spans.end + shift,
                url,
            }));
        if let Some((start, _)) = &mut self.link {
            *start = 0;
        }
        spans.append(&mut self.spans);
        self.lines.push(Line::from(spans));
    }

    fn blank(&mut self) {
        self.flush();
        if self.lines.last().is_some_and(|line| !line.spans.is_empty()) {
            self.lines.push(Line::default());
        }
    }

    fn text(&mut self, text: &str) {
        if self.code_block {
            for line in text.lines() {
                self.lines.push(Line::from(vec![
                    Span::styled("│ ", self.theme.muted),
                    Span::styled(line.to_string(), self.theme.code),
                ]));
            }
            return;
        }
        let span = Span::styled(text.to_string(), self.style());
        self.spans.push(span);
    }

    fn event(&mut self, event: Event<'_>) {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(text) => self.text(&text),
            Event::Code(code) => self
                .spans
                .push(Span::styled(format!("`{code}`"), self.theme.code)),
            Event::SoftBreak => self.spans.push(Span::raw(" ")),
            Event::HardBreak => self.flush(),
            Event::Rule => {
                self.blank();
                self.lines
                    .push(Line::styled("─".repeat(40), self.theme.muted));
                self.lines.push(Line::default());
            }
            _ => {}
        }
    }

    fn start(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Heading { level, .. } => {
                self.blank();
                let prefix = "#".repeat(level as usize);
                let heading = self.theme.heading;
                self.push_style(|_| heading);
                self.spans
                    .push(Span::styled(format!("{prefix} "), self.theme.muted));
            }
            Tag::BlockQuote(_) => {
                self.blank();
                self.quote_depth += 1;
                self.push_style(Style::italic);
            }
            Tag::CodeBlock(kind) => {
                self.blank();
                let lang = match kind {
                    CodeBlockKind::Fenced(lang) if !lang.is_empty() => lang.to_uppercase(),
                    _ => "CODE".into(),
                };
                self.lines
                    .push(Line::styled(format!("╭─ {lang}"), self.theme.muted));
                self.code_block = true;
            }
            Tag::List(start) => {
                self.flush();
                if self.lists.is_empty() {
                    self.blank();
                }
                self.lists.push(start);
            }
            Tag::Item => {
                self.flush();
                let indent = "  ".repeat(self.lists.len().saturating_sub(1));
                let marker = match self.lists.last_mut() {
                    Some(Some(n)) => {
                        *n += 1;
                        format!("{indent}{}. ", *n - 1)
                    }
                    _ => format!("{indent}• "),
                };
                self.spans.push(Span::styled(marker, self.theme.accent));
            }
            Tag::Emphasis => self.push_style(Style::italic),
            Tag::Strong => self.push_style(Style::bold),
            Tag::Strikethrough => self.push_style(Style::crossed_out),
            Tag::Link { dest_url, .. } => {
                let link = self.theme.link;
                self.push_style(|s| s.patch(link));
                self.link = Some((self.spans.len(), dest_url.to_string()));
            }
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph => self.blank(),
            TagEnd::Heading(_) => {
                self.styles.pop();
                self.blank();
            }
            TagEnd::BlockQuote(_) => {
                self.styles.pop();
                self.flush();
                self.quote_depth -= 1;
                self.blank();
            }
            TagEnd::CodeBlock => {
                self.code_block = false;
                self.lines.push(Line::styled("╰─", self.theme.muted));
                self.lines.push(Line::default());
            }
            TagEnd::List(_) => {
                self.lists.pop();
                if self.lists.is_empty() {
                    self.blank();
                }
            }
            TagEnd::Item => self.flush(),
            TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough => {
                self.styles.pop();
            }
            TagEnd::Link => {
                self.styles.pop();
                let Some((start, url)) = self.link.take() else {
                    return;
                };
                let muted = self.theme.muted.remove_modifier(Modifier::UNDERLINED);
                let expanded = self
                    .expand
                    .then(|| Span::styled(format!(" ({url})"), muted));
                self.pending.push((start..self.spans.len(), url));
                self.spans.extend(expanded);
            }
            _ => {}
        }
    }
}
