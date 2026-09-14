// Real browser capture for one opaque WeBWorK Assignment Attempt.
//
// This deliberately manipulates the renderer's document as a browser does.
// It neither knows nor classifies WeBWorK controls; the enclosing PLE surface
// captures the renderer-owned form response through its public bridge.

import { chromium } from "playwright";

const [port, attempt] = process.argv.slice(2);
if (!/^[0-9]+$/u.test(port ?? "") || !/^R-[1-9][0-9]{0,9}$/u.test(attempt ?? "")) {
  throw new Error("expected a gateway port and an Assignment Attempt reference");
}

const origin = `https://localhost:${port}`;
const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ ignoreHTTPSErrors: true });
const page = await context.newPage();

async function makeOneOrdinaryEdit(frame) {
  const editable = frame
    .locator(
      "input:not([type=hidden]):not([type=submit]):not([type=button]):not([type=reset]), select, textarea",
    )
    .first();
  await editable.waitFor();
  const tag = await editable.evaluate((element) => element.tagName.toLowerCase());
  const type = await editable.getAttribute("type");
  if (tag === "select") {
    await editable.selectOption({ index: 0 });
  } else if (type === "checkbox" || type === "radio") {
    await editable.check();
  } else {
    await editable.fill("1");
  }
}

try {
  await page.goto(`${origin}/sign-in`, { waitUntil: "domcontentloaded" });
  await page.getByRole("button", { name: "Assume the role of Student Mary Okafor" }).click();
  await page.waitForURL(`${origin}/`);
  await page.goto(`${origin}/assignment-attempts/${attempt}`, { waitUntil: "domcontentloaded" });
  await page.locator('[data-route-surface="assignmentAttempt"]').waitFor();
  const frame = page.frameLocator('iframe[title="Question document"]');
  await makeOneOrdinaryEdit(frame);
  await page.getByRole("button", { name: "Save response", exact: true }).click();
  await page.getByText("Response saved.", { exact: true }).waitFor();

  const persisted = page.waitForResponse((response) =>
    response.url().includes(`/api/assignment-attempts/${attempt}/student-question?position=1`),
  );
  await page.reload({ waitUntil: "domcontentloaded" });
  const response = await persisted;
  const body = await response.json();
  if (!response.ok() || body?.savedResponse?.kind !== "backendOwned") {
    throw new Error("WeBWorK saved response was not retained after browser reload");
  }
  console.log("WeBWorK browser capture and reload: PASS");
} finally {
  await context.close();
  await browser.close();
}
