{ pkgs }:

pkgs.writeShellApplication {
  name = "ratkara-deploy";
  runtimeInputs = with pkgs; [
    coreutils
    curl
  ];
  text = ''
    image=ghcr.io/afkaralp/ratkaradotdev:latest
    health=http://127.0.0.1:6701/
    state=''${XDG_STATE_HOME:-$HOME/.local/state}/ratkaradotdev
    compose() { docker compose -p ratkaradotdev -f ${../deploy/compose.yml} "$@"; }
    log() { echo "$(date -Is) $*"; }
    mkdir -p "$state"

    running=$(docker inspect --format '{{.Image}}' "$(compose ps -q ratkara)" 2>/dev/null || true)
    docker pull -q "$image" > /dev/null
    latest=$(docker image inspect --format '{{.Id}}' "$image")

    [ "$latest" = "$running" ] && exit 0
    [ "$latest" = "$(cat "$state/bad" 2>/dev/null || true)" ] && exit 0

    log "deploying $latest"
    compose up -d
    for _ in $(seq 30); do
      if curl -fs -o /dev/null "$health"; then
        log "healthy"
        docker image prune -f > /dev/null
        exit 0
      fi
      sleep 1
    done

    log "unhealthy, rolling back to $running"
    echo "$latest" > "$state/bad"
    docker tag "$running" "$image"
    compose up -d
  '';
}
