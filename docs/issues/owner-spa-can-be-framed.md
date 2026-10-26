# The Owner SPA Can Be Framed by a Same-Site Origin

**Severity: medium** (framing confirmed; the two-click reproduction was not executed)

Investigated 2026-10-25 / Target commit `4f071e1`

One item of [issues.md](../issues.md). Found by the security audit of 2026-10-24. The framing condition was confirmed in the second pass; the click-through itself was not executed in a browser.

## Symptom

The daemon serves the web client and the API without `X-Frame-Options`, a `frame-ancestors` directive, or `Cross-Origin-Resource-Policy`. A page on a same-site origin, such as another port on the same host, can frame the authenticated client and lay its own controls over it.

The session cookie is `SameSite=Strict`, which does not prevent this: a same-site frame still receives the cookie. The audit confirmed that the response carries no framing headers:

```
HTTP/1.1 200 OK
content-type: text/html
accept-ranges: bytes
last-modified: ...
```

## Locations

- [server.rs#L58](../../daemon/entrypoints/daemon/src/interface/server.rs#L58) — `router` builds the API and the static layer and applies only `UnauthorizedBody` and `OriginCheck`; no response header limits framing
- [session.rs#L62](../../daemon/entrypoints/daemon/src/interface/session.rs#L62) — the cookie attributes, which do not stop same-site framing
- [BrowsePage.tsx#L104](../../ui-web/src/pages/BrowsePage.tsx#L104) — the folder view whose delete control is the target of the two clicks

## Cause

No layer sets framing policy. The daemon was designed to be reached directly on localhost or a trusted LAN, and the threat of a page on another origin of the same host was not considered.

## Impact

An attacker who can host a page on a same-site origin can overlay the client and make the owner trigger a destructive action, such as a recursive delete, with two clicks. The attacker cannot read the result. A page on a different site cannot send the `SameSite=Strict` cookie.

The end-to-end click-through was not reproduced, so the severity rests on the framing precondition plus the absence of any interaction guard on the destructive controls.

## Handling Policy

Add `Content-Security-Policy: frame-ancestors 'none'` and `X-Frame-Options: DENY` to every response, and `Cross-Origin-Resource-Policy: same-origin` to the API and asset responses. This is cheap and does not depend on the rest of the clickjacking analysis, so it can be applied before the reproduction.

The regression test asserts the headers on the SPA entry, on an API response, and on a thumbnail response.
