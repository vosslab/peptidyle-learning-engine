// Dense top-bar keyboard traversal assertions shared by M2 shell evidence.

import assert from "node:assert/strict";

async function assertVisibleKeyboardFocus(locator, name) {
  const focus = await locator.evaluate((element) => {
    const bounds = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    return {
      focused: document.activeElement === element,
      focusVisible: element.matches(":focus-visible"),
      inViewport:
        bounds.top >= 0 &&
        bounds.left >= 0 &&
        bounds.bottom <= window.innerHeight &&
        bounds.right <= window.innerWidth,
      outlineStyle: style.outlineStyle,
      outlineWidth: style.outlineWidth,
    };
  });
  assert.equal(focus.focused, true, `${name} receives browser keyboard focus`);
  assert.equal(focus.focusVisible, true, `${name} exposes keyboard focus styling`);
  assert.equal(focus.inViewport, true, `${name} remains reachable in the viewport`);
  assert.ok(
    focus.outlineStyle !== "none" && focus.outlineWidth !== "0px",
    `${name} retains a visible focus outline`,
  );
}

/**
 * Verifies the browser's forward and reverse Tab order across the product
 * ribbon's visible controls. Each entry is [Playwright Locator, description].
 */
export async function assertDenseTopBarKeyboardTraversal(page, orderedControls) {
  for (const [control, name] of orderedControls) {
    await page.keyboard.press("Tab");
    await assertVisibleKeyboardFocus(control, name);
  }
  for (const [control, name] of orderedControls.slice(0, -1).reverse()) {
    await page.keyboard.press("Shift+Tab");
    await assertVisibleKeyboardFocus(control, name);
  }
  await page.keyboard.press("Tab");
  await assertVisibleKeyboardFocus(
    orderedControls[1][0],
    "the Peptidyle home control after the skip link",
  );
}
