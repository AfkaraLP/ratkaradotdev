{
  description = "AfkaraLP's site as a TUI: ratzilla in the browser, ssh and curl on one server";

  inputs = {
    nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.1"; # unstable Nixpkgs
    fenix = {
      url = "https://flakehub.com/f/nix-community/fenix/0.1";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    { self, ... }@inputs:

    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forEachSupportedSystem =
        f:
        inputs.nixpkgs.lib.genAttrs supportedSystems (
          system:
          f {
            pkgs = import inputs.nixpkgs {
              inherit system;
              overlays = [
                inputs.self.overlays.default
              ];
            };
          }
        );
      site =
        pkgs:
        import ./nix/site.nix {
          inherit pkgs self;
          inherit (inputs) crane;
        };
    in
    {
      overlays.default = final: prev: {
        rustToolchain =
          with inputs.fenix.packages.${prev.stdenv.hostPlatform.system};
          combine (
            with stable;
            [
              clippy
              rustc
              cargo
              rustfmt
              rust-src
              targets.wasm32-unknown-unknown.stable.rust-std
            ]
          );
        # has to match the wasm-bindgen pin in crates/web/Cargo.toml
        wasmBindgenCli = final.wasm-bindgen-cli_0_2_130;
      };

      nixosModules.default = import ./nix/module.nix self;

      packages = forEachSupportedSystem ({ pkgs }: (site pkgs).packages);

      checks = forEachSupportedSystem ({ pkgs }: (site pkgs).checks);

      devShells = forEachSupportedSystem (
        { pkgs }:
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              rustToolchain
              rust-analyzer
              trunk
              wasmBindgenCli
              cargo-watch
              nixfmt
            ];

            env = {
              # Required by rust-analyzer
              RUST_SRC_PATH = "${pkgs.rustToolchain}/lib/rustlib/src/rust/library";
            };
          };
        }
      );
    };
}
