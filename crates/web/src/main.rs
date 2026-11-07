use std::{cell::RefCell, rc::Rc};

use ratkara_core::{
    Action, App, Key, Platform,
    content::{PROJECTS, year_from_unix},
    ratatui::Terminal,
    theme::ferra,
};
use ratzilla::{
    FontAtlasConfig, WebGl2Backend, WebRenderer,
    backend::webgl2::WebGl2BackendOptions,
    event::{KeyCode, KeyEvent, MouseButton, MouseEventKind},
    utils::open_url,
};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{
    HtmlElement, Response, Window,
    js_sys::Date,
    wasm_bindgen::{JsCast, JsValue},
};

const FONT: &str = "Fira Code";

fn main() {
    console_error_panic_hook::set_once();
    spawn_local(async {
        if let Err(err) = run().await {
            web_sys::console::error_1(&err);
        }
    });
}

async fn run() -> Result<(), JsValue> {
    let window = web_sys::window().ok_or("no window")?;
    let document = window.document().ok_or("no document")?;

    let narrow = window.inner_width()?.as_f64().unwrap_or_default() < 640.0;
    let font_size = [16.0, 13.0][usize::from(narrow)];
    // the glyph atlas is rasterized once, so the webfont has to be there first
    let load = document.fonts().load(&format!("{font_size}px \"{FONT}\""));
    _ = JsFuture::from(load).await;

    #[allow(clippy::cast_possible_truncation)]
    let year = year_from_unix((Date::now() / 1000.0) as i64);
    let app = Rc::new(RefCell::new(App::new(Platform::Web, year)));
    let location = window.location();
    let route = Some(location.hash()?)
        .filter(|hash| hash.len() > 1)
        .map_or_else(|| location.pathname(), Ok)?;
    app.borrow_mut().goto(&route);
    perform(&window, &app.borrow(), Action::None);

    let options = WebGl2BackendOptions::new()
        .font_atlas_config(FontAtlasConfig::dynamic(&[FONT, "monospace"], font_size))
        .canvas_padding_color(ferra::NIGHT)
        .enable_hyperlinks();
    let backend = WebGl2Backend::new_with_options(options).map_err(|e| e.to_string())?;
    let mut terminal = Terminal::new(backend).map_err(|e| e.to_string())?;

    terminal
        .on_key_event({
            let app = app.clone();
            let window = window.clone();
            move |event| {
                let Some(key) = to_key(&event) else {
                    return;
                };
                let action = app.borrow_mut().handle_key(key);
                perform(&window, &app.borrow(), action);
            }
        })
        .map_err(|e| e.to_string())?;

    terminal
        .on_mouse_event({
            let app = app.clone();
            let window = window.clone();
            move |event| {
                if event.kind == MouseEventKind::SingleClick(MouseButton::Left) {
                    let action = app.borrow_mut().click(event.col, event.row);
                    perform(&window, &app.borrow(), action);
                }
            }
        })
        .map_err(|e| e.to_string())?;

    for (idx, project) in PROJECTS.iter().enumerate() {
        let app = app.clone();
        let window = window.clone();
        let url = project.api_url();
        spawn_local(async move {
            if let Some(stat) = fetch_text(&window, &url)
                .await
                .as_deref()
                .and_then(|json| project.parse_stat(json))
            {
                app.borrow_mut().set_stat(idx, stat);
            }
        });
    }

    if let Some(canvas) = document.query_selector("canvas")? {
        _ = canvas.dyn_into::<HtmlElement>()?.focus();
    }

    terminal.draw_web(move |frame| app.borrow_mut().draw(frame));
    Ok(())
}

fn perform(window: &Window, app: &App, action: Action) {
    if let Action::Open(url) = action {
        _ = open_url(&url, !url.starts_with("mailto:"));
    }
    let path = match app.route().as_str() {
        "about" => "/".to_string(),
        route => format!("/{route}"),
    };
    if window.location().pathname().ok().as_deref() != Some(path.as_str())
        && let Ok(history) = window.history()
    {
        _ = history.replace_state_with_url(&JsValue::NULL, "", Some(&path));
    }
    if let Some(document) = window.document() {
        document.set_title(&app.title());
    }
}

async fn fetch_text(window: &Window, url: &str) -> Option<String> {
    let response: Response = JsFuture::from(window.fetch_with_str(url))
        .await
        .ok()?
        .dyn_into()
        .ok()?;
    if !response.ok() {
        return None;
    }
    JsFuture::from(response.text().ok()?)
        .await
        .ok()?
        .as_string()
}

fn to_key(event: &KeyEvent) -> Option<Key> {
    Some(match event.code {
        KeyCode::Char('c') if event.ctrl => Key::CtrlC,
        KeyCode::Char(c) => Key::Char(c),
        KeyCode::Tab if event.shift => Key::BackTab,
        KeyCode::Tab => Key::Tab,
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Left => Key::Left,
        KeyCode::Right => Key::Right,
        KeyCode::Enter => Key::Enter,
        KeyCode::Esc => Key::Esc,
        KeyCode::Backspace => Key::Backspace,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::PageDown => Key::PageDown,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        _ => return None,
    })
}
