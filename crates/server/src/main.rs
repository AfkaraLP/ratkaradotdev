mod args;
mod http;
mod input;
mod ssh;
mod stats;

use std::time::{SystemTime, UNIX_EPOCH};

use clap::Parser;
use ratkara_core::content::year_from_unix;

use crate::args::Args;

fn current_year() -> u32 {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    year_from_unix(i64::try_from(secs).unwrap_or_default())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let stats = stats::spawn_refresh();
    tokio::try_join!(
        ssh::serve(&args.host, args.ssh_port, &args.host_key, stats.clone()),
        http::serve(
            args.http_host.as_deref().unwrap_or(&args.host),
            args.http_port,
            args.web_dir.clone(),
            args.site_url.clone(),
            stats,
        ),
    )?;
    Ok(())
}
