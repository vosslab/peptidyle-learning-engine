// ui_walkthrough_smoke.spec.ts - live-only public gateway readiness evidence.

import { expect, test } from "@playwright/test";

import { configuredUiWalkthroughInputs } from "../../playwright.config";

test.describe.configure({ mode: "serial" });

test.skip(
  configuredUiWalkthroughInputs === undefined,
  "requires the explicit UI walkthrough live-stack invocation",
);

test("public gateway health is ready at the configured live origin", async ({ page, baseURL }) => {
  const inputs = configuredUiWalkthroughInputs;
  if (inputs === undefined || baseURL === undefined) {
    throw new Error("the declaration-time live walkthrough skip did not apply");
  }
  expect(new URL(baseURL).origin).toBe(new URL(inputs.baseUrl).origin);

  const response = await page.goto("/health");
  expect(response).not.toBeNull();
  expect(response?.status()).toBe(200);
  expect(new URL(page.url()).origin).toBe(new URL(inputs.baseUrl).origin);
  await expect(page.locator("body")).toHaveText('{"status":"ready"}');
  expect(await page.locator("body").textContent()).toBe('{"status":"ready"}');
});
