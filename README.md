# ratkaradotdev

My personal site, now as a TUI. Same app in the browser through [ratzilla], over ssh through [russh], and as plain text over `curl/http get`.

```sh
curl afkara.dev
```

## Layout

| crate           | what                                                                  |
| --------------- | --------------------------------------------------------------------- |
| `crates/core`   | content, markdown, app state + drawing, `ratkara-pages` for OG embeds |
| `crates/web`    | ratzilla WebGL2 frontend                                              |
| `crates/server` | ssh + http in one binary; http serves the web build, text for curl    |

## Embeds

The server serves every page with its own title and OG tags, posts use their opening text.

## Keys

Vim bindings, `v` to pick a link (opens it in the browser, copies it over ssh).

## Dev

```sh
direnv allow                                  # or: nix develop
cd crates/web && trunk serve                  # web with live reload on :8090
nix build .#web && cargo run -p ratkara-server -- --web-dir result
```

## Nix

```sh
nix build .#web      # static files with prerendered embeds
nix run .            # ratkara-server, serving .#web on :8080 and ssh on :2222
```

`wasm-bindgen` is pinned in `crates/web/Cargo.toml` and has to match `wasmBindgenCli` in `flake.nix`.

## Deploy

CI publishes `ghcr.io/afkaralp/ratkaradotdev` on every push to `main`, the host pulls it:

```sh
mkdir -p ~/ratkaradotdev && cp deploy/compose.yml ~/ratkaradotdev/
cd ~/ratkaradotdev && docker compose up -d

# crontab -e
*/5 * * * * cd ~/ratkaradotdev && docker compose pull -q && docker compose up -d
```

Behind Cloudflare, turn off "Always Use HTTPS": the server redirects browsers itself so `curl afkara.dev` gets text.

## NixOS

```nix
{
  imports = [ ratkaradotdev.nixosModules.default ];

  services.ratkara = {
    enable = true;
    openFirewall = true; # ssh on 2222, http stays on 127.0.0.1:8080
    siteUrl = "https://afkara.dev";
  };

  services.caddy = {
    enable = true;
    virtualHosts."afkara.dev".extraConfig = "reverse_proxy 127.0.0.1:8080";
  };
  networking.firewall.allowedTCPPorts = [ 80 443 ];
}
```

[ratzilla]: https://github.com/ratatui/ratzilla
[russh]: https://github.com/Eugeny/russh
