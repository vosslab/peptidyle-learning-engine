// Question Pool review temporarily replaces, then restores, Library search.

import assert from "node:assert/strict";
import test from "node:test";

import { openRibbonShellEvidencePage } from "./ribbon_shell_helpers.mjs";

const selectedAnchor = "7K3M-79QP";
const alreadyInPool = "2R5X-E7YA";

test("Question Pool review restores the Library task when the selection changes", async () => {
  const { browser, consoleErrors, harnessServer, page, pageErrors } =
    await openRibbonShellEvidencePage();
  try {
    await page.evaluate(() => window.ribbonShell.releaseSession());
    await page.evaluate(() => window.ribbonShell.currentNavigate("/library/browse?tag=protein"));

    const library = page.locator(
      '[data-m10-case="current-production"] [data-route-surface="library-browse"]',
    );
    await library.getByRole("heading", { name: "Browse Question Library", exact: true }).waitFor({
      state: "visible",
    });
    const createPoolButton = library.getByRole("button", { name: "Create Question Pool" });
    await createPoolButton.click();

    const picker = page.getByRole("dialog");
    await picker.waitFor({ state: "visible" });
    await picker.getByRole("radio").first().check();
    await picker.getByRole("button", { name: "Review selected Questions" }).click();
    await library.getByRole("heading", { name: "Create Question Pool", exact: true }).waitFor({
      state: "visible",
    });
    await createPoolButton.waitFor({ state: "hidden" });

    await library.getByRole("button", { name: "Choose different Questions" }).click();
    await picker.waitFor({ state: "visible" });
    const anchoredRows = picker.getByRole("list", { name: "Question results", exact: true });
    await anchoredRows.waitFor();
    assert.equal(
      await anchoredRows.locator(`[data-record-id="${selectedAnchor}"]`).count(),
      0,
      "the exact anchor stays in the tray rather than appearing as another candidate",
    );
    await anchoredRows.locator(`[data-record-id="${alreadyInPool}"]`).waitFor();
    await picker.getByRole("combobox", { name: "Backend", exact: true }).waitFor();
    assert.equal(
      await picker.getByRole("combobox", { name: "Backend", exact: true }).isDisabled(),
      true,
      "the anchored Pool backend constrains subsequent discovery",
    );
    await createPoolButton.waitFor({ state: "visible" });
    assert.deepEqual(pageErrors, []);
    assert.deepEqual(consoleErrors, []);
  } finally {
    await browser.close();
    await harnessServer.close();
  }
});
