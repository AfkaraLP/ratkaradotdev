//! usage: `RATKARA_FONT_DIR=<dir with FiraMono-{Regular,Bold}.ttf> ratkara-pages <dist> [site-url]`

mod card;

use std::{
    env, fs,
    hash::{DefaultHasher, Hash, Hasher},
    path::Path,
};

use anyhow::{Context, bail};
use ratkara_core::{App, Platform, content};

use crate::card::{Card, Fonts};

const START: &str = "<!-- meta -->";
const END: &str = "<!-- /meta -->";

fn main() -> anyhow::Result<()> {
    let mut args = env::args().skip(1);
    let Some(dist) = args.next() else {
        bail!("usage: ratkara-pages <dist> [site-url]");
    };
    let site = args.next().unwrap_or_else(|| "https://afkara.dev".into());
    let site = site.trim_end_matches('/');
    let dist = Path::new(&dist);
    let fonts = Fonts::load(Path::new(
        &env::var("RATKARA_FONT_DIR").context("RATKARA_FONT_DIR is not set")?,
    ))?;

    let template = fs::read_to_string(dist.join("index.html")).context("reading index.html")?;
    let (Some(start), Some(end)) = (template.find(START), template.find(END)) else {
        bail!("index.html is missing the {START} ... {END} block");
    };
    let page = |app: &App, route: &str, image: &str| {
        let head = meta(app, &format!("{site}/{route}"), image);
        format!(
            "{}{head}{}",
            &template[..start],
            &template[end + END.len()..]
        )
    };

    let site_card = write_card(dist, site, "og.png", &Card::Site, &fonts)?;
    let mut app = App::new(Platform::Web, 0);
    fs::write(dist.join("index.html"), page(&app, "", &site_card))?;

    let routes = app.routes();
    for route in &routes {
        app.goto(route);
        let image = match (app.date(), route.strip_prefix("blog/")) {
            (Some(date), Some(slug)) => {
                let card = Card::Post {
                    title: &heading(&app),
                    date,
                    excerpt: &app.description(),
                };
                write_card(dist, site, &format!("og/{slug}.png"), &card, &fonts)?
            }
            _ => site_card.clone(),
        };
        let dir = dist.join(route);
        fs::create_dir_all(&dir)?;
        fs::write(dir.join("index.html"), page(&app, route, &image))?;
    }

    // most static hosts serve this for unknown paths, the app shows its own 404
    app.goto("404");
    fs::write(dist.join("404.html"), page(&app, "404", &site_card))?;

    eprintln!("wrote {} pages to {}", routes.len() + 2, dist.display());
    Ok(())
}

/// Renders a card into `dist` and returns its url, versioned by content: previews and
/// cloudflare cache images by url, so a changed card needs a new one.
fn write_card(
    dist: &Path,
    site: &str,
    path: &str,
    card: &Card,
    fonts: &Fonts,
) -> anyhow::Result<String> {
    let png = card.png(fonts)?;
    let file = dist.join(path);
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(&file, &png)?;
    let mut hasher = DefaultHasher::new();
    png.hash(&mut hasher);
    Ok(format!("{site}/{path}?v={:x}", hasher.finish()))
}

/// The page title without the site name, previews show that on their own.
fn heading(app: &App) -> String {
    app.title()
        .trim_end_matches(&format!(" · {}", content::NAME))
        .to_string()
}

fn meta(app: &App, url: &str, image: &str) -> String {
    let title = escape(&app.title());
    let heading = escape(&heading(app));
    let description = escape(&app.description());
    let (kind, published) = match app.date() {
        Some(date) => (
            "article",
            format!(
                "\n    <meta property=\"article:published_time\" content=\"{}\" />",
                escape(date)
            ),
        ),
        None => ("website", String::new()),
    };

    format!(
        r#"<title>{title}</title>
    <meta name="description" content="{description}" />
    <meta property="og:site_name" content="AfkaraLP" />
    <meta property="og:type" content="{kind}" />
    <meta property="og:title" content="{heading}" />
    <meta property="og:description" content="{description}" />
    <meta property="og:url" content="{url}" />
    <meta property="og:image" content="{image}" />
    <meta property="og:image:width" content="1200" />
    <meta property="og:image:height" content="630" />
    <meta name="twitter:card" content="summary_large_image" />
    <meta name="twitter:image" content="{image}" />
    <meta name="twitter:title" content="{heading}" />
    <meta name="twitter:description" content="{description}" />
    <link rel="canonical" href="{url}" />{published}"#
    )
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
