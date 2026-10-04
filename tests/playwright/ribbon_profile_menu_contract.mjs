// Profile menu role, keyboard, focus, dismissal, and action behavior.

import assert from "node:assert/strict";
import { chromium } from "playwright";

import { bundleRibbonResponsiveHarness } from "../support/ribbon_responsive_loader.ts";

const bundle = await bundleRibbonResponsiveHarness();
const bundleUrl = `data:text/javascript;base64,${Buffer.from(bundle.javascript).toString("base64")}`;
const markup = [
  "<!doctype html><html><head>",
  `<style>${bundle.stylesheet}</style>`,
  '</head><body><div id="root"></div><button id="outside">Outside</button>',
  '<script type="module">',
  `import { mountRibbonResponsiveHarness } from "${bundleUrl}";`,
  'window.ribbonResponsive = mountRibbonResponsiveHarness(document.querySelector("#root"));',
  "</script></body></html>",
].join("");

async function flush(page) {
  await page.evaluate(() => new Promise((resolve) => queueMicrotask(resolve)));
  await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(resolve)));
}

const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  await page.setContent(markup);
  await page.waitForFunction(() => "ribbonResponsive" in window);
  await page.evaluate(() => {
    window.profileMenuSignOutActions = 0;
    document.addEventListener("ple-ribbon-action", (event) => {
      if (event instanceof CustomEvent && event.detail?.id === "signOut") {
        window.profileMenuSignOutActions += 1;
      }
    });
  });
  await flush(page);

  const profile = page.getByRole("button", { name: "Profile", exact: true });
  const menu = page.getByRole("menu", { name: "Profile menu", exact: true });
  const profileLink = page.getByRole("menuitem", { name: "Profile settings", exact: true });
  const accountSettings = page.getByRole("menuitem", { name: "Account settings", exact: true });
  const signOut = page.getByRole("menuitem", { name: "Sign Out", exact: true });

  assert.equal(await page.getByRole("button", { name: "Sign Out", exact: true }).count(), 0);
  for (const role of ["student", "instructor", "sysadmin"]) {
    await page.evaluate((userRole) => window.ribbonResponsive.setRoleHome(userRole), role);
    await flush(page);
    await profile.click();
    await menu.waitFor({ state: "visible" });
    assert.equal(await profileLink.count(), 1, `${role} has one Profile command`);
    assert.equal(await accountSettings.count(), 0, `${role} has no second time-zone page`);
    assert.equal(await signOut.count(), 1, `${role} has one Sign Out command`);
    assert.equal(
      await menu.getByRole("menuitem").count(),
      2,
      `${role} Profile menu contains Profile settings and Sign Out`,
    );
    assert.equal(
      await page
        .locator(".ple-app-ribbon__tabs, .ple-app-ribbon__tasks")
        .getByText("Sign Out", { exact: true })
        .count(),
      0,
      `${role} does not scatter Sign Out into Ribbon navigation`,
    );
    assert.equal(await profileLink.getAttribute("href"), "/profile", role);
    assert.equal(await page.locator('a[href="/account-settings"]').count(), 0);
    await page.keyboard.press("Escape");
  }

  await page.evaluate(() => window.ribbonResponsive.setRoleHome("instructor"));
  await flush(page);
  await profile.focus();
  await page.keyboard.press("ArrowDown");
  await menu.waitFor({ state: "visible" });
  assert.equal(await profileLink.evaluate((element) => document.activeElement === element), true);
  await page.keyboard.press("ArrowDown");
  assert.equal(await signOut.evaluate((element) => document.activeElement === element), true);
  await page.keyboard.press("ArrowUp");
  assert.equal(await profileLink.evaluate((element) => document.activeElement === element), true);
  await page.keyboard.press("Escape");
  assert.equal(await menu.count(), 0);
  assert.equal(await profile.evaluate((element) => document.activeElement === element), true);

  await profile.click();
  await page.locator("#outside").click();
  assert.equal(await menu.count(), 0);

  await profile.click();
  await signOut.click();
  assert.equal(await page.evaluate(() => window.profileMenuSignOutActions), 1);
  assert.equal(await menu.count(), 0);

  process.stdout.write("Ribbon Profile menu behavior: PASS\n");
  process.stdout.write("Account actions stay together in the Profile menu: PASS\n");
} finally {
  await browser.close();
}
