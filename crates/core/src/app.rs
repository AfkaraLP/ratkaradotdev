use std::ops::Range;

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Margin, Position, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Paragraph, Widget, Wrap},
};

use crate::{
    art::{Art, BIRD},
    blog::{Post, excerpt, load_posts, render_markdown},
    content::{self, BOOKMARKS, CONTACTS, PROJECTS},
    theme::{self, Theme},
    wrap::{Anchor, Link, url_anchors, wrap},
};

const MAX_WIDTH: u16 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Up,
    Down,
    Left,
    Right,
    Enter,
    Esc,
    Tab,
    BackTab,
    PageUp,
    PageDown,
    Home,
    End,
    Backspace,
    CtrlC,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    None,
    Open(String),
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Web,
    Ssh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    About,
    Projects,
    Blog,
    Bookmarks,
    Contact,
}

impl Tab {
    pub const ALL: [Tab; 5] = [
        Tab::About,
        Tab::Projects,
        Tab::Blog,
        Tab::Bookmarks,
        Tab::Contact,
    ];

    #[must_use]
    pub fn title(self) -> &'static str {
        match self {
            Tab::About => "About",
            Tab::Projects => "Projects",
            Tab::Blog => "Blog",
            Tab::Bookmarks => "Bookmarks",
            Tab::Contact => "Contact",
        }
    }

    fn index(self) -> usize {
        Tab::ALL.iter().position(|&t| t == self).unwrap_or_default()
    }

    fn offset(self, by: isize) -> Self {
        let len = Tab::ALL.len().cast_signed();
        let idx = (self.index().cast_signed() + by).rem_euclid(len);
        Tab::ALL[idx.cast_unsigned()]
    }

    fn slug(self) -> String {
        self.title().to_lowercase()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Tab(Tab),
    Post(usize),
    NotFound,
}

impl Page {
    fn tab(self) -> Tab {
        match self {
            Page::Tab(tab) => tab,
            Page::Post(_) | Page::NotFound => Tab::Blog,
        }
    }
}

/// Footer hint: its keys, each clickable on its own, and a label.
type Hint = (&'static [(&'static str, Key)], &'static str);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Hit {
    Tab(Tab),
    Key(Key),
    Item(usize),
    Link(usize),
}

#[derive(Default)]
struct Frame {
    hits: Vec<(Rect, Hit)>,
    links: Vec<Link>,
    height: u16,
    max_scroll: u16,
}

/// Laid out rows, ready to render as they are.
struct View {
    lines: Vec<Line<'static>>,
    entries: Vec<Range<usize>>,
    links: Vec<Link>,
}

#[derive(Clone, Copy)]
enum Mode<'a> {
    Tui,
    /// Static output, with this base url for links.
    Print(&'a str),
}

pub struct App {
    platform: Platform,
    theme: &'static Theme,
    posts: Vec<Post>,
    page: Page,
    selected: [usize; Tab::ALL.len()],
    scroll: u16,
    visual: Option<usize>,
    hover: Option<Position>,
    status: Option<String>,
    stats: Vec<Option<u32>>,
    year: u32,
    last: Frame,
}

impl App {
    #[must_use]
    pub fn new(platform: Platform, year: u32) -> Self {
        Self {
            platform,
            theme: match platform {
                Platform::Web => &theme::FERRA,
                Platform::Ssh => &theme::ANSI,
            },
            posts: load_posts(),
            page: Page::Tab(Tab::About),
            selected: [0; Tab::ALL.len()],
            scroll: 0,
            visual: None,
            hover: None,
            status: None,
            stats: vec![None; PROJECTS.len()],
            year,
            last: Frame::default(),
        }
    }

    pub fn set_stats(&mut self, stats: &[Option<u32>]) {
        let len = self.stats.len().min(stats.len());
        self.stats[..len].clone_from_slice(&stats[..len]);
    }

    pub fn set_stat(&mut self, idx: usize, stat: u32) {
        if let Some(slot) = self.stats.get_mut(idx) {
            *slot = Some(stat);
        }
    }

    pub fn notify(&mut self, msg: impl Into<String>) {
        self.status = Some(msg.into());
    }

    #[must_use]
    pub fn routes(&self) -> Vec<String> {
        let tabs = Tab::ALL.into_iter().map(Tab::slug);
        let posts = self.posts.iter().map(|post| format!("blog/{}", post.slug));
        tabs.chain(posts).collect()
    }

    #[must_use]
    pub fn route(&self) -> String {
        match self.page {
            Page::Tab(tab) => tab.slug(),
            Page::Post(idx) => format!("blog/{}", self.posts[idx].slug),
            Page::NotFound => "404".into(),
        }
    }

    #[must_use]
    pub fn title(&self) -> String {
        let page = match self.page {
            Page::Tab(Tab::About) => return content::NAME.to_string(),
            Page::Tab(tab) => tab.title(),
            Page::Post(idx) => self.posts[idx].title,
            Page::NotFound => "Not Found",
        };
        format!("{page} · {}", content::NAME)
    }

    #[must_use]
    pub fn description(&self) -> String {
        match self.page {
            Page::Tab(Tab::About) => format!("{}. {}", content::HEADLINE, content::SUBHEADLINE),
            Page::Tab(Tab::Projects) => {
                "Things I've built: Rust tools, models and datasets.".into()
            }
            Page::Tab(Tab::Blog) => "Posts about whatever I'm working on.".into(),
            Page::Tab(Tab::Bookmarks) => "Papers and books worth reading.".into(),
            Page::Tab(Tab::Contact) => content::CONTACT_TEXT.into(),
            Page::Post(idx) => excerpt(self.posts[idx].body, 160),
            Page::NotFound => "That page doesn't exist.".into(),
        }
    }

    #[must_use]
    pub fn date(&self) -> Option<&str> {
        match self.page {
            Page::Post(idx) => Some(self.posts[idx].date),
            Page::Tab(_) | Page::NotFound => None,
        }
    }

    #[must_use]
    pub fn is_not_found(&self) -> bool {
        self.page == Page::NotFound
    }

    pub fn goto(&mut self, route: &str) {
        let route = route.trim_matches(['#', '/']);
        let tab = Tab::ALL.into_iter().find(|tab| tab.slug() == route);
        let post = route
            .strip_prefix("blog/")
            .and_then(|slug| self.posts.iter().position(|post| post.slug == slug));
        let page = match (route, tab, post) {
            ("", ..) => return,
            (_, Some(tab), _) => Page::Tab(tab),
            (_, _, Some(idx)) => Page::Post(idx),
            _ => Page::NotFound,
        };
        self.set_page(page);
    }

    fn set_page(&mut self, page: Page) {
        if let Page::Post(idx) = page {
            self.selected[Tab::Blog.index()] = idx;
        }
        self.page = page;
        self.scroll = 0;
        self.visual = None;
    }

    fn item_count(&self) -> Option<usize> {
        match self.page {
            Page::Tab(Tab::About) | Page::Post(_) => None,
            Page::Tab(Tab::Projects) => Some(PROJECTS.len()),
            Page::Tab(Tab::Blog) | Page::NotFound => Some(self.posts.len()),
            Page::Tab(Tab::Bookmarks) => Some(BOOKMARKS.len()),
            Page::Tab(Tab::Contact) => Some(CONTACTS.len()),
        }
    }

    fn selected(&self) -> usize {
        self.selected[self.page.tab().index()]
    }

    fn select(&mut self, idx: usize) {
        self.selected[self.page.tab().index()] = idx;
    }

    fn step(&mut self, by: isize) {
        match self.item_count() {
            Some(0) => {}
            Some(len) => self.select(self.selected().saturating_add_signed(by).min(len - 1)),
            None => {
                let by = i16::try_from(by).unwrap_or(i16::MAX);
                self.scroll = self
                    .scroll
                    .saturating_add_signed(by)
                    .min(self.last.max_scroll);
            }
        }
    }

    fn activate(&mut self) -> Action {
        let sel = self.selected();
        match self.page {
            Page::Tab(Tab::Blog) | Page::NotFound if sel < self.posts.len() => {
                self.set_page(Page::Post(sel));
                Action::None
            }
            Page::Tab(Tab::Projects) => Action::Open(PROJECTS[sel].url()),
            Page::Tab(Tab::Bookmarks) => Action::Open(BOOKMARKS[sel].url.into()),
            Page::Tab(Tab::Contact) => Action::Open(CONTACTS[sel].url.into()),
            _ => Action::None,
        }
    }

    fn back(&mut self) -> Action {
        match self.page {
            Page::Post(_) | Page::NotFound => self.set_page(Page::Tab(Tab::Blog)),
            Page::Tab(Tab::About) => {}
            Page::Tab(_) => self.set_page(Page::Tab(Tab::About)),
        }
        Action::None
    }

    pub fn handle_key(&mut self, key: Key) -> Action {
        self.status = None;
        if let Some(current) = self.visual {
            return self.handle_visual(key, current);
        }
        let page =
            isize::from(i16::try_from(self.last.height.saturating_sub(2).max(1)).unwrap_or(1));
        let tab = self.page.tab();
        match key {
            Key::CtrlC => return Action::Quit,
            Key::Char('q') if self.platform == Platform::Ssh => return Action::Quit,
            Key::Char('v') => self.enter_visual(),
            Key::Char(c @ '1'..='5') => {
                self.set_page(Page::Tab(Tab::ALL[c as usize - '1' as usize]));
            }
            Key::Tab | Key::Right | Key::Char('l') => self.set_page(Page::Tab(tab.offset(1))),
            Key::BackTab | Key::Left | Key::Char('h') => self.set_page(Page::Tab(tab.offset(-1))),
            Key::Down | Key::Char('j') => self.step(1),
            Key::Up | Key::Char('k') => self.step(-1),
            Key::PageDown | Key::Char(' ') => self.step(page),
            Key::PageUp => self.step(-page),
            Key::Home | Key::Char('g') => self.step(isize::MIN / 2),
            Key::End | Key::Char('G') => self.step(isize::MAX / 2),
            Key::Enter => return self.activate(),
            Key::Esc | Key::Backspace => return self.back(),
            Key::Char(_) => {}
        }
        Action::None
    }

    fn enter_visual(&mut self) {
        let (sel, top) = (self.selected(), usize::from(self.scroll));
        let links = &self.last.links;
        let start = links.iter().position(|link| match link.entry {
            Some(entry) => entry >= sel,
            None => link.row() >= top,
        });
        self.visual = (!links.is_empty()).then(|| start.unwrap_or(0));
    }

    fn handle_visual(&mut self, key: Key, current: usize) -> Action {
        let len = self.last.links.len().max(1);
        let next = match key {
            Key::CtrlC => return Action::Quit,
            Key::Down | Key::Tab | Key::Char('j' | 'n' | 'l') => (current + 1) % len,
            Key::Up | Key::BackTab | Key::Char('k' | 'N' | 'h') => (current + len - 1) % len,
            Key::Home | Key::Char('g') => 0,
            Key::End | Key::Char('G') => len - 1,
            Key::Enter | Key::Char('o' | 'y') => {
                self.visual = None;
                return self
                    .last
                    .links
                    .get(current)
                    .map_or(Action::None, |link| Action::Open(link.url.clone()));
            }
            Key::Esc | Key::Backspace | Key::Char('v' | 'q') => {
                self.visual = None;
                return Action::None;
            }
            _ => current,
        };
        self.visual = Some(next);
        if let Some(entry) = self.last.links.get(next).and_then(|link| link.entry) {
            self.select(entry);
        }
        Action::None
    }

    /// Moves the pointer, `None` once it leaves. Returns whether a click there would do something.
    pub fn hover(&mut self, at: Option<(u16, u16)>) -> bool {
        self.hover = at.map(|(col, row)| Position::new(col, row));
        self.hovered().is_some()
    }

    fn hovered(&self) -> Option<(Rect, Hit)> {
        let pos = self.hover?;
        self.last
            .hits
            .iter()
            .rev()
            .find(|(rect, _)| rect.contains(pos))
            .copied()
    }

    pub fn click(&mut self, col: u16, row: u16) -> Action {
        self.status = None;
        self.visual = None;
        // taps on touch screens leave no pointer behind
        self.hover = None;
        let pos = Position::new(col, row);
        let hit = self
            .last
            .hits
            .iter()
            .rev()
            .find(|(rect, _)| rect.contains(pos));
        match hit.map(|&(_, hit)| hit) {
            Some(Hit::Tab(tab)) => {
                self.set_page(Page::Tab(tab));
                Action::None
            }
            Some(Hit::Key(key)) => self.handle_key(key),
            Some(Hit::Item(idx)) => {
                self.select(idx);
                self.activate()
            }
            Some(Hit::Link(idx)) => self
                .last
                .links
                .get(idx)
                .map_or(Action::None, |link| Action::Open(link.url.clone())),
            None => Action::None,
        }
    }

    pub fn draw(&mut self, frame: &mut ratatui::Frame) {
        self.last.hits.clear();
        let area = frame.area();
        frame.render_widget(Block::new().style(self.theme.base), area);
        let width = area.width.min(MAX_WIDTH);
        let area = Rect {
            x: area.x + (area.width - width) / 2,
            width,
            ..area
        };
        let tab_rows = tab_layout(area.width.saturating_sub(4));
        let header_height = 3 + u16::try_from(tab_rows.len()).unwrap_or(1);
        let [header, body, footer] = Layout::vertical([
            Constraint::Length(header_height),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(area);

        self.draw_header(frame, header, &tab_rows);

        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(self.theme.border_focus)
            .title(Line::styled(
                format!(" {} ", self.section()),
                self.theme.heading,
            ));
        let inner = block.inner(body).inner(Margin::new(1, 0));
        frame.render_widget(block, body);
        self.last.height = inner.height;

        let view = self.view(inner.width, Mode::Tui);
        match self.item_count() {
            Some(_) => self.draw_list(frame, inner, view),
            None => self.draw_text(frame, inner, view),
        }
        self.draw_footer(frame, footer);
        self.draw_hover(frame);
    }

    fn draw_hover(&self, frame: &mut ratatui::Frame) {
        let rects = match self.hovered() {
            Some((_, Hit::Link(idx))) => self
                .last
                .hits
                .iter()
                .filter(|(_, hit)| *hit == Hit::Link(idx))
                .map(|(rect, _)| *rect)
                .collect(),
            Some((_, Hit::Tab(tab))) if tab == self.page.tab() => Vec::new(),
            Some((rect, Hit::Tab(_) | Hit::Key(_) | Hit::Item(_))) => vec![rect],
            None => Vec::new(),
        };
        for rect in rects {
            frame.buffer_mut().set_style(rect, self.theme.hover);
        }
    }

    fn draw_header(
        &mut self,
        frame: &mut ratatui::Frame,
        area: Rect,
        tab_rows: &[Vec<(Tab, String)>],
    ) {
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(self.theme.border);
        let inner = block.inner(area).inner(Margin::new(1, 0));
        frame.render_widget(block, area);
        let [brand, tabs] =
            Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(inner);

        let mut name = vec![Span::styled(content::NAME, self.theme.heading)];
        if brand.width as usize > content::NAME.len() + content::TAGLINE.len() + 3 {
            name.push(Span::styled(" · ", self.theme.muted));
            name.push(Span::styled(content::TAGLINE, Style::new().italic()));
        }
        frame.render_widget(Line::from(name), brand);
        let name = Rect {
            width: u16::try_from(content::NAME.len()).unwrap_or_default(),
            ..brand
        };
        self.last
            .hits
            .push((name.intersection(brand), Hit::Tab(Tab::About)));

        for (y, row) in (tabs.y..).zip(tab_rows) {
            let mut x = tabs.x;
            for (tab, label) in row {
                let current = usize::from(*tab == self.page.tab());
                let style = [Style::new(), self.theme.selected.reversed()][current];
                let w = u16::try_from(label.len()).unwrap_or_default();
                let rect = Rect::new(x, y, w, 1).intersection(tabs);
                frame.render_widget(Span::styled(label.clone(), style), rect);
                self.last.hits.push((rect, Hit::Tab(*tab)));
                x = x.saturating_add(w + 1);
            }
        }
    }

    fn draw_footer(&mut self, frame: &mut ratatui::Frame, area: Rect) {
        let inner = area.inner(Margin::new(1, 0));
        if let Some(status) = &self.status {
            frame.render_widget(Line::styled(status.clone(), self.theme.accent), inner);
            return;
        }

        let arrows = |label| ([("↑", Key::Up), ("↓", Key::Down)].as_slice(), label);
        let tabs: Hint = (&[("←", Key::Left), ("→", Key::Right)], "tabs");
        let mut hints: Vec<Hint> = match (self.visual, self.item_count()) {
            (Some(_), _) => vec![
                (&[("-- VISUAL --", Key::Char('v'))], ""),
                (&[("j", Key::Char('j')), ("k", Key::Char('k'))], "next"),
                (&[("↵", Key::Enter)], "open"),
                (&[("esc", Key::Esc)], "exit"),
            ],
            (None, Some(_)) => vec![tabs, arrows("select"), (&[("↵", Key::Enter)], "open")],
            (None, None) => vec![tabs, arrows("scroll")],
        };
        let extras: [(bool, Hint); 3] = [
            (
                !self.last.links.is_empty(),
                (&[("v", Key::Char('v'))], "links"),
            ),
            (
                self.page != Page::Tab(Tab::About),
                (&[("esc", Key::Esc)], "back"),
            ),
            (
                self.platform == Platform::Ssh,
                (&[("q", Key::Char('q'))], "quit"),
            ),
        ];
        hints.extend(
            extras
                .into_iter()
                .filter(|&(shown, _)| shown && self.visual.is_none())
                .map(|(_, hint)| hint),
        );

        let glyphs =
            |keys: &[(&str, Key)]| keys.iter().map(|(g, _)| g.chars().count()).sum::<usize>();
        let label_width = |label: &str| (label.len() + 1) * usize::from(!label.is_empty());
        let hints_width: usize = hints
            .iter()
            .map(|(keys, label)| glyphs(keys) + label_width(label) + 1)
            .sum();
        let copyright = format!("© {} {}", self.year, content::NAME);
        if (inner.width as usize) >= hints_width + copyright.chars().count() {
            frame.render_widget(
                Line::styled(copyright, self.theme.muted).right_aligned(),
                inner,
            );
        }

        // every key is its own click target, the label does what the last key does
        let mut x = inner.x;
        for (keys, label) in hints {
            for &(glyph, key) in keys {
                let w = u16::try_from(glyph.chars().count()).unwrap_or_default();
                let rect = Rect::new(x, inner.y, w, 1).intersection(inner);
                frame.render_widget(Span::styled(glyph, self.theme.accent), rect);
                self.last.hits.push((rect, Hit::Key(key)));
                x = x.saturating_add(w);
            }
            let w = u16::try_from(label_width(label)).unwrap_or_default();
            let rect = Rect::new(x, inner.y, w, 1).intersection(inner);
            frame.render_widget(Span::styled(format!(" {label}"), self.theme.muted), rect);
            if let Some(&(_, key)) = keys.last() {
                self.last.hits.push((rect, Hit::Key(key)));
            }
            x = x.saturating_add(w + 1);
        }
    }

    /// Makes the visible part of every link clickable, and highlights the one picked in visual mode.
    fn draw_links(&mut self, frame: &mut ratatui::Frame, area: Rect, offset: usize) {
        let visible = offset..offset + usize::from(area.height);
        let cells = self.last.links.iter().enumerate().flat_map(|(idx, link)| {
            link.cells
                .iter()
                .filter(|(row, _)| visible.contains(row))
                .map(move |(row, cols)| {
                    let y = u16::try_from(row - offset).unwrap_or_default();
                    let rect = Rect::new(area.x + cols.start, area.y + y, cols.end - cols.start, 1);
                    (rect.intersection(area), idx)
                })
        });
        let hits: Vec<_> = cells.collect();
        let selected = self.theme.selected.reversed();
        hits.iter()
            .filter(|&&(_, idx)| self.visual == Some(idx))
            .for_each(|&(rect, _)| frame.buffer_mut().set_style(rect, selected));
        self.last
            .hits
            .extend(hits.into_iter().map(|(rect, idx)| (rect, Hit::Link(idx))));
    }

    fn draw_text(&mut self, frame: &mut ratatui::Frame, area: Rect, view: View) {
        self.last.links = view.links;
        let height = u16::try_from(view.lines.len()).unwrap_or(u16::MAX);
        self.last.max_scroll = height.saturating_sub(area.height);
        if let Some(link) = self.visual.and_then(|idx| self.last.links.get(idx)) {
            let row = u16::try_from(link.row()).unwrap_or(u16::MAX);
            let top = (row + 1).saturating_sub(area.height.max(1));
            self.scroll = self.scroll.clamp(top, row);
        }
        self.scroll = self.scroll.min(self.last.max_scroll);
        frame.render_widget(Paragraph::new(view.lines).scroll((self.scroll, 0)), area);
        self.draw_links(frame, area, usize::from(self.scroll));
    }

    fn draw_list(&mut self, frame: &mut ratatui::Frame, area: Rect, view: View) {
        self.last.links = view.links;

        let height = area.height as usize;
        let offset = view
            .entries
            .get(self.selected())
            .map_or(0, |range| range.start.min(range.end.saturating_sub(height)));
        let end = offset + height;

        let hits: Vec<_> = view
            .entries
            .iter()
            .enumerate()
            .map(|(idx, range)| (range.start.max(offset)..range.end.min(end), idx))
            .filter(|(rows, _)| !rows.is_empty())
            .map(|(rows, idx)| {
                let y = u16::try_from(rows.start - offset).unwrap_or_default();
                let h = u16::try_from(rows.len()).unwrap_or_default();
                (Rect::new(area.x, area.y + y, area.width, h), Hit::Item(idx))
            })
            .collect();
        self.last.hits.extend(hits);

        let lines: Vec<Line> = view.lines.into_iter().skip(offset).take(height).collect();
        frame.render_widget(Paragraph::new(lines), area);
        self.draw_links(frame, area, offset);
    }

    fn section(&self) -> &'static str {
        self.page.tab().title()
    }

    fn view(&self, width: u16, mode: Mode) -> View {
        match self.page {
            Page::Post(idx) => self.post_view(idx, width, mode),
            Page::Tab(Tab::About) => self.about_view(width),
            Page::Tab(_) | Page::NotFound => {
                self.list_view(usize::from(width).saturating_sub(2), mode)
            }
        }
    }

    fn about_view(&self, width: u16) -> View {
        let bird = Art::fitting(&BIRD, width).map(|art| art.lines(width));
        let intro = [
            Line::styled(content::HEADLINE.to_uppercase(), self.theme.heading),
            Line::styled(content::SUBHEADLINE, Style::new().italic()),
            Line::default(),
        ];
        let paragraphs = content::ABOUT
            .iter()
            .flat_map(|paragraph| [Line::raw(*paragraph), Line::default()]);
        let lines = intro
            .into_iter()
            .chain(paragraphs)
            .chain(bird.into_iter().flatten())
            .collect();
        let wrapped = wrap(lines, &[], width);
        View {
            lines: wrapped.lines,
            entries: Vec::new(),
            links: wrapped.links,
        }
    }

    fn post_view(&self, idx: usize, width: u16, mode: Mode) -> View {
        let post = &self.posts[idx];
        let back = match mode {
            Mode::Tui => "← esc back to blog".to_string(),
            Mode::Print(base) => format!("← {base}/blog"),
        };
        let mut lines = vec![
            Line::styled(post.title, self.theme.heading),
            Line::styled(post.date, self.theme.muted.italic()),
            Line::styled("─".repeat(width.into()), self.theme.muted),
            Line::default(),
        ];
        // urls are only written out where links can't be clicked
        let expand = match (self.platform, mode) {
            (Platform::Web, Mode::Tui) => false,
            (Platform::Ssh, _) | (_, Mode::Print(_)) => true,
        };
        let markdown = render_markdown(post.body, self.theme, expand);
        let anchors: Vec<_> = markdown
            .anchors
            .into_iter()
            .map(|anchor| Anchor {
                line: anchor.line + lines.len(),
                ..anchor
            })
            .collect();
        lines.extend(markdown.lines);
        lines.extend([Line::default(), Line::styled(back, self.theme.muted)]);
        let wrapped = wrap(lines, &anchors, width);
        View {
            lines: wrapped.lines,
            entries: Vec::new(),
            links: wrapped.links,
        }
    }

    /// Entries are wrapped here, not by the `Paragraph`, so their row ranges are exact.
    fn list_view(&self, width: usize, mode: Mode) -> View {
        let selected = match mode {
            Mode::Tui => Some(self.selected()),
            Mode::Print(_) => None,
        };
        let mut lines = self.list_header(width);
        let mut entries = Vec::new();
        let mut targets = Vec::new();
        for (idx, entry) in self
            .list_entries(width, selected, mode)
            .into_iter()
            .enumerate()
        {
            let gutter = selected
                .filter(|&sel| sel == idx)
                .map_or(Span::raw("  "), |_| Span::styled("▌ ", self.theme.selected));
            let start = lines.len();
            let anchors = url_anchors(&entry);
            let wrapped = wrap(entry, &anchors, u16::try_from(width).unwrap_or(u16::MAX));
            let gutter_width = u16::try_from(gutter.width()).unwrap_or_default();
            targets.extend(
                wrapped
                    .links
                    .into_iter()
                    .map(|link| link.shifted(start, gutter_width, Some(idx))),
            );
            lines.extend(
                wrapped
                    .lines
                    .into_iter()
                    .map(|line| Line::from_iter(std::iter::once(gutter.clone()).chain(line.spans))),
            );
            entries.push(start..lines.len());
            lines.push(Line::default());
        }
        View {
            lines,
            entries,
            links: targets,
        }
    }

    fn list_header(&self, width: usize) -> Vec<Line<'static>> {
        match self.page {
            Page::NotFound => vec![
                Line::styled("404 · not found", self.theme.heading),
                Line::raw("That page doesn't exist."),
                Line::default(),
            ],
            Page::Tab(Tab::Blog) if self.posts.is_empty() => vec![Line::raw("No posts yet.")],
            Page::Tab(Tab::Contact) => {
                let mut lines = wrapped(content::CONTACT_TEXT, width + 2, Style::new().italic());
                lines.push(Line::default());
                lines
            }
            _ => Vec::new(),
        }
    }

    fn list_entries(
        &self,
        width: usize,
        selected: Option<usize>,
        mode: Mode,
    ) -> Vec<Vec<Line<'static>>> {
        let title = |idx: usize, text: &'static str| {
            let style = selected
                .filter(|&sel| sel == idx)
                .map_or(Style::new().bold(), |_| self.theme.selected);
            Line::styled(text, style)
        };
        let plain = Style::new();

        match self.page.tab() {
            Tab::Projects => PROJECTS
                .iter()
                .enumerate()
                .map(|(idx, project)| {
                    let stat = self.stats[idx].map(|n| format!(" · {} {n}", project.host.symbol));
                    let meta = Line::from(vec![
                        Span::styled(project.kind, self.theme.muted.italic()),
                        Span::styled(stat.unwrap_or_default(), self.theme.muted),
                    ]);
                    let links = std::iter::once(project.url())
                        .chain(project.site.map(String::from))
                        .map(|url| Line::styled(url, self.theme.link));
                    [title(idx, project.name), meta]
                        .into_iter()
                        .chain(wrapped(project.description, width, plain))
                        .chain(links)
                        .collect()
                })
                .collect(),
            Tab::Blog => self
                .posts
                .iter()
                .enumerate()
                .map(|(idx, post)| {
                    let read = match mode {
                        Mode::Tui => Line::styled("read →", self.theme.accent),
                        Mode::Print(base) => {
                            Line::styled(format!("{base}/blog/{}", post.slug), self.theme.link)
                        }
                    };
                    [
                        title(idx, post.title),
                        Line::styled(post.date, self.theme.muted.italic()),
                    ]
                    .into_iter()
                    .chain(wrapped(&post.description, width, plain))
                    .chain([read])
                    .collect()
                })
                .collect(),
            Tab::Bookmarks => BOOKMARKS
                .iter()
                .enumerate()
                .map(|(idx, bookmark)| {
                    let tags: Vec<_> = bookmark.tags.iter().map(|t| format!("#{t}")).collect();
                    std::iter::once(title(idx, bookmark.title))
                        .chain(wrapped(&tags.join(" "), width, self.theme.tag))
                        .chain(wrapped(bookmark.description, width, plain))
                        .chain([Line::styled(bookmark.url, self.theme.link)])
                        .collect()
                })
                .collect(),
            Tab::Contact => CONTACTS
                .iter()
                .enumerate()
                .map(|(idx, contact)| {
                    vec![
                        title(idx, contact.label),
                        Line::styled(contact.value, self.theme.link),
                    ]
                })
                .collect(),
            Tab::About => Vec::new(),
        }
    }

    pub fn print(&mut self, route: &str, base: &str, width: u16) -> Buffer {
        let route = route.trim_matches('/');
        let mut lines = vec![
            Line::from(vec![
                Span::styled(content::NAME, self.theme.heading),
                Span::styled(" · ", self.theme.muted),
                Span::styled(content::TAGLINE, Style::new().italic()),
            ]),
            Line::default(),
        ];
        let (section, body, long_page) = match route {
            "" => ("Pages", self.index(base), "projects"),
            route => {
                self.goto(route);
                (
                    self.section(),
                    self.view(width, Mode::Print(base)).lines,
                    route,
                )
            }
        };
        let rule = "─".repeat(usize::from(width).saturating_sub(section.len() + 4));
        lines.extend([
            Line::styled(format!("── {section} {rule}"), self.theme.heading),
            Line::default(),
        ]);
        lines.extend(body);
        lines.extend([
            Line::from(vec![
                Span::styled("tip ", self.theme.muted),
                Span::styled(
                    format!("curl -s {base}/{long_page} | less -R"),
                    self.theme.code,
                ),
            ]),
            Line::styled(
                format!(
                    "© {} {} · open {base} in a browser",
                    self.year,
                    content::NAME
                ),
                self.theme.muted,
            ),
        ]);

        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
        let height = u16::try_from(paragraph.line_count(width)).unwrap_or(u16::MAX);
        let area = Rect::new(0, 0, width, height);
        let mut buf = Buffer::empty(area);
        paragraph.render(area, &mut buf);
        buf
    }

    fn index(&self, base: &str) -> Vec<Line<'static>> {
        let pages = Tab::ALL.into_iter().map(|tab| {
            Line::from(vec![
                Span::styled(format!("  {:<11}", tab.slug()), self.theme.accent),
                Span::styled(format!("{base}/{}", tab.slug()), self.theme.link),
            ])
        });
        let intro = [
            Line::styled(content::SUBHEADLINE, Style::new().italic()),
            Line::default(),
        ];
        intro
            .into_iter()
            .chain(pages)
            .chain([Line::default()])
            .collect()
    }
}

/// Numbered labels on one row when they fit, plain titles wrapped over rows otherwise.
fn tab_layout(width: u16) -> Vec<Vec<(Tab, String)>> {
    let width = usize::from(width);
    let numbered: Vec<_> = (1..)
        .zip(Tab::ALL)
        .map(|(n, tab)| (tab, format!("[{n}] {}", tab.title())))
        .collect();
    let numbered_width = numbered
        .iter()
        .map(|(_, label)| label.len() + 1)
        .sum::<usize>();
    if numbered_width <= width + 1 {
        return vec![numbered];
    }

    let mut rows = vec![Vec::new()];
    let mut used = 0;
    for tab in Tab::ALL {
        let len = tab.title().len();
        if used > 0 && used + len > width {
            rows.push(Vec::new());
            used = 0;
        }
        if let Some(row) = rows.last_mut() {
            row.push((tab, tab.title().to_string()));
        }
        used += len + 1;
    }
    rows
}

fn wrapped(text: &str, width: usize, style: Style) -> Vec<Line<'static>> {
    let width = width.max(10);
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let len = line.chars().count();
        if len > 0 && len + 1 + word.chars().count() > width {
            lines.push(Line::styled(std::mem::take(&mut line), style));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(Line::styled(line, style));
    }
    lines
}
