<p align="center">
<img width="128" src="https://github.com/omnius-labs/pxna/blob/main/docs/logo.png?raw=true" alt="Pxna logo">
</p>

<h1 align="center">Pxna - Omnius Remote File Manager (Work in Progress)</h1>

[![test-daemon-all](https://github.com/omnius-labs/pxna/actions/workflows/test-daemon-all.yml/badge.svg)](https://github.com/omnius-labs/pxna/actions/workflows/test-daemon-all.yml)

Pxna is a self-hosted remote file manager. A Rust daemon serves files from directories on your server (Libraries), and a web client lets you browse, view, search, and manage them from a browser.

## Features

- Browse a Library as a tree, grid, or list, with thumbnails for images and videos
- View images and play videos, including on-the-fly HLS transcoding of formats a browser cannot play
- Timeline view that groups media by date
- Ratings and tags, with filtering in search
- Upload, rename, move, copy, and delete files
- Single-owner authentication with server-side sessions
- Docker image with ffmpeg included

The design documents are in [docs/design.md](docs/design.md).

## Development

### Repository Setup

Please initialize the submodules right after cloning.

```sh
git submodule update --init
```

### Installing Required Dependencies

#### Debian and Ubuntu

```sh
sudo apt-get install -y libclang-dev
```
This package is required for RocksDB.

#### Node.js

Node.js is managed with [mise](https://mise.jdx.dev/). Run `mise install` in the repository root to install the version pinned in `mise.toml` and `mise.lock`. This also provides the `openapi-typescript` command used by `npm run gen:api`.

### Running the Web Client

The web client (`ui-web/`) is a Vite and React single-page application. The daemon serves its build output on the same port as the API.

```sh
# Build the web client. daemon/config/pxna.toml.example points [web] dir at ui-web/dist.
cd ui-web
npm ci
npm run build

# Create your own configuration from the example. daemon/config/pxna.toml is not tracked by git.
cd ../daemon
cp config/pxna.toml.example config/pxna.toml

# Start the daemon. It listens on 0.0.0.0:6050 and serves the web client at http://localhost:6050/.
cargo run -p omnius-pxna-daemon -- start --config-dir ./config
```

While changing the web client, run `npm run dev` in `ui-web/` instead of rebuilding. The dev server proxies `/api` to `http://localhost:6050`.
After editing `daemon/openapi.yaml`, run `npm run gen:api` in `ui-web/` to regenerate the API types.

The application uses TypeScript 7. Lint and formatting are enforced by Biome; run `npm run lint` in `ui-web/` (Biome itself is independent of the TypeScript version).
API type generation runs the mise-managed `openapi-typescript`, so run `mise install` once before `npm run gen:api`.

### Adding UI Components

UI components are checked into `ui-web/src/components/ui/`; the shadcn CLI is not a project dependency.
When adding a component, run the pinned CLI from `ui-web/`:

```sh
npx --yes shadcn@4.21.4 add <component>
```

Use trusted registry configuration and local paths.
The CLI still depends on [braces with an unpatched denial-of-service vulnerability](https://github.com/advisories/GHSA-vfj7-8cjw-p6xm); removing it from the project does not patch the CLI itself.

### Linting

```sh
(cd daemon && cargo make lint)
(cd ui-web && npm run lint)
```

## Running in Docker

Initialize the submodules, then build and start the container:

```sh
cp .env.example .env
# Edit .env and set PXNA_LIBRARY_DIR to the absolute path of your library.
docker compose up --build -d
```

Open `http://localhost:6050` and create the owner account. The image includes the SPA, ffmpeg, and ffprobe.
The container listens on `127.0.0.1` only. Change the port with `PXNA_PORT`.
The container runs as `PXNA_UID:PXNA_GID` (default `1000:1000`), so match them to the owner of the library (`id -u`, `id -g`) when you want to modify files.
The SQLite database, uploads, and caches live in `.pxna/state`, which survives restarts. The `init` service sets its owner on startup.
The configuration is [config/pxna.toml](config/pxna.toml), mounted read-only at `/config/pxna.toml`; Library paths refer to paths inside the container.
`TZ` (default `Asia/Tokyo`) sets the time zone used by the timeline.
The image built from the `Dockerfile` runs as UID/GID `10001:10001`, and its configuration directory must be writable so the daemon can create its process lock. Set `[web] dir` to `/opt/pxna/web` when supplying another configuration.

## Contributing

This project is not accepting external contributions for now.

## License

Pxna is licensed under the [GNU Affero General Public License v3.0](LICENSE).

## Links

- Official Documentation: https://docs.omnius-labs.com/
