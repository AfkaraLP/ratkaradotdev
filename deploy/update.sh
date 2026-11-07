#!/bin/sh
set -eu
cd "$(dirname "$0")"

image=ghcr.io/afkaralp/ratkaradotdev:latest
health=http://127.0.0.1:6701/
log() { echo "$(date -Is) $*"; }

running=$(docker inspect --format '{{.Image}}' "$(docker compose ps -q ratkara)" 2>/dev/null || true)
docker pull -q "$image" > /dev/null
latest=$(docker image inspect --format '{{.Id}}' "$image")

[ "$latest" = "$running" ] && exit 0
[ "$latest" = "$(cat bad 2>/dev/null || true)" ] && exit 0

log "deploying $latest"
docker compose up -d
for _ in $(seq 30); do
    if curl -fs -o /dev/null "$health"; then
        log "healthy"
        docker image prune -f > /dev/null
        exit 0
    fi
    sleep 1
done

log "unhealthy, rolling back to $running"
echo "$latest" > bad
docker tag "$running" "$image"
docker compose up -d
