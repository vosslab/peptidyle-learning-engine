// Production-browser proof for visible Sysadmin Instructor Account management.

import { chromium } from "playwright";
import { liveDemoChromiumArgs } from "./helper_gateway_trust.mjs";
import { localDemoAuthenticationCode } from "./screenshot_corpus/local_demo_authenticator.ts";

const port = process.argv[2];
if (!/^[0-9]+$/.test(port ?? "")) {
  throw new Error("expected the fixed HTTPS gateway port");
}

const origin = `https://localhost:${port}`;
const browser = await chromium.launch({ headless: true, args: liveDemoChromiumArgs(origin) });
const context = await browser.newContext();
const page = await context.newPage();

try {
  await page.goto(`${origin}/sign-in`, { waitUntil: "domcontentloaded" });
  await page.getByRole("button", { name: "Assume the role of Sysadmin Morgan Delgado" }).click();
  const setupFile = process.env["PLE_LOCAL_DEMO_TOTP_SETUP_FILE"];
  if (setupFile === undefined) {
    throw new Error("Sysadmin browser acceptance requires the owned local authenticator path");
  }
  const code = await localDemoAuthenticationCode(setupFile);
  await page.getByLabel("Authentication code", { exact: true }).fill(code);
  await page
    .getByRole("button", { name: "Verify and open administrator tools", exact: true })
    .click();
  const sysadminHome = page.getByRole("heading", { name: "System administration", exact: true });
  const mfaFailure = page.getByRole("alert");
  await Promise.race([sysadminHome.waitFor(), mfaFailure.waitFor()]);
  if (await mfaFailure.isVisible()) {
    throw new Error("Morgan's ordinary MFA form did not reach administrator tools");
  }
  await page
    .getByRole("navigation", { name: "Ribbon tabs" })
    .getByRole("link", { name: "Instructor Accounts" })
    .click();
  await page.waitForURL(`${origin}/sysadmin/instructor-accounts`);
  await page.locator('[data-route-surface="instructorAccounts"]').waitFor();
  await page.locator("#instructor-accounts-heading").waitFor();

  await page.locator("#instructor-account-email").fill(`m16-browser-${Date.now()}@example.invalid`);
  await page.getByLabel("First name", { exact: true }).fill("M16");
  await page.getByLabel("Last name", { exact: true }).fill("Browser Instructor");
  await page.getByLabel("Affiliation", { exact: true }).fill("PLE test");
  await page.getByRole("button", { name: "Create Instructor Account" }).click();
  await page
    .getByText("Instructor Account created, but setup email could not be sent.", { exact: true })
    .waitFor();
  const created = page
    .getByRole("list", { name: "Instructor Accounts", exact: true })
    .getByRole("listitem")
    .first();
  const accountId = (await created.getByRole("heading", { level: 3 }).textContent())?.trim();
  if (!/^U[0-9A-HJKMNP-TV-Z]{8}$/u.test(accountId ?? "")) {
    throw new Error("created Instructor Account did not have a canonical public ID");
  }
  await created.locator(`#deactivate-reason-${accountId}`).fill("Live demo access review");
  await created.getByRole("button", { name: "Deactivate Instructor Account" }).click();
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Deactivate Instructor Account" })
    .click();
  await created.getByText("Deactivated", { exact: true }).waitFor();
  await created.getByRole("button", { name: "Reactivate Instructor Account" }).click();
  await created.getByText("Active", { exact: true }).waitFor();
} finally {
  await context.close();
  await browser.close();
}
