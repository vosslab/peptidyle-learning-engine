// Direct seeded-role entry proves ordinary Sysadmin authorization and generic passkey sign-in.
// Selector contract: src/pages/sign_in_page.tsx owns the sign-in labels; account_security_page.tsx
// owns the account-security route surface, passkey form, visible success status, and passkey card.
import { expect, test } from "@playwright/test";

import { configuredLiveDemoInputs } from "../../../playwright.config";
import { installVirtualAuthenticator, removeVirtualAuthenticator } from "../helper_live_demo";
import {
  chooseSeededIdentity,
  observeContextOrigins,
  requireScenarioInput,
  restoreViewportOrigin,
  selectVisibleCourse,
  signOutVisible,
  writeOriginReceipt,
} from "./real_stack_ui";
import { captureRealStackScreenshot } from "./real_stack_screenshot_capture";

test("direct role entry: Morgan retains Sysadmin authorization after passkey sign-in", async ({
  browser,
}) => {
  test.setTimeout(180_000);
  const scenarioInput = requireScenarioInput(configuredLiveDemoInputs);
  expect(scenarioInput.scenarioId).toBe("direct_role_entry");
  expect(scenarioInput.namespace).toMatch(/^bs1-[0-9a-f]{12}-direct_role_entry$/u);
  const context = await browser.newContext({ ignoreHTTPSErrors: true });
  const pageOrigins = new Set<string>();
  const requestOrigins = new Set<string>();
  observeContextOrigins(context, pageOrigins, requestOrigins);
  const page = await context.newPage();
  const authenticator = await installVirtualAuthenticator(page);
  try {
    await chooseSeededIdentity(page, /Morgan/u);
    await selectVisibleCourse(page, "Genetics Practice Course");
    await expect(page.getByRole("link", { name: "Teaching operations" })).toBeVisible();
    await page.getByRole("link", { name: "Account", exact: true }).click();
    await expect(page.getByRole("heading", { name: "Your passkeys", exact: true })).toBeVisible();
    const accountSecurity = page
      .getByRole("main")
      .locator('[data-route-surface="accountSecurity"]');
    const passkeyLabel = "Morgan's security key";
    await accountSecurity.getByLabel("Passkey name").fill(passkeyLabel);
    await accountSecurity.getByRole("button", { name: "Add passkey" }).click();
    await expect(accountSecurity.getByRole("status")).toHaveText("Passkey added.");
    const passkeyCard = accountSecurity.locator(".passkey-card").filter({ hasText: passkeyLabel });
    await expect(passkeyCard).toBeVisible();
    await expect(
      passkeyCard.getByRole("heading", { name: passkeyLabel, exact: true }),
    ).toBeVisible();
    await restoreViewportOrigin(page);
    await captureRealStackScreenshot(page, scenarioInput, "sysadmin_account_security_passkey");
    await signOutVisible(page);
    await page.getByRole("button", { name: "Sign in with a passkey" }).click();
    await expect(page.getByRole("heading", { name: "Choose your course" })).toBeVisible();
    await selectVisibleCourse(page, "Genetics Practice Course");
    await page.getByRole("link", { name: "Teaching operations" }).click();
    await expect(page.getByRole("heading", { name: "Instructor approval" })).toBeVisible();
  } finally {
    await removeVirtualAuthenticator(authenticator);
    await context.close();
    writeOriginReceipt(pageOrigins, requestOrigins);
  }
});
