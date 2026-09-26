// ProvidedAvatarPicker browser contract.
// Selector contract: provided_avatar_picker.tsx exposes native radio values from the generated
// catalog and accessible Gallery/List presentation controls.

import assert from "node:assert/strict";

import { PROVIDED_AVATAR_CATALOG } from "../../src/features/profile_avatar/avatar_catalog_generated.ts";
import { openProvidedAvatarPickerHarness } from "./helper_provided_avatar_picker_harness.mjs";
import { CANONICAL_VIEWPORTS } from "./screenshot_corpus/manifest.ts";
const EXPECTED_AVATAR_IDS = PROVIDED_AVATAR_CATALOG.filter((entry) => entry.isSelectable).map(
  (entry) => entry.id,
);

function avatarIds(group) {
  return group
    .locator("[data-record-id]")
    .evaluateAll((elements) => elements.map((element) => element.getAttribute("data-record-id")));
}

function checkedAvatarIds(page) {
  return page
    .locator('input[type="radio"]:checked')
    .evaluateAll((inputs) => inputs.map((input) => input.getAttribute("value")));
}

async function waitForSelectedAvatar(page, avatarId) {
  await page.waitForFunction(
    (expectedAvatarId) =>
      document
        .querySelector("[data-provided-avatar-picker-current]")
        ?.getAttribute("data-provided-avatar-picker-current") === expectedAvatarId,
    avatarId,
  );
}

const { browser, consoleErrors, harnessServer, page, pageErrors } =
  await openProvidedAvatarPickerHarness();

try {
  const group = page.getByRole("group", { name: "Choose an avatar", exact: true });
  assert.deepEqual(await avatarIds(group), EXPECTED_AVATAR_IDS, "Gallery keeps every catalog ID");
  assert.deepEqual(
    await checkedAvatarIds(page),
    [EXPECTED_AVATAR_IDS[0]],
    "one Gallery radio starts selected",
  );

  const galleryRadios = page.getByRole("radio");
  await galleryRadios.first().focus();
  await page.keyboard.press("ArrowDown");
  const keyboardSelectedAvatarId = EXPECTED_AVATAR_IDS[1];
  assert.notEqual(keyboardSelectedAvatarId, undefined, "catalog has a second selectable avatar");
  await waitForSelectedAvatar(page, keyboardSelectedAvatarId);
  assert.deepEqual(
    await checkedAvatarIds(page),
    [keyboardSelectedAvatarId],
    "native ArrowDown selects exactly one next Gallery radio",
  );

  for (const [profileId, viewport] of Object.entries(CANONICAL_VIEWPORTS)) {
    await page.setViewportSize({ width: viewport.width, height: viewport.height });
    for (const presentation of ["Gallery", "List"]) {
      const button = page.getByRole("button", { name: presentation, exact: true });
      await button.click();
      assert.equal(await button.getAttribute("aria-pressed"), "true");
      await group.waitFor({ state: "visible" });
      assert.deepEqual(
        await avatarIds(group),
        EXPECTED_AVATAR_IDS,
        `${profileId} ${presentation} retains catalog identity and order`,
      );
      assert.deepEqual(
        await checkedAvatarIds(page),
        [keyboardSelectedAvatarId],
        `${profileId} ${presentation} retains the selected avatar`,
      );
    }
  }
  assert.deepEqual(pageErrors, [], "picker harness has no browser page errors");
  assert.deepEqual(consoleErrors, [], "picker harness has no browser console errors");
  process.stdout.write(
    "ProvidedAvatarPicker Gallery/List browser contract passed across four viewports.\n",
  );
} finally {
  await browser.close();
  await harnessServer.close();
}
