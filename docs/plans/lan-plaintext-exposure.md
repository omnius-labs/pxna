# Let the Container Be Published on the LAN in Plain HTTP

## 1. About This Plan

This work lets the operator publish the container started by `compose.yaml` to other devices on the LAN or a VPN such as Tailscale with `PXNA_BIND`, and lets them log in over plain HTTP.

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | The steps and acceptance checks of this work |
| [design.md §11.1](../design.md#leave-encryption-to-the-network-and-serve-plain-http) | The decision and its reasons |
| [design.md §9.1](../design.md#91-configuration-file), [§9.3](../design.md#93-distribution) | The meaning of `insecure_cookie` and what the container is given |
| [README.md](../../README.md) | The setup procedure that this work rewrites |

[design.md](../design.md#11-division-of-responsibility-between-documents) holds the map of the document family.

### 1.2 Creation Date and Target Commit

Created 2026-10-23 / Target commit `47b81b0`

## 2. Purpose and Completion Conditions

With `PXNA_BIND=0.0.0.0` set, a phone on the same LAN or VPN can open `http://<host>:6050` after `docker compose up`, create the owner, and stay logged in. Without it, the container stays on `127.0.0.1`.

### 2.1 Completion Conditions

1. `docker compose config` shows the `pxna` service publishing container port 6050 on `127.0.0.1` by default, and on the address of `PXNA_BIND` when it is set
2. A login through the host's LAN address over plain HTTP returns a `Set-Cookie` of `pxna_session` without `Secure`, and a following authenticated request with that cookie returns 200
3. `config/pxna.toml` sets `[web] insecure_cookie = true`, and the daemon's default stays `false`
4. `README.md` and `.env.example` describe `PXNA_BIND` and its default `127.0.0.1`, warn that `0.0.0.0` exposes the container, and state that access from outside the LAN goes through a VPN such as Tailscale
5. `cd daemon && cargo make lint` passes

### 2.2 Non-Goals

- Changing the daemon's default of `insecure_cookie`. It stays `false` by the decision in design.md §11.1.
- Documenting a Tailscale Serve or reverse-proxy setup. It was rejected in design.md §11.1.
- Limiting login attempts. It stays a deferred item in [design.md §11.2](../design.md#limiting-login-attempts).

## 3. Current State

[compose.yaml#L28](../../compose.yaml#L28) publishes `127.0.0.1:${PXNA_PORT:-6050}:6050`, so other devices cannot reach the container.
[config/pxna.toml#L9](../../config/pxna.toml#L9) has a `[web]` table without `insecure_cookie`, so the session cookie keeps `Secure` and is rejected by browsers over plain HTTP from anywhere other than `localhost`.
[README.md#L100](../../README.md#L100) states that the container listens on `127.0.0.1` only, and the committed [.env.example#L10](../../.env.example#L10) describes `PXNA_PORT` as the port on `127.0.0.1`. Neither mentions an address variable.
Without these changes, the plain-HTTP operation decided in design.md §11.1 cannot log in from a phone even if the operator changes the published address by editing `compose.yaml`.

## 4. Change Policy

Change only the files of the compose setup and its documentation. The daemon's code does not change.

### 4.1 Decisions Assumed

| Decision | Source |
| --- | --- |
| Serve plain HTTP, publish on the address of `PXNA_BIND` (default `127.0.0.1`), and enable `insecure_cookie` only in the bundled configuration | [design.md §11.1](../design.md#leave-encryption-to-the-network-and-serve-plain-http) |

### 4.2 Scope of Impact

`compose.yaml`, `config/pxna.toml`, `README.md`, and `.env.example` change.
An operator who already runs the compose setup keeps the port on `127.0.0.1` at the next `docker compose up` unless they set `PXNA_BIND`. Nothing else becomes incompatible.
The session cookie name changes to `pxna_session` for that setup, so existing browser sessions have to log in again once.

## 5. Steps

| No. | Content | Dependencies | State |
| --- | --- | --- | --- |
| [1](#s-1) | Take the published address from `PXNA_BIND` | - | Not started |
| [2](#s-2) | Enable `insecure_cookie` in the bundled configuration | - | Not started |
| [3](#s-3) | Rewrite the setup documentation | 1, 2 | Not started |

<a id="s-1"></a>
### Step 1. Take the Published Address from `PXNA_BIND`

**Locations to change**
[compose.yaml#L28](../../compose.yaml#L28)

**What to do**
Change the port mapping to `${PXNA_BIND:-127.0.0.1}:${PXNA_PORT:-6050}:6050`.

**Completion condition**
`docker compose config` shows `host_ip: 127.0.0.1` by default, and `host_ip: 0.0.0.0` when `PXNA_BIND=0.0.0.0`.

<a id="s-2"></a>
### Step 2. Enable `insecure_cookie` in the Bundled Configuration

**Locations to change**
[config/pxna.toml#L9](../../config/pxna.toml#L9)

**What to do**
Add `insecure_cookie = true` under `[web]`, with a comment that the compose setup is reached over plain HTTP. Leave [daemon/config/pxna.toml.example#L11](../../daemon/config/pxna.toml.example#L11) commented out.

**Completion condition**
The container started by compose returns `Set-Cookie: pxna_session=...` without `Secure` on login.

<a id="s-3"></a>
### Step 3. Rewrite the Setup Documentation

**Locations to change**
[README.md#L100](../../README.md#L100), [.env.example#L10](../../.env.example#L10)

**What to do**
State that the container listens on `127.0.0.1` by default, that setting `PXNA_BIND` (for example to `0.0.0.0`) exposes it in plain HTTP and is meant for a trusted LAN, and that access from outside the LAN goes through a VPN such as Tailscale. Add `PXNA_BIND` to `.env.example` and drop "127.0.0.1" from the `PXNA_PORT` comment.

**Completion condition**
`rg -n 'PXNA_BIND' README.md .env.example` returns a line in each file, and `README.md` warns that `0.0.0.0` exposes the container.

## 6. Acceptance Checks

| Condition | Command | Expected result |
| --- | --- | --- |
| 1 | `docker compose config pxna \| rg -A4 'ports:'`, then the same with `PXNA_BIND=0.0.0.0` set | `host_ip: 127.0.0.1` for the first, `host_ip: 0.0.0.0` for the second, both with `target: 6050` and `published: "6050"` |
| 2 | With `PXNA_BIND=0.0.0.0` set, from another device: `curl -si -X POST http://<host LAN address>:6050/api/v1/auth/login -H 'Content-Type: application/json' -d '{"username":"<owner>","password":"<password>","sessionTransport":"cookie"}'`, then `curl -si -b 'pxna_session=<value>' http://<host LAN address>:6050/api/v1/auth/me` | The first response has `Set-Cookie: pxna_session=` without `Secure`, and the second returns 200 |
| 3 | `rg -n 'insecure_cookie' config/pxna.toml daemon/config/pxna.toml.example daemon/entrypoints/daemon/src/config.rs` | `config/pxna.toml` has `insecure_cookie = true`, the example keeps it commented, and the config default is still `false` |
| 4 | `rg -n 'PXNA_BIND\|Tailscale\|0\.0\.0\.0' README.md .env.example` | A line with `PXNA_BIND` in each file, a line that mentions Tailscale, and a line in `README.md` that warns about `0.0.0.0` |
| 5 | `cd daemon && cargo make lint` | Exits with 0 |
