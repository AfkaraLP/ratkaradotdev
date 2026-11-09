use std::path::PathBuf;

use axum::{
    Router,
    extract::{Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Redirect, Response},
};
use ratkara_core::{App, Platform, ansi::to_ansi};
use tower_http::services::{ServeDir, ServeFile};

use crate::{current_year, stats::Stats};

const CLI_AGENTS: &[&str] = &["curl/", "Wget/", "HTTPie/", "xh/", "nushell"];
const WIDTH: u16 = 80;

#[derive(Clone)]
struct Site {
    stats: Stats,
    url: Option<String>,
}

pub async fn serve(
    host: &str,
    port: u16,
    web_dir: Option<PathBuf>,
    site_url: Option<String>,
    stats: Stats,
) -> anyhow::Result<()> {
    let files = web_dir.map_or_else(Router::new, |dir| {
        let not_found = ServeFile::new(dir.join("404.html"));
        Router::new().fallback_service(
            ServeDir::new(dir)
                .redirect_to_trailing_slash(false)
                .not_found_service(not_found),
        )
    });
    let site = Site {
        stats,
        url: site_url.map(|url| url.trim_end_matches('/').to_string()),
    };
    let router = files.layer(middleware::from_fn_with_state(site, terminal));

    let listener = tokio::net::TcpListener::bind((host, port)).await?;
    eprintln!("http on {host}:{port}");
    axum::serve(listener, router).await?;
    Ok(())
}

async fn terminal(State(site): State<Site>, req: Request, next: Next) -> Response {
    let agent = header_str(req.headers(), header::USER_AGENT.as_str());
    // files stay files, cloudflare caches them by extension for every user agent
    let page = std::path::Path::new(req.uri().path()).extension().is_none();
    let cli = CLI_AGENTS.iter().any(|cli| agent.starts_with(cli));
    let text = (page && cli).then(|| site.print(&req));
    let insecure = header_str(req.headers(), "cf-visitor").contains(r#""scheme":"http""#);
    let mut response = match (text, insecure, &site.url) {
        (Some(text), ..) => text,
        (None, true, Some(url)) => {
            let path = req.uri().path_and_query().map_or("/", |p| p.as_str());
            Redirect::permanent(&format!("{url}{path}")).into_response()
        }
        (None, ..) => next.run(req).await,
    };
    response
        .headers_mut()
        .insert(header::VARY, HeaderValue::from_static("user-agent"));
    response
}

impl Site {
    fn print(&self, req: &Request) -> Response {
        let headers = req.headers();
        let base = self.url.clone().unwrap_or_else(|| {
            let scheme = Some(header_str(headers, "x-forwarded-proto"))
                .filter(|proto| !proto.is_empty())
                .unwrap_or("http");
            format!("{scheme}://{}", header_str(headers, header::HOST.as_str()))
        });

        let mut app = App::new(Platform::Ssh, current_year());
        if let Ok(stats) = self.stats.read() {
            app.set_stats(&stats);
        }
        let text = to_ansi(&app.print(req.uri().path(), &base, WIDTH));
        let code = [StatusCode::OK, StatusCode::NOT_FOUND][usize::from(app.is_not_found())];
        let content_type = [(header::CONTENT_TYPE, "text/plain; charset=utf-8")];
        (code, content_type, text).into_response()
    }
}

fn header_str<'a>(headers: &'a HeaderMap, name: &str) -> &'a str {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
}
