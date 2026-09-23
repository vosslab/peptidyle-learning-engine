// Shared HTTP/browser setup for the synthetic Student current-Course fixture.

import { chromium } from "playwright";

import { bundleStudentCourseEntryM6Harness } from "../support/student_course_entry_m6_loader.ts";
import { startHarnessServer } from "./ribbon_harness_server.mjs";

export async function openStudentCourseEntryHarness({ mode, headless = true }) {
  const bundle = await bundleStudentCourseEntryM6Harness();
  const additionalAssets = new Map([
    [
      "/student_course_entry_m6_harness.js",
      { body: bundle.javascript, contentType: "text/javascript; charset=utf-8" },
    ],
  ]);
  const markup = `<!doctype html><html><head><style>${bundle.stylesheet}</style></head>
    <body><div id="root"></div><script type="module">
      import { mountStudentCourseEntryM6Harness } from "/student_course_entry_m6_harness.js";
      window.studentCourseEntryM6 = mountStudentCourseEntryM6Harness(
        document.querySelector("#root"), ${JSON.stringify(mode)});
    </script></body></html>`;
  const harnessServer = await startHarnessServer(markup, bundle.stylesheet, additionalAssets);
  const browser = await chromium.launch({ headless });
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  page.setDefaultTimeout(5_000);
  const pageErrors = [];
  const consoleErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") consoleErrors.push(message.text());
  });
  await page.goto(harnessServer.evidenceUrl);
  await page
    .getByRole("heading", { name: "Biochemistry 301: Proteins and Peptides", exact: true })
    .waitFor({ state: "visible" });
  await page.getByRole("list", { name: "Coursework", exact: true }).waitFor({
    state: "visible",
  });
  return { browser, consoleErrors, harnessServer, page, pageErrors };
}
