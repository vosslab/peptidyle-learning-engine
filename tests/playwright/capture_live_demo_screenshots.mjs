// Capture the current production-bundle Live Demo through its fixed HTTPS owner.

import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";
import { mkdir } from "node:fs/promises";

import { chromium } from "playwright";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const repositoryRoot = path.resolve(scriptDirectory, "../..");
const outputDirectory = path.join(repositoryRoot, "docs/screenshots/live_demo");

function requireEntryUrl(argument) {
  if (argument === undefined) {
    throw new Error("the Live Demo launcher did not report its ready entry URL");
  }
  const url = new URL(argument);
  if (url.protocol !== "https:" || url.hostname !== "localhost" || url.pathname !== "/sign-in") {
    throw new Error("the Live Demo entry must be its local HTTPS sign-in URL");
  }
  return url;
}

function screenshotPath(filename) {
  return path.join(outputDirectory, filename);
}

async function capture(page, filename, locator) {
  const target = screenshotPath(filename);
  const options = { animations: "disabled", caret: "hide", path: target };
  if (locator === undefined) {
    await page.screenshot(options);
  } else {
    await locator.screenshot(options);
  }
  console.log(`Captured ${path.relative(repositoryRoot, target)}`);
}

async function openAccountSelection(browser, entryUrl, viewport, mobile = false) {
  const context = await browser.newContext({
    colorScheme: "light",
    deviceScaleFactor: 1,
    hasTouch: mobile,
    ignoreHTTPSErrors: true,
    isMobile: mobile,
    reducedMotion: "reduce",
    viewport,
  });
  const page = await context.newPage();
  const pageErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error));
  page.setDefaultTimeout(30_000);
  page.setDefaultNavigationTimeout(60_000);
  await page.goto(entryUrl.href, { waitUntil: "domcontentloaded" });
  await page
    .getByRole("heading", { level: 1, name: "Explore Peptidyle Learning Engine", exact: true })
    .waitFor();
  await page.getByRole("button", { name: /Continue as Elena Instructor/u }).waitFor();
  return { context, page, pageErrors };
}

async function enterInstructorShowcase(page, entryUrl) {
  await page.getByRole("button", { name: /Continue as Elena Instructor/u }).click();
  await page.waitForURL(new URL("live-demo/ribbon", entryUrl.origin).href);
  await page
    .getByRole("heading", {
      level: 1,
      name: "Ribbon implementation and invitation email trial",
      exact: true,
    })
    .waitFor();
  await page
    .getByRole("heading", { level: 2, name: "Populated Instructor Ribbon", exact: true })
    .waitFor();
  await page.locator('[data-ribbon-control="assignments"]').waitFor();
  await page.evaluate(async () => document.fonts.ready);
}

function requireNoPageErrors(pageErrors) {
  if (pageErrors.length > 0) {
    throw new AggregateError(pageErrors, "the Live Demo raised browser page errors during capture");
  }
}

async function captureDesktop(browser, entryUrl) {
  const { context, page, pageErrors } = await openAccountSelection(browser, entryUrl, {
    width: 1280,
    height: 800,
  });
  try {
    await capture(page, "01_account_selection.png");
    await enterInstructorShowcase(page, entryUrl);
    await capture(page, "02_instructor_ribbon.png");

    await page.getByRole("link", { name: "Gradebook", exact: true }).click();
    await page
      .getByRole("status")
      .filter({ hasText: "Gradebook selected in the structural preview." })
      .waitFor();
    await capture(page, "03_gradebook_selected.png");

    const mailer = page.getByRole("region", {
      name: "Try the attended invitation email hack",
      exact: true,
    });
    await mailer.scrollIntoViewIfNeeded();
    await capture(page, "04_invitation_email_hack.png", mailer);
    requireNoPageErrors(pageErrors);
  } finally {
    await context.close();
  }
}

async function captureResponsive(
  browser,
  entryUrl,
  filename,
  viewport,
  mobile,
  ribbonOnly = false,
) {
  const { context, page, pageErrors } = await openAccountSelection(
    browser,
    entryUrl,
    viewport,
    mobile,
  );
  try {
    await enterInstructorShowcase(page, entryUrl);
    const ribbon = page.locator(".live-demo-ribbon-preview");
    await ribbon.scrollIntoViewIfNeeded();
    await capture(page, filename, ribbonOnly ? ribbon : undefined);
    requireNoPageErrors(pageErrors);
  } finally {
    await context.close();
  }
}

const entryUrl = requireEntryUrl(process.argv[2]);
await mkdir(outputDirectory, { recursive: true });

const browser = await chromium.launch();
try {
  await captureDesktop(browser, entryUrl);
  await captureResponsive(
    browser,
    entryUrl,
    "05_instructor_ribbon_tablet.png",
    { width: 768, height: 1024 },
    true,
  );
  await captureResponsive(
    browser,
    entryUrl,
    "06_instructor_ribbon_phone.png",
    { width: 320, height: 640 },
    true,
    true,
  );
} finally {
  await browser.close();
}

console.log(`Screenshot capture complete: ${path.relative(repositoryRoot, outputDirectory)}`);
