import assert from "node:assert/strict";
import { once } from "node:events";
import { createServer } from "node:http";
import { after, test } from "node:test";

import { chromium } from "playwright";
import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";

const bundledHarness = await build({
  entryPoints: [
    new URL("../support/course_instance_assessment_refresh_harness.tsx", import.meta.url).pathname,
  ],
  bundle: true,
  format: "esm",
  platform: "browser",
  write: false,
  loader: { ".css": "empty" },
  plugins: [solidPlugin({ solid: { generate: "dom", hydratable: false } })],
});
const server = createServer((request, response) => {
  if (request.url === "/harness.js") {
    response.writeHead(200, { "content-type": "text/javascript; charset=utf-8" });
    response.end(bundledHarness.outputFiles[0].contents);
    return;
  }
  response.writeHead(200, { "content-type": "text/html; charset=utf-8" });
  response.end(`<!doctype html><html><body><script type="module">
    import { mountAssessmentRow } from "/harness.js";
    window.mountAssessmentRow = mountAssessmentRow;
  </script></body></html>`);
});
server.listen(0, "127.0.0.1");
await once(server, "listening");
const address = server.address();
if (address === null || typeof address === "string") {
  throw new Error("Assessment refresh evidence server did not bind TCP.");
}

const existingAssessment = {
  id: "A1",
  assessmentType: "regular_assignment",
  title: "Old title",
  dueAt: "2026-10-01T09:00:00",
  displayTimeZone: "America/Chicago",
  status: "unreleased",
  assessmentEditNumber: "4",
};
const refreshedAssessment = {
  ...existingAssessment,
  title: "Updated elsewhere",
  dueAt: "2026-10-02T10:30:00",
  status: "released",
  assessmentEditNumber: "5",
};

test("same-ID row refresh preserves typed draft and rebases only on explicit load", async () => {
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage();
    await page.goto(`http://127.0.0.1:${String(address.port)}/`);
    await page.waitForFunction(() => globalThis.mountAssessmentRow !== undefined);
    await page.evaluate(
      ({ initial, refreshed }) => globalThis.mountAssessmentRow(initial, refreshed),
      {
        initial: existingAssessment,
        refreshed: refreshedAssessment,
      },
    );

    await page.getByRole("button", { name: "Edit title and due date" }).click();
    const titleInput = page.getByRole("textbox", { name: "Title" });
    await titleInput.fill("My typed draft");
    const dueDateInput = page.getByRole("textbox", { name: "Due date" });
    const dueTimeInput = page.getByRole("textbox", { name: "Due time" });
    await dueDateInput.fill("2026-11-15");
    await dueTimeInput.fill("14:45");
    await page.evaluate(() => globalThis.courseInstanceAssessmentFixture.refresh());

    assert.equal(await page.locator("h3").innerText(), "Updated elsewhere");
    assert.match(
      await page.locator("p").filter({ hasText: "Released" }).first().innerText(),
      /^Released - Due:/,
    );
    assert.match(
      await page.locator("p").filter({ hasText: "Due:" }).first().innerText(),
      /Oct 2, 2026/,
    );
    assert.equal(await titleInput.inputValue(), "My typed draft");
    assert.equal(await dueDateInput.inputValue(), "2026-11-15");
    assert.equal(await dueTimeInput.inputValue(), "14:45");

    await page.getByRole("button", { name: "Save title and due date" }).click();
    await page.getByRole("button", { name: "Load current Assessment" }).click();
    await page.getByText("Latest Assessment loaded.").waitFor();
    assert.equal(await titleInput.inputValue(), "My typed draft");
    assert.equal(await dueDateInput.inputValue(), "2026-11-15");
    assert.equal(await dueTimeInput.inputValue(), "14:45");
    await page.getByRole("button", { name: "Save title and due date" }).click();

    assert.deepEqual(
      await page.evaluate(() => globalThis.courseInstanceAssessmentFixture.saveBaselines),
      ["4", "5"],
    );
  } finally {
    await browser.close();
  }
});

after(async () => {
  await new Promise((resolve, reject) =>
    server.close((error) => (error ? reject(error) : resolve())),
  );
});
