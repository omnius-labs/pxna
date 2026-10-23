#!/bin/sh
# Initialize the submodules, then build and start the containers.
# Extra arguments are passed to `docker compose up`.
set -eu

cd "$(dirname "$0")"

git submodule update --init --recursive
exec docker compose up --build -d "$@"
