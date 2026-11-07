self:
{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.services.ratkara;
in
{
  options.services.ratkara = {
    enable = lib.mkEnableOption "the ratkara site server (ssh + http)";

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.stdenv.hostPlatform.system}.server;
    };

    sshHost = lib.mkOption {
      type = lib.types.str;
      default = "0.0.0.0";
    };

    httpHost = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
      description = "Put a reverse proxy in front for TLS, it also handles slow clients.";
    };

    siteUrl = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      example = "https://afkara.dev";
      description = "Base for links in curl responses, otherwise taken from the Host header.";
    };

    sshPort = lib.mkOption {
      type = lib.types.port;
      default = 2222;
    };

    httpPort = lib.mkOption {
      type = lib.types.port;
      default = 8080;
    };

    openFirewall = lib.mkOption {
      type = lib.types.bool;
      default = false;
    };
  };

  config = lib.mkIf cfg.enable {
    systemd.services.ratkara = {
      wantedBy = [ "multi-user.target" ];
      wants = [ "network-online.target" ];
      after = [ "network-online.target" ];
      environment = {
        RATKARA_HOST = cfg.sshHost;
        RATKARA_HTTP_HOST = cfg.httpHost;
        RATKARA_SSH_PORT = toString cfg.sshPort;
        RATKARA_HTTP_PORT = toString cfg.httpPort;
        RATKARA_HOST_KEY = "/var/lib/ratkara/host_key";
      }
      // lib.optionalAttrs (cfg.siteUrl != null) { RATKARA_SITE_URL = cfg.siteUrl; };
      serviceConfig = {
        ExecStart = lib.getExe cfg.package;
        DynamicUser = true;
        StateDirectory = "ratkara";
        Restart = "on-failure";
        NoNewPrivileges = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        PrivateTmp = true;
      };
    };

    networking.firewall.allowedTCPPorts = lib.mkIf cfg.openFirewall [ cfg.sshPort ];
  };
}
