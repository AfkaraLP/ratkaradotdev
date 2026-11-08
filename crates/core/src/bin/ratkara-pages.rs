//! usage: ratkara-pages <dist> [site-url]

use std::{env, fs, path::Path};

use anyhow::{Context, bail};
use ratkara_core::{App, Platform};

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

    let template = fs::read_to_string(dist.join("index.html")).context("reading index.html")?;
    let (Some(start), Some(end)) = (template.find(START), template.find(END)) else {
        bail!("index.html is missing the {START} ... {END} block");
    };
    let page = |app: &App, route: &str| {
        let head = meta(app, &format!("{site}/{route}"), &format!("{site}/og.png"));
        format!(
            "{}{head}{}",
            &template[..start],
            &template[end + END.len()..]
        )
    };

    let mut app = App::new(Platform::Web, 0);
    fs::write(dist.join("index.html"), page(&app, ""))?;

    let routes = app.routes();
    for route in &routes {
        app.goto(route);
        let dir = dist.join(route);
        fs::create_dir_all(&dir)?;
        fs::write(dir.join("index.html"), page(&app, route))?;
    }

    // most static hosts serve this for unknown paths, the app shows its own 404
    app.goto("404");
    fs::write(dist.join("404.html"), page(&app, "404"))?;

    eprintln!("wrote {} pages to {}", routes.len() + 2, dist.display());
    Ok(())
}

fn meta(app: &App, url: &str, image: &str) -> String {
    let title = escape(&app.title());
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
    <meta property="og:title" content="{title}" />
    <meta property="og:description" content="{description}" />
    <meta property="og:url" content="{url}" />
    <meta property="og:image" content="{image}" />
    <meta property="og:image:width" content="1200" />
    <meta property="og:image:height" content="630" />
    <meta name="twitter:card" content="summary_large_image" />
    <meta name="twitter:image" content="{image}" />
    <meta name="twitter:title" content="{title}" />
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
