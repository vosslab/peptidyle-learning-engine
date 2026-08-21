// Visible, generation-bound Sysadmin transition used by the suite owner and focused A1 gate.

import { expect, test } from "@playwright/test";
import { writeFileSync } from "node:fs";

import { configuredLiveDemoInputs } from "../../../playwright.config";
import { installVirtualAuthenticator } from "../helper_live_demo";
import { liveDemoOriginReceiptPathFromEnvironment } from "../live_demo_live_config";
import {
  courseChoice,
  observeContextOrigins,
  requireScenarioInput,
  selectVisibleCourse,
  signOutVisible,
} from "./real_stack_ui";

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
  observeContextOrigins(context, pageOrigins, requestOrigins);
  try {
    const page = await context.newPage();
    await installVirtualAuthenticator(page);
    await page.goto("/live-demo/sysadmin-setup");
    await expect(page.getByRole("heading", { name: "Set up administrator access" })).toBeVisible();
    await page.getByLabel("Administrator setup code").fill(requiredOwnershipProof());
    await page.getByLabel("Passkey name").fill(`Morgan passkey ${scenarioInput.namespace}`);
    await page.getByRole("button", { name: "Set up administrator passkey" }).click();
    await expect(page.getByRole("heading", { name: "Choose your course" })).toBeVisible();
    await expect(courseChoice(page, "Genetics Practice Course")).toHaveCount(1);
    await expect(courseChoice(page, "Biochemistry Base Course")).toHaveCount(0);

    const completedSetup = await context.newPage();
    await completedSetup.goto("/live-demo/sysadmin-setup");
    await expect(
      completedSetup.getByText("Administrator setup is already complete."),
    ).toBeVisible();
    await completedSetup.close();

    await selectVisibleCourse(page, "Genetics Practice Course");
    await signOutVisible(page);
    await page.getByRole("button", { name: "Sign in with a passkey" }).click();
    await expect(page.getByRole("heading", { name: "Choose your course" })).toBeVisible();
    await expect(courseChoice(page, "Genetics Practice Course")).toHaveCount(1);
  } finally {
    try {
      await context.close();
    } finally {
      writeOriginReceipt(pageOrigins, requestOrigins);
    }
  }
});
