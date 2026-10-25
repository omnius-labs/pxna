# Pre-Auth Password Hashing Has No Concurrency Limit

**Severity: medium** (confirmed by reproduction; unauthenticated requests allocate memory)

Investigated 2026-10-25 / Target commit `4f071e1`

One item of [issues.md](../issues.md). Found by the security audit of 2026-10-24 and measured in the second pass. The rate limit itself is deferred in [design.md §11.2](../design.md#limiting-login-attempts); this item covers the resource bound, which does not depend on the exposure policy.

## Symptom

Every unauthenticated login and setup request runs argon2id on the blocking pool with no limit on how many run at once. Each run holds about 16 MB of resident memory for its duration, so a small number of concurrent requests consumes the host's memory.

Measured on a 7 GiB host with 4 GiB available:

```
concurrency 100 -> peak RSS 1,602,576 KB (+1.5 GB)
concurrency 300 -> peak RSS 4,088,152 KB (+4.0 GB)
```

No request needed credentials; every one used a wrong password. Authenticated reads stayed fast, so the effect is memory pressure rather than a general slowdown.

## Locations

- [password.rs#L9](../../daemon/modules/engine/src/service/auth/password.rs#L9) — `hash`, which runs `Argon2::default()` inside `spawn_blocking` with no permit
- [password.rs#L26](../../daemon/modules/engine/src/service/auth/password.rs#L26) — `verify`, which resolves the stored hash on the blocking pool in the same way
- [auth.rs#L45](../../daemon/entrypoints/daemon/src/interface/features/auth.rs#L45) — `login` reaches `verify` before any credential is checked
- [setup.rs#L20](../../daemon/entrypoints/daemon/src/interface/features/setup.rs#L20) — `createInitialSetup` reaches `hash` while the daemon is uninitialized

## Cause

`Argon2::default()` uses `m=19,456 KiB, t=2, p=1`. The work is moved off the async threads but nothing bounds how many blocking jobs exist at once, and Tokio's blocking pool defaults to 512 threads. The memory cost of one request is therefore multiplied by the number of requests, and the product reaches several GiB.

## Impact

A device on the same network as the daemon can allocate gigabytes of memory without credentials and push the process into the OOM killer. The condition for the deferred rate limit assumes the operator trusts the network; the memory ceiling applies even there, because a single misbehaving device or a page in the browser is enough.

## Handling Policy

Put a semaphore around pre-auth hashing and verification, sized so that the worst case fits the memory the daemon is willing to spend, and reject with `429` once a bounded queue is full. Keep the existing deferred item for the attempt-rate policy; this issue does not decide it.

A regression test fires more concurrent logins than the permit count and asserts that the excess is rejected with `429` and that the process RSS stays under the configured bound.
