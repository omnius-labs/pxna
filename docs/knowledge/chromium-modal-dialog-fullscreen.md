# Chromium's Modal Dialog Cannot Be Made Full Screen, and Elements Drawn Outside the Dialog Cannot Be Operated

**Source: Verified** (Chromium 154.0.8037.98 / 2026-10-10)

One item of [knowledge.md](../knowledge.md).

## Fact

In Playwright's Chromium 154.0.8037.98 on macOS, we confirmed the following for a `dialog` opened with `dialog.showModal()`.

- Calling `requestFullscreen()` on the `dialog` element is rejected with `TypeError: Dialog elements are invalid`, even within a user interaction.
- `requestFullscreen()` succeeds on an element that is a descendant of the `dialog`, and `document.fullscreenElement` becomes that element. An element drawn as a descendant of the full-screen element is visible and accepts clicks inside the full screen.
- While the modal is open, an element added directly under `body` with `position: fixed` ends up behind the modal even with a large `z-index`. `document.elementFromPoint` returns the `dialog` side, and clicks do not reach it. The same element added as a descendant of the `dialog` is visible and accepts clicks.
- When the full-screen element is removed from the DOM, `document.fullscreenElement` becomes `null`.

We did not check browsers other than Chromium, real devices, or the path on which the browser itself handles Esc during full screen.
`keyboard.press` of `playwright-cli` delivers `keydown` to the page even during full screen, and the full screen ended with `fullscreenchange`. Real browsers may not pass Esc during full screen as a `keydown`.

## Basis

After `playwright-cli open about:blank`, we ran the following with `playwright-cli run-code --filename=probe.js`.

```js
async page => {
  await page.setContent(`<body><dialog id="d"><div id="o" style="height:100%"></div></dialog><button id="start">start</button>
    <script>
      window.result = {};
      document.getElementById('start').onclick = () => {
        document.getElementById('d').showModal();
        const add = (id, parent, top) => { const b = document.createElement('button'); b.id = id; b.textContent = id; b.style.cssText = 'position:fixed;left:50px;z-index:99999;top:' + top + 'px'; parent.append(b); };
        add('inBody', document.body, 50);
        add('inOuter', document.getElementById('o'), 120);
        for (const [id, sel, left] of [['fsDialog', '#d', 0], ['fsOuter', '#o', 200]]) {
          const b = document.createElement('button'); b.id = id; b.textContent = id; b.style.cssText = 'position:fixed;bottom:0;z-index:99999;left:' + left + 'px';
          b.onclick = async () => { try { await document.querySelector(sel).requestFullscreen(); window.result[id] = 'ok:' + document.fullscreenElement.id; await document.exitFullscreen(); } catch (e) { window.result[id] = e.name + ': ' + e.message; } };
          document.getElementById('o').append(b);
        }
      };
    </script></body>`);
  await page.click('#start');
  const hit = await page.evaluate(() => ['inBody', 'inOuter'].map((id) => { const e = document.getElementById(id); const r = e.getBoundingClientRect(); return [id, document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2) === e]; }));
  await page.click('#fsDialog'); await page.click('#fsOuter');
  return { hit, fs: await page.evaluate(() => window.result) };
}
```

The result was `hit` of `[["inBody", false], ["inOuter", true]]` and `fs` of `{"fsDialog": "TypeError: Dialog elements are invalid", "fsOuter": "ok:o"}`.
For the check of removing the full-screen element, in a separate procedure, immediately after `requestFullscreen()` on the outer frame element succeeded, we ran `document.getElementById('d').close()` and `remove()` and saw `document.fullscreenElement` become `null`.

## Implication

This is the premise of the decision in [Client Design §5.1](../design/client.md#draw-the-viewers-confirmation-dialogs-menus-and-toasts-inside-the-outer-frame-element).
[Modal.tsx](../../ui-web/src/components/Modal.tsx#L72) passes the outer frame element, and [Viewer.tsx](../../ui-web/src/components/Viewer.tsx#L207) makes that element full screen and also uses it as the target for drawing confirmation dialogs, menus, and toasts.
Making the `dialog` element full screen, or going back to a Portal directly under `body`, does not work.
[useFullscreen.ts](../../ui-web/src/viewer/useFullscreen.ts#L6) does not throw an exception when the full-screen request is refused, and simply does not enter full screen.
