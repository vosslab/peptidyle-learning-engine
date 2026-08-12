// ui_walkthrough_smoke.spec.ts - live-only public gateway readiness evidence.

import { expect, test } from "./ui_walkthrough_fixture";

test.describe.configure({ mode: "serial" });

test("public gateway health is ready at the configured live origin", async ({
  page,
  baseURL,
  uiWalkthroughInputs,
}) => {
  test.skip(uiWalkthroughInputs === undefined, "requires the explicit UI walkthrough config");
  if (uiWalkthroughInputs === undefined || baseURL === undefined) return;
  const inputs = uiWalkthroughInputs;
  expect(new URL(baseURL).origin).toBe(new URL(inputs.baseUrl).origin);

  const response = await page.goto("/health");
  expect(response).not.toBeNull();
  expect(response?.status()).toBe(200);
  expect(new URL(page.url()).origin).toBe(new URL(inputs.baseUrl).origin);
  await expect(page.locator("body")).toHaveText('{"status":"ready"}');
  expect(await page.locator("body").textContent()).toBe('{"status":"ready"}');
});
