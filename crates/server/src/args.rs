use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {
    #[arg(short = 'H', long, default_value = "0.0.0.0", env = "RATKARA_HOST")]
    pub host: String,

    /// Defaults to --host, use 127.0.0.1 behind a reverse proxy
    #[arg(long, env = "RATKARA_HTTP_HOST")]
    pub http_host: Option<String>,

    #[arg(long, default_value_t = 2222, env = "RATKARA_SSH_PORT")]
    pub ssh_port: u16,

    #[arg(long, default_value_t = 8080, env = "RATKARA_HTTP_PORT")]
    pub http_port: u16,

    /// Ed25519 host key, generated on first start if missing
    #[arg(
        short = 'k',
        long,
        default_value = "host_key",
        env = "RATKARA_HOST_KEY"
    )]
    pub host_key: PathBuf,

    /// Built web frontend (`nix build .#web`), browsers get a 404 without it
    #[arg(short, long, env = "RATKARA_WEB_DIR")]
    pub web_dir: Option<PathBuf>,

    /// Base for links in text responses, taken from the Host header if unset
    #[arg(long, env = "RATKARA_SITE_URL")]
    pub site_url: Option<String>,

    /// Ask the running http server for / and exit, for container health checks
    #[arg(long)]
    pub health_check: bool,

    /// Only used for GitHub star counts, raises the rate limit on shared (CGNAT) IPs
    #[arg(long, env = "RATKARA_GITHUB_TOKEN", hide_env_values = true)]
    pub github_token: Option<String>,
}
