#!/bin/sh
# Initialize the submodules, create config/pxna.toml if missing, then build and start the containers.
# Extra arguments are passed to `docker compose up`.
set -eu

cd "$(dirname "$0")"

if [ ! -e config/pxna.toml ]; then
    cp config/pxna.toml.example config/pxna.toml
    echo "Created config/pxna.toml from config/pxna.toml.example."
fi

git submodule update --init --recursive
exec docker compose up --build -d "$@"
