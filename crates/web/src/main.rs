use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use ratkara_core::{
    Action, App, Key, Platform,
    content::{PROJECTS, year_from_unix},
    ratatui::{Terminal, backend::Backend},
    theme::ferra,
};
use ratzilla::{
    CanvasBackend, CellSized, FontAtlasConfig, WebEventHandler, WebGl2Backend, WebRenderer,
    backend::webgl2::WebGl2BackendOptions,
    event::{KeyCode, KeyEvent, MouseButton, MouseEventKind},
    utils::open_url,
};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{
    Element, HtmlElement, Response, TouchEvent, WheelEvent, Window,
    js_sys::Date,
    wasm_bindgen::{JsCast, JsValue, closure::Closure},
};

const FONT: &str = "Fira Code";

type Shared = Rc<RefCell<App>>;

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
    let app: Shared = Rc::new(RefCell::new(App::new(Platform::Web, year)));
    let location = window.location();
    let route = Some(location.hash()?)
        .filter(|hash| hash.len() > 1)
        .map_or_else(|| location.pathname(), Ok)?;
    app.borrow_mut().goto(&route);
    perform(&window, &app.borrow(), Action::None);
    fetch_stats(&window, &app);

    // css sizes the canvas (100dvh follows mobile toolbars), the backend follows it
    let options = WebGl2BackendOptions::new()
        .font_atlas_config(FontAtlasConfig::dynamic(&[FONT, "monospace"], font_size))
        .canvas_padding_color(ferra::NIGHT)
        .disable_auto_css_resize()
        .enable_hyperlinks();
    match WebGl2Backend::new_with_options(options) {
        Ok(backend) => launch(terminal(backend)?, &app, &window),
        Err(err) => {
            let msg = format!("webgl2 unavailable ({err}), falling back to a 2d canvas");
            web_sys::console::warn_1(&msg.into());
            let dead = document.query_selector_all("canvas")?;
            (0..dead.length())
                .filter_map(|idx| dead.item(idx)?.dyn_into::<Element>().ok())
                .for_each(|canvas| canvas.remove());
            let backend = CanvasBackend::new().map_err(|e| e.to_string())?;
            launch(terminal(backend)?, &app, &window)
        }
    }
}

fn terminal<B: Backend>(backend: B) -> Result<Terminal<B>, JsValue> {
    Terminal::new(backend).map_err(|e| e.to_string().into())
}

fn launch<B>(mut terminal: Terminal<B>, app: &Shared, window: &Window) -> Result<(), JsValue>
where
    B: Backend + WebEventHandler + CellSized + 'static,
{
    terminal
        .on_key_event({
            let (app, window) = (app.clone(), window.clone());
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
            let (app, window) = (app.clone(), window.clone());
            move |event| {
                if event.kind == MouseEventKind::SingleClick(MouseButton::Left) {
                    let action = app.borrow_mut().click(event.col, event.row);
                    perform(&window, &app.borrow(), action);
                }
            }
        })
        .map_err(|e| e.to_string())?;

    let document = window.document().ok_or("no document")?;
    let canvas: HtmlElement = document
        .query_selector("canvas")?
        .ok_or("no canvas")?
        .dyn_into()?;
    let (_, row) = terminal.backend().cell_size_css_px();
    scroll_gestures(&canvas, app, f64::from(row))?;
    _ = canvas.focus();

    let app = app.clone();
    terminal.draw_web(move |frame| app.borrow_mut().draw(frame));
    Ok(())
}

/// Swipes and the mouse wheel move by whole rows, like the arrow keys.
fn scroll_gestures(canvas: &HtmlElement, app: &Shared, row: f64) -> Result<(), JsValue> {
    let scroll = Rc::new({
        let app = app.clone();
        move |rows: f64| {
            let key = [Key::Up, Key::Down][usize::from(rows > 0.0)];
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            (0..rows.abs() as usize).for_each(|_| _ = app.borrow_mut().handle_key(key));
        }
    });

    let touch = Rc::new(Cell::new(None::<f64>));
    let start = Closure::<dyn FnMut(TouchEvent)>::new({
        let touch = touch.clone();
        move |event: TouchEvent| touch.set(event.touches().get(0).map(|t| f64::from(t.client_y())))
    });
    let moved = Closure::<dyn FnMut(TouchEvent)>::new({
        let scroll = scroll.clone();
        move |event: TouchEvent| {
            let (Some(from), Some(point)) = (touch.get(), event.touches().get(0)) else {
                return;
            };
            let rows = ((from - f64::from(point.client_y())) / row).trunc();
            scroll(rows);
            touch.set(Some(from - rows * row));
        }
    });

    // pixel, line and page deltas, firefox reports lines
    let unit = [1.0, row, row * 20.0];
    let pending = Cell::new(0.0);
    let wheel = Closure::<dyn FnMut(WheelEvent)>::new(move |event: WheelEvent| {
        event.prevent_default();
        let scale = unit
            .get(event.delta_mode() as usize)
            .copied()
            .unwrap_or(row);
        let total = pending.get() + event.delta_y() * scale;
        let rows = (total / row).trunc();
        scroll(rows);
        pending.set(total - rows * row);
    });

    canvas.add_event_listener_with_callback("touchstart", start.as_ref().unchecked_ref())?;
    canvas.add_event_listener_with_callback("touchmove", moved.as_ref().unchecked_ref())?;
    canvas.add_event_listener_with_callback("wheel", wheel.as_ref().unchecked_ref())?;
    start.forget();
    moved.forget();
    wheel.forget();
    Ok(())
}

fn fetch_stats(window: &Window, app: &Shared) {
    for (idx, project) in PROJECTS.iter().enumerate() {
        let (app, window) = (app.clone(), window.clone());
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
