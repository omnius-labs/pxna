# Chromium Shows a Leave Confirmation When a beforeunload Is Canceled After an Operation

**Source: Verified** (Chromium 155.0.8059.39 / 2026-10-07)

One item of [knowledge.md](../knowledge.md).

## Fact

In headless Chromium 155.0.8059.39 on Ubuntu, after clicking the page, when a `beforeunload` listener ran `preventDefault()` and set `returnValue = ""`, the browser's leave confirmation appeared on reload. A reload after removing the listener did not show it.
We did not check before any user interaction, other browsers, or closing a tab on a real device.

## Basis

With playwright-cli, we clicked the page and uploaded a 33 MiB file. After 16 MiB had been sent, we held the next chunk with a route and ran `playwright-cli reload`. We confirmed the native `beforeunload` dialog and returned to the page with `dialog-dismiss`.
After the upload was canceled and the listener removed, the same `reload` completed without a dialog. The listener does the following.

```js
const beforeUnload = event => {
    event.preventDefault();
    event.returnValue = "";
};
window.addEventListener("beforeunload", beforeUnload);
// On completion, remove it with the same function.
window.removeEventListener("beforeunload", beforeUnload);
```

The results on the real browser are recorded in [Acceptance Verification of C8b](../reviews/2026-10-07-web-entry-transfer.md#5-what-was-checked-and-found-sound).

## Implication

This fact is the premise of the leave confirmation during uploads in [Client Design §3.7](../design/client.md#37-entry-operations). Rechecking the confirmation requires a state in which a user interaction has happened.
