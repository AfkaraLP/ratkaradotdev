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

      packages = forEachSupportedSystem (
        { pkgs }:
        let
          inherit (pkgs) lib;
          craneLib = (inputs.crane.mkLib pkgs).overrideToolchain (_: pkgs.rustToolchain);
          src = lib.fileset.toSource {
            root = ./.;
            fileset = lib.fileset.unions [
              (craneLib.fileset.commonCargoSources ./.)
              ./crates/core/posts
              ./crates/web/index.html
            ];
          };
          siteUrl = "https://afkara.dev";
          common = {
            inherit src;
            strictDeps = true;
            pname = "ratkaradotdev";
            version = "0.1.0";
          };
        in
        let
          site = rec {
            server = craneLib.buildPackage (
              common
              // {
                pname = "ratkara-server";
                cargoExtraArgs = "-p ratkara-server";
                cargoArtifacts = craneLib.buildDepsOnly (common // { cargoExtraArgs = "-p ratkara-server"; });
                nativeBuildInputs = [ pkgs.makeWrapper ];
                postInstall = ''
                  wrapProgram $out/bin/ratkara-server --set-default RATKARA_WEB_DIR ${web}
                '';
                meta.mainProgram = "ratkara-server";
              }
            );

            pages = craneLib.buildPackage (
              common
              // {
                pname = "ratkara-pages";
                cargoExtraArgs = "-p ratkara-core --bin ratkara-pages";
              }
            );

            web = craneLib.buildTrunkPackage (
              common
              // {
                pname = "ratkara-web";
                cargoExtraArgs = "-p ratkara-web";
                # trunk resolves the crate from the working directory
                preBuild = "cd crates/web";
                trunkIndexPath = "index.html";
                installPhaseCommand = ''
                  cp -r dist $out
                  ${pages}/bin/ratkara-pages $out ${siteUrl}
                '';
                CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
                cargoArtifacts = craneLib.buildDepsOnly (
                  common
                  // {
                    cargoExtraArgs = "-p ratkara-web";
                    CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
                    doCheck = false;
                  }
                );
                wasm-bindgen-cli = pkgs.wasmBindgenCli;
              }
            );

            default = server;
          };
        in
        site
        // lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
          image = pkgs.dockerTools.buildLayeredImage {
            name = "ghcr.io/afkaralp/ratkaradotdev";
            tag = "latest";
            fakeRootCommands = ''
              mkdir -p data
              chown 65534:65534 data
            '';
            config = {
              Entrypoint = [ (lib.getExe site.server) ];
              User = "65534:65534";
              Env = [ "RATKARA_HOST_KEY=/data/host_key" ];
              ExposedPorts = {
                "2222/tcp" = { };
                "8080/tcp" = { };
              };
              Volumes."/data" = { };
              Labels."org.opencontainers.image.source" = "https://github.com/AfkaraLP/ratkaradotdev";
            };
          };
        }
      );

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
