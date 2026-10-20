# Radix's Confirmation Dialog and Menu Propagate the Esc keydown, and a Tab in a Portal Child Reaches React Ancestors

**Source: Verified** (radix-ui 1.6.7, @radix-ui/react-dismissable-layer 1.1.19 / 2026-10-10)

One item of [knowledges.md](../knowledges.md).

## Fact

In Chromium 154.0.8037.98, for Radix's `DropdownMenu` and `AlertDialog` drawn by a Portal inside the outer frame element of a `dialog`, we confirmed the following.

- Opening the menu and pressing Esc closes the menu. The Esc also reaches the `window` bubble-phase `keydown` listener, `defaultPrevented` is true, and `target` is the element with `role="menu"`. Radix calls `preventDefault()` when it handles Esc, but does not stop propagation.
- React events propagate from a Portal child to React ancestors. A Tab inside the confirmation dialog reaches the `onKeyDown` of the ancestor `dialog`. If that `onKeyDown` calls `preventDefault()` on the Tab and moves focus to the next operable element, the confirmation dialog's focus trap returns the focus, and the Tab cannot move off "Delete" (削除する).

We did not check the `defaultPrevented` of the `AlertDialog` Esc `keydown`, or other versions of radix-ui.

## Basis

For Esc, we opened the viewer on a narrow screen (width 390) and ran the following with `playwright-cli run-code`.

```js
await page.evaluate(() => { window.__keys = []; window.addEventListener('keydown', (e) => window.__keys.push([e.key, e.defaultPrevented, e.target.getAttribute?.('role')])); });
await page.click('button[aria-label="その他"]');
await page.keyboard.press('Escape');
return await page.evaluate(() => window.__keys);
```

The result was `[["Escape", true, "menu"]]`.

For Tab, we temporarily removed the exclusion by `NESTED_LAYER` in [Modal.tsx](../../ui-web/src/components/Modal.tsx#L46), pressed the viewer's delete button on a wide screen, and with the confirmation dialog showing, pressed Tab 4 times.
The text of `document.activeElement` after each press was "削除する" (Delete) four times in a row. With the exclusion restored, it cycled through "削除する", "取り消す" (Cancel), "削除する", "取り消す".

## Implication

[Viewer.tsx](../../ui-web/src/components/Viewer.tsx#L257)'s `keydown` looks at `defaultPrevented` and does not handle Esc, so that a menu or dialog Radix closed with Esc stays closed and the viewer does not close too. Arrow keys are stopped by `nestedOpen` while a confirmation dialog or menu is open.
`NESTED_LAYER` in [Modal.tsx](../../ui-web/src/components/Modal.tsx#L13) excludes a Tab inside `role="alertdialog"` and `role="menu"` from the Modal's cycling and leaves it to Radix's focus trap.
This is the premise of the rules in [Client Design §3.5](../design/client.md#35-viewer) for key operations between nested dialogs and menus.
