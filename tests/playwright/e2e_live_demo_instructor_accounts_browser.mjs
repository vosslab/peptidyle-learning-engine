// Production-browser proof for visible Sysadmin Instructor Account management.

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
  await page.goto(`${origin}/sign-in`, { waitUntil: "domcontentloaded" });
  await page.getByRole("button", { name: "Continue as Morgan Delgado" }).click();
  await page.waitForURL(`${origin}/`);
  await page
    .getByRole("navigation", { name: "Ribbon tabs" })
    .getByRole("link", { name: "Instructor Accounts" })
    .click();
  await page.waitForURL(`${origin}/sysadmin/instructor-accounts`);
  await page.locator('[data-route-surface="instructorAccounts"]').waitFor();
  await page.locator("#instructor-accounts-heading").waitFor();

  await page.locator("#instructor-account-email").fill(`m16-browser-${Date.now()}@example.invalid`);
  await page.getByRole("button", { name: "Create Instructor Account" }).click();
  await page.getByText("Instructor Account created.").waitFor();
  const created = page.locator('section[aria-label="Instructor Accounts"] > .auth-panel').first();
  const reference = await created.locator("h2").textContent();
  if (!/^U-[1-9][0-9]{0,9}$/u.test(reference ?? "")) {
    throw new Error("created Instructor Account did not have a canonical public reference");
  }
  await created.locator(`#deactivate-reason-${reference}`).fill("Live demo access review");
  await created.getByRole("button", { name: "Deactivate Instructor Account" }).click();
  await created.getByText("State: Deactivated").waitFor();
  await created.getByRole("button", { name: "Reactivate Instructor Account" }).click();
  await created.getByText("State: Active").waitFor();
} finally {
  await context.close();
  await browser.close();
}
