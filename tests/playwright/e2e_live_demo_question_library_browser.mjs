// Production-browser proof for the restored Instructor Question Library task.

import { chromium } from "playwright";

const port = process.argv[2];
if (!/^[0-9]+$/.test(port ?? "")) {
  throw new Error("expected the fixed HTTPS gateway port");
}

const origin = `https://localhost:${port}`;
const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ ignoreHTTPSErrors: true });
const page = await context.newPage();

try {
  // The production gateway keeps its own readiness probes active.  Wait for
  // the document and then the actual demo control rather than requiring every
  // background request in the live stack to go idle.
  await page.goto(`${origin}/sign-in`, { waitUntil: "domcontentloaded" });
  await page.getByRole("button", { name: "Continue as Elena Rivera" }).click();
  await page.waitForURL(`${origin}/library`);
  await page.getByRole("link", { name: "Question Library" }).waitFor();
  const search = page.getByLabel("Search published questions");
  await search.fill("rotation");
  await page.getByRole("heading", { name: "Peptide bond rotation" }).waitFor();
  await page.getByRole("link", { name: "Open question" }).click();
  await page.waitForURL(`${origin}/library/PNE-0001`);
  await page.getByRole("heading", { name: "Peptide bond rotation" }).waitFor();
  await page.getByRole("region", { name: "Question prompt" }).waitFor();
} finally {
  await context.close();
  await browser.close();
}
