// Visible, generation-bound Sysadmin transition used by the suite owner and focused A1 gate.

import { expect, test } from "@playwright/test";
import { writeFileSync } from "node:fs";

import { configuredLiveDemoInputs } from "../../../playwright.config";
import { webAuthnContinuationPathForProducerFromEnvironment } from "../browser_suite_live_config";
import {
  exportWebAuthnContinuation,
  importWebAuthnContinuation,
  installVirtualAuthenticator,
  removeVirtualAuthenticator,
  type VirtualAuthenticator,
  writeWebAuthnContinuation,
} from "../helper_live_demo";
import { liveDemoOriginReceiptPathFromEnvironment } from "../browser_suite_live_config";
import {
  courseChoice,
  observeContextOrigins,
  requireScenarioInput,
  restoreViewportOrigin,
  selectVisibleCourse,
  signOutVisible,
} from "./real_stack_ui";
import { captureRealStackScreenshot } from "./real_stack_screenshot_capture";

function writeOriginReceipt(pageOrigins: Set<string>, requestOrigins: Set<string>): void {
  writeFileSync(
    liveDemoOriginReceiptPathFromEnvironment(process.env),
    JSON.stringify({
      pageOrigins: [...pageOrigins].sort(),
      requestOrigins: [...requestOrigins].sort(),
    }),
    { encoding: "ascii", flag: "wx", mode: 0o600 },
  );
}

function requiredOwnershipProof(): string {
  const input = requireScenarioInput(configuredLiveDemoInputs);
  if (input.sysadminRequirement !== "unclaimed" || input.sysadminOwnershipProof === undefined) {
    throw new Error("the first-claim scenario requires the owner-issued proof");
  }
  return input.sysadminOwnershipProof;
}

test("sysadmin first claim: visible setup completes and passkey reauthenticates", async ({
  browser,
}) => {
  test.setTimeout(180_000);
  const scenarioInput = requireScenarioInput(configuredLiveDemoInputs);
  expect(scenarioInput.scenarioId).toBe("sysadmin_first_claim");
  const context = await browser.newContext({ ignoreHTTPSErrors: true });
  const pageOrigins = new Set<string>();
  const requestOrigins = new Set<string>();
  let authenticator: VirtualAuthenticator | undefined;
  observeContextOrigins(context, pageOrigins, requestOrigins);
  try {
    const page = await context.newPage();
    const installedAuthenticator =
      await test.step("install the browser virtual authenticator", async () =>
        installVirtualAuthenticator(page));
    authenticator = installedAuthenticator;
    await test.step("claim the seeded administrator through the setup form", async () => {
      await page.goto("/live-demo/sysadmin-setup");
      await expect(
        page.getByRole("heading", { name: "Set up administrator access" }),
      ).toBeVisible();
      await page.getByLabel("Administrator setup code").fill(requiredOwnershipProof());
      await page.getByLabel("Passkey name").fill(`Morgan passkey ${scenarioInput.namespace}`);
      await page.getByRole("button", { name: "Set up administrator passkey" }).click();
      await expect(page.getByRole("heading", { name: "Choose your course" })).toBeVisible();
      await expect(courseChoice(page, "Genetics Practice Course")).toHaveCount(1);
      await expect(courseChoice(page, "Biochemistry Base Course")).toHaveCount(0);
    });
    await test.step("observe that a second setup page recognizes the completed claim", async () => {
      const completedSetupContext = await browser.newContext({ ignoreHTTPSErrors: true });
      observeContextOrigins(completedSetupContext, pageOrigins, requestOrigins);
      const completedSetup = await completedSetupContext.newPage();
      try {
        await completedSetup.goto("/live-demo/sysadmin-setup");
        await expect(
          completedSetup.getByText("Administrator setup is already complete."),
        ).toBeVisible();
      } finally {
        await completedSetupContext.close();
      }
    });
    await test.step("sign out and reenter with the visible passkey button", async () => {
      await selectVisibleCourse(page, "Genetics Practice Course");
      await signOutVisible(page);
      await page.getByRole("button", { name: "Sign in with a passkey" }).click();
      await expect(page.getByRole("heading", { name: "Choose your course" })).toBeVisible();
      await expect(courseChoice(page, "Genetics Practice Course")).toHaveCount(1);
      await selectVisibleCourse(page, "Genetics Practice Course");
    });
    await test.step("observe the persisted passkey on the account page", async () => {
      await page.getByRole("link", { name: "Account", exact: true }).click();
      await expect(page.getByRole("heading", { name: "Your passkeys", exact: true })).toBeVisible();
      await expect(
        page.getByText(`Morgan passkey ${scenarioInput.namespace}`, { exact: true }),
      ).toBeVisible();
      await restoreViewportOrigin(page);
      await captureRealStackScreenshot(page, scenarioInput, "sysadmin_account_security_passkey");
    });
    await test.step("export the verified authenticator for the claimed child", async () => {
      const continuation = await exportWebAuthnContinuation(
        installedAuthenticator,
        new URL(scenarioInput.baseUrl).origin,
        "localhost",
      );
      writeWebAuthnContinuation(
        webAuthnContinuationPathForProducerFromEnvironment(process.env),
        continuation,
      );
      const importedContext = await browser.newContext({ ignoreHTTPSErrors: true });
      try {
        await importWebAuthnContinuation(await importedContext.newPage(), continuation);
      } finally {
        await importedContext.close();
      }
    });
  } finally {
    try {
      const activeAuthenticator = authenticator;
      if (activeAuthenticator !== undefined) {
        await test.step("remove the virtual authenticator before closing the browser context", () =>
          removeVirtualAuthenticator(activeAuthenticator));
      }
    } finally {
      try {
        await context.close();
      } finally {
        writeOriginReceipt(pageOrigins, requestOrigins);
      }
    }
  }
});
