#!/bin/sh
# Stop and remove the containers that start.sh started. .pxna/state and config/pxna.toml are kept.
# Extra arguments are passed to `docker compose down`.
set -eu

cd "$(dirname "$0")"

exec docker compose down "$@"
