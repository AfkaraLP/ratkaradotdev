use std::{
    collections::HashMap,
    io::Write,
    net::IpAddr,
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::Context;
use ratatui::{
    Terminal, TerminalOptions, Viewport,
    backend::CrosstermBackend,
    crossterm::{
        cursor::{Hide, Show},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen},
    },
    layout::Rect,
};
use ratkara_core::{Action, App, Platform};
use russh::{
    Channel, ChannelId, MethodKind, MethodSet, Pty,
    keys::{Algorithm, PrivateKey, ssh_key::LineEnding},
    server::{Auth, ChannelOpenHandle, Config, Handler, Msg, Session, VerifiedPublicKey},
};

use tokio::net::TcpListener;

use crate::{current_year, input::parse_keys, stats::Stats};

#[derive(Default)]
struct Output(Vec<u8>);

impl Write for Output {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

type SshTerminal = Terminal<CrosstermBackend<Output>>;

const MAX_PER_IP: usize = 4;
const MAX_TOTAL: usize = 256;
/// Clients pick their own pty size, an unbounded one would allocate a huge buffer.
const MAX_SIZE: (u32, u32) = (500, 200);

#[derive(Clone, Default)]
struct Peers(Arc<Mutex<HashMap<IpAddr, usize>>>);

struct Admitted {
    peers: Peers,
    ip: IpAddr,
}

impl Peers {
    fn admit(&self, ip: IpAddr) -> Option<Admitted> {
        let mut peers = self.0.lock().ok()?;
        let total: usize = peers.values().sum();
        let count = peers.get(&ip).copied().unwrap_or_default();
        (count < MAX_PER_IP && total < MAX_TOTAL).then(|| {
            peers.insert(ip, count + 1);
            Admitted {
                peers: self.clone(),
                ip,
            }
        })
    }
}

impl Drop for Admitted {
    fn drop(&mut self) {
        let Ok(mut peers) = self.peers.0.lock() else {
            return;
        };
        match peers.get(&self.ip).copied() {
            Some(count @ 2..) => _ = peers.insert(self.ip, count - 1),
            _ => _ = peers.remove(&self.ip),
        }
    }
}

struct Client {
    app: App,
    terminal: SshTerminal,
    stats: Stats,
}

impl Client {
    fn new(stats: Stats) -> Self {
        let options = TerminalOptions {
            // the real size arrives with the pty request
            viewport: Viewport::Fixed(Rect::default()),
        };
        let backend = CrosstermBackend::new(Output::default());
        Self {
            app: App::new(Platform::Ssh, current_year()),
            terminal: Terminal::with_options(backend, options)
                .expect("a fixed viewport never queries the terminal"),
            stats,
        }
    }

    fn resize(&mut self, cols: u32, rows: u32) -> anyhow::Result<()> {
        let (max_cols, max_rows) = MAX_SIZE;
        let rect = Rect::new(
            0,
            0,
            u16::try_from(cols.min(max_cols)).unwrap_or_default(),
            u16::try_from(rows.min(max_rows)).unwrap_or_default(),
        );
        self.terminal.resize(rect)?;
        Ok(())
    }

    fn draw(&mut self, channel: ChannelId, session: &mut Session) -> anyhow::Result<()> {
        if let Ok(stats) = self.stats.read() {
            self.app.set_stats(&stats);
        }
        self.terminal.draw(|frame| self.app.draw(frame))?;
        self.send(channel, session)
    }

    fn quit(&mut self, channel: ChannelId, session: &mut Session) -> anyhow::Result<()> {
        execute!(self.terminal.backend_mut(), LeaveAlternateScreen, Show)?;
        self.send(channel, session)?;
        session.exit_status_request(channel, 0)?;
        session.eof(channel)?;
        session.close(channel)?;
        Ok(())
    }

    fn send(&mut self, channel: ChannelId, session: &mut Session) -> anyhow::Result<()> {
        let out = std::mem::take(&mut self.terminal.backend_mut().writer_mut().0);
        if !out.is_empty() {
            session.data(channel, out)?;
        }
        Ok(())
    }
}

// russh requires async fns here even when they never await
#[allow(clippy::unused_async_trait_impl)]
impl Handler for Client {
    type Error = anyhow::Error;

    async fn auth_none(&mut self, _: &str) -> Result<Auth, Self::Error> {
        Ok(Auth::Accept)
    }

    async fn auth_publickey(
        &mut self,
        _: &str,
        _: VerifiedPublicKey<'_>,
    ) -> Result<Auth, Self::Error> {
        Ok(Auth::Accept)
    }

    async fn channel_open_session(
        &mut self,
        _: Channel<Msg>,
        reply: ChannelOpenHandle,
        _: &mut Session,
    ) -> Result<(), Self::Error> {
        reply.accept().await;
        Ok(())
    }

    async fn pty_request(
        &mut self,
        channel: ChannelId,
        _: &str,
        cols: u32,
        rows: u32,
        _: u32,
        _: u32,
        _: &[(Pty, u32)],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.resize(cols, rows)?;
        session.channel_success(channel)?;
        Ok(())
    }

    async fn shell_request(
        &mut self,
        channel: ChannelId,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        session.channel_success(channel)?;
        execute!(self.terminal.backend_mut(), EnterAlternateScreen, Hide)?;
        self.draw(channel, session)
    }

    async fn window_change_request(
        &mut self,
        channel: ChannelId,
        cols: u32,
        rows: u32,
        _: u32,
        _: u32,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.resize(cols, rows)?;
        self.draw(channel, session)
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        for key in parse_keys(data) {
            match self.app.handle_key(key) {
                Action::Quit => return self.quit(channel, session),
                Action::Open(url) => {
                    session.data(channel, osc52(&url))?;
                    self.app.notify(format!("copied {url}"));
                }
                Action::None => {}
            }
        }
        self.draw(channel, session)
    }
}

fn load_or_create_key(path: &Path) -> anyhow::Result<PrivateKey> {
    if path.exists() {
        return russh::keys::load_secret_key(path, None)
            .with_context(|| format!("failed to load host key {}", path.display()));
    }
    let key = PrivateKey::random(&mut rand::rng(), Algorithm::Ed25519)?;
    key.write_openssh_file(path, LineEnding::LF)
        .with_context(|| format!("failed to write host key {}", path.display()))?;
    eprintln!("generated new host key at {}", path.display());
    Ok(key)
}

pub async fn serve(host: &str, port: u16, host_key: &Path, stats: Stats) -> anyhow::Result<()> {
    let config = Arc::new(Config {
        methods: MethodSet::from(&[MethodKind::None, MethodKind::PublicKey][..]),
        inactivity_timeout: Some(Duration::from_mins(10)),
        auth_rejection_time: Duration::from_secs(1),
        auth_rejection_time_initial: Some(Duration::from_secs(0)),
        keys: vec![load_or_create_key(host_key)?],
        ..Default::default()
    });
    let listener = TcpListener::bind((host, port)).await?;
    let peers = Peers::default();
    eprintln!("ssh on {host}:{port}");

    loop {
        let Ok((stream, addr)) = listener.accept().await else {
            tokio::time::sleep(Duration::from_millis(100)).await;
            continue;
        };
        let Some(admitted) = peers.admit(addr.ip()) else {
            continue;
        };
        _ = stream.set_nodelay(true);
        let (config, client) = (config.clone(), Client::new(stats.clone()));
        tokio::spawn(async move {
            let _admitted = admitted;
            if let Ok(session) = russh::server::run_stream(config, stream, client).await {
                _ = session.await;
            }
        });
    }
}

fn osc52(text: &str) -> Vec<u8> {
    const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let encoded: String = text
        .as_bytes()
        .chunks(3)
        .flat_map(|chunk| {
            let n = chunk
                .iter()
                .enumerate()
                .fold(0u32, |n, (i, &b)| n | u32::from(b) << (16 - 8 * i));
            (0..=chunk.len())
                .map(move |i| char::from(B64[(n >> (18 - 6 * i) & 63) as usize]))
                .chain(std::iter::repeat_n('=', 3 - chunk.len()))
        })
        .collect();
    format!("\x1b]52;c;{encoded}\x07").into_bytes()
}
