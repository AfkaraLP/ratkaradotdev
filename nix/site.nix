{
  pkgs,
  crane,
  self,
}:

let
  inherit (pkgs) lib;
  craneLib = (crane.mkLib pkgs).overrideToolchain (_: pkgs.rustToolchain);
  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      (craneLib.fileset.commonCargoSources ../.)
      ../crates/core/posts
      ../crates/core/art
      ../crates/web/index.html
    ];
  };
  siteUrl = "https://afkara.dev";

  common = {
    inherit src;
    strictDeps = true;
    pname = "ratkaradotdev";
    version = "0.1.0";
    doCheck = false;
  };
  native = common // {
    cargoExtraArgs = "--workspace --exclude ratkara-web";
  };
  wasm = common // {
    cargoExtraArgs = "-p ratkara-web";
    CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
  };

  # dependencies are built once per lockfile and shared by every package and check
  nativeDeps = craneLib.buildDepsOnly native;
  wasmDeps = craneLib.buildDepsOnly wasm;

  packages = {
    server = craneLib.buildPackage (
      common
      // {
        pname = "ratkara-server";
        cargoExtraArgs = "-p ratkara-server";
        cargoArtifacts = nativeDeps;
        nativeBuildInputs = [ pkgs.makeWrapper ];
        postInstall = ''
          wrapProgram $out/bin/ratkara-server --set-default RATKARA_WEB_DIR ${packages.web}
        '';
        meta.mainProgram = "ratkara-server";
      }
    );

    pages = craneLib.buildPackage (
      common
      // {
        pname = "ratkara-pages";
        cargoExtraArgs = "-p ratkara-core --bin ratkara-pages";
        cargoArtifacts = nativeDeps;
      }
    );

    web = craneLib.buildTrunkPackage (
      wasm
      // {
        pname = "ratkara-web";
        cargoArtifacts = wasmDeps;
        # trunk resolves the crate from the working directory
        preBuild = "cd crates/web";
        trunkIndexPath = "index.html";
        installPhaseCommand = ''
          cp -r dist $out
          ${packages.pages}/bin/ratkara-pages $out ${siteUrl}
        '';
        wasm-bindgen-cli = pkgs.wasmBindgenCli;
      }
    );

    default = packages.server;
  }
  // lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
    image = pkgs.dockerTools.buildLayeredImage {
      name = "ghcr.io/afkaralp/ratkaradotdev";
      tag = "latest";
      contents = [ packages.server ];
      fakeRootCommands = ''
        mkdir -p data
        chown 65534:65534 data
      '';
      config = {
        Entrypoint = [ "/bin/ratkara-server" ];
        Healthcheck = {
          Test = [
            "CMD"
            "/bin/ratkara-server"
            "--health-check"
          ];
          # nanoseconds
          Interval = 30 * 1000000000;
          Timeout = 5 * 1000000000;
          StartPeriod = 5 * 1000000000;
          Retries = 3;
        };
        User = "65534:65534";
        Env = [ "RATKARA_HOST_KEY=/data/host_key" ];
        ExposedPorts = {
          "2222/tcp" = { };
          "8080/tcp" = { };
        };
        Volumes."/data" = { };
        Labels = {
          "org.opencontainers.image.source" = "https://github.com/AfkaraLP/ratkaradotdev";
          "org.opencontainers.image.revision" = self.rev or self.dirtyRev or "unknown";
        };
      };
    };
  };

  checks = {
    inherit (packages) server web;
    fmt = craneLib.cargoFmt { inherit (common) src pname version; };
    quadlet = pkgs.runCommand "quadlet-check" { } ''
      QUADLET_UNIT_DIRS=${../deploy} ${pkgs.podman}/libexec/podman/quadlet -dryrun -user > $out
      grep -q -- '--sdnotify=healthy' $out
      grep -q -- '--health-cmd "\[' $out
    '';
    clippy = craneLib.cargoClippy (
      native
      // {
        cargoArtifacts = nativeDeps;
        cargoClippyExtraArgs = "--all-targets -- -D clippy::pedantic";
      }
    );
    clippy-web = craneLib.cargoClippy (
      wasm
      // {
        cargoArtifacts = wasmDeps;
        cargoClippyExtraArgs = "-- -D clippy::pedantic";
      }
    );
  };
in
{
  inherit packages checks;
}
