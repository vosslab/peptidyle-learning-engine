// Account-zone regression: format the server instant in the Account display timezone.

import assert from "node:assert/strict";

import { chromium } from "playwright";

import { bundleAssessmentsDueSoonHarness } from "../support/assessments_due_soon_harness_loader.ts";
import { startHarnessServer } from "./ribbon_harness_server.mjs";

const bundle = await bundleAssessmentsDueSoonHarness();
const harnessServer = await startHarnessServer(
  `<!doctype html><html><head><style>${bundle.stylesheet}</style></head><body><div id="root"></div></body></html>`,
  bundle.stylesheet,
);
const browser = await chromium.launch({ headless: true });
const page = await browser.newPage();

try {
  await page.goto(harnessServer.evidenceUrl);
  await page.addScriptTag({ content: Buffer.from(bundle.javascript).toString("utf8") });
  await page.evaluate(() => {
    const target = document.querySelector("#root");
    if (!(target instanceof HTMLElement)) throw new Error("Due Soon harness root is missing.");
    window.PleAssessmentsDueSoonHarness.mountAssessmentsDueSoonHarness(target, {
      items: [
        {
          courseInstanceId: "CI8H4N6PAW",
          courseLongName: "Molecular Biology",
          assessmentId: "A9J5V7WA3",
          assessmentType: "quiz",
          assessmentTitle: "DNA repair",
          assessmentStatus: "released",
          dueAtMillis: 1790971200125,
        },
      ],
      nextCursor: null,
      displayTimeZone: "America/Chicago",
    });
  });

  const due = page.locator('time[datetime="2026-10-02T20:00:00.125Z"]');
  await due.waitFor({ state: "visible" });
  assert.equal(
    await due.innerText(),
    "Oct 2, 2026, 3:00 PM",
    "the Due instant uses the Account display timezone",
  );
  process.stdout.write("Due Soon Account-zone regression passed.\n");
} finally {
  await browser.close();
  await harnessServer.close();
}
