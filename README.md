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
nix flake check                               # fmt, clippy pedantic (native + wasm), quadlet, builds
```

## Nix

```sh
nix build .#web      # static files with prerendered embeds
nix run .            # ratkara-server, serving .#web on :8080 and ssh on :2222
```

`wasm-bindgen` is pinned in `crates/web/Cargo.toml` and has to match `wasmBindgenCli` in `flake.nix`.

## Deploy

CI checks, smoke tests the image and, on `main`, publishes `ghcr.io/afkaralp/ratkaradotdev` (`latest` + `sha-*`, with build provenance). The host runs it as a rootless podman quadlet. `podman auto-update` pulls new images every 5 minutes, and since the unit only counts as started once `--health-check` passes, a bad image gets rolled back automatically:

```sh
mkdir -p ~/.config/containers/systemd ~/.config/systemd/user/podman-auto-update.timer.d
cp deploy/ratkara.container ~/.config/containers/systemd/
cp deploy/auto-update.conf ~/.config/systemd/user/podman-auto-update.timer.d/
systemctl --user daemon-reload
systemctl --user start ratkara
systemctl --user enable --now podman-auto-update.timer
loginctl enable-linger
```

Verify an image: `gh attestation verify oci://ghcr.io/afkaralp/ratkaradotdev:latest -R AfkaraLP/ratkaradotdev`

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
