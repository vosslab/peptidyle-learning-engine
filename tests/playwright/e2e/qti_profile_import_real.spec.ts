// Elena imports the tracked Canvas QTI fixture through the visible production PLE interface.
import { expect, test, type BrowserContext, type Page } from "@playwright/test";
import { writeFileSync } from "node:fs";

import { configuredLiveDemoInputs } from "../../../playwright.config";
import { liveDemoOriginReceiptPathFromEnvironment } from "../browser_suite_live_config";
import {
  chooseSeededIdentity,
  observeContextOrigins,
  requireScenarioInput,
  selectVisibleCourse,
} from "./real_stack_ui";
import { canvasQtiFixtureArchive } from "./qti_fixture_archive";

const actionTimeoutMs = 30_000;
const qtiReadyTimeoutMs = 180_000;

interface ObservedOrigins {
  readonly pageOrigins: Set<string>;
  readonly requestOrigins: Set<string>;
}

function configure(context: BrowserContext, page: Page): void {
  context.setDefaultTimeout(actionTimeoutMs);
  context.setDefaultNavigationTimeout(actionTimeoutMs);
  page.setDefaultTimeout(actionTimeoutMs);
  page.setDefaultNavigationTimeout(actionTimeoutMs);
}

function writeOriginReceipt(contexts: Readonly<Record<string, ObservedOrigins>>): void {
  const pageOrigins = new Set<string>();
  const requestOrigins = new Set<string>();
  for (const origins of Object.values(contexts)) {
    for (const origin of origins.pageOrigins) pageOrigins.add(origin);
    for (const origin of origins.requestOrigins) requestOrigins.add(origin);
  }
  writeFileSync(
    liveDemoOriginReceiptPathFromEnvironment(process.env),
    JSON.stringify({
      pageOrigins: [...pageOrigins].sort(),
      requestOrigins: [...requestOrigins].sort(),
      contexts: Object.fromEntries(
        Object.entries(contexts).map(([name, origins]) => [
          name,
          {
            pageOrigins: [...origins.pageOrigins].sort(),
            requestOrigins: [...origins.requestOrigins].sort(),
          },
        ]),
      ),
    }),
    { encoding: "ascii", flag: "wx", mode: 0o600 },
  );
}

function expectScenarioOrigin(origins: ObservedOrigins, expectedOrigin: string): void {
  expect([...origins.pageOrigins].sort()).toEqual([expectedOrigin]);
  expect([...origins.requestOrigins].sort()).toEqual([expectedOrigin]);
}

async function createPrivateWorkspace(page: Page, namespace: string): Promise<void> {
  await page.getByRole("link", { name: "Workspace", exact: true }).click();
  await page.getByRole("button", { name: "Create flat question", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Flat question", exact: true })).toBeVisible();
  await page.getByLabel("Question title").fill(`QTI source draft ${namespace}`);
  await page.getByRole("button", { name: "Save private draft", exact: true }).click();
  await expect(page.getByRole("status", { name: "Private draft status" })).toHaveText(
    "Private draft saved. It is not published.",
  );
}

async function importCanvasFixture(page: Page, namespace: string): Promise<void> {
  const archiveName = `canvas-qti-${namespace}.zip`;
  const panel = page.locator('[data-route-surface="qtiProfileImport"]');
  await page.getByLabel("QTI ZIP archive").setInputFiles({
    name: archiveName,
    mimeType: "application/zip",
    buffer: canvasQtiFixtureArchive(),
  });
  await page.getByRole("button", { name: "Start import", exact: true }).click();
  await expect(panel.getByRole("status")).toContainText("queued for server-side review");

  const report = page.getByRole("heading", { name: "QTI import report", exact: true });
  await expect
    .poll(
      async () => {
        const refresh = page.getByRole("button", { name: "Refresh status", exact: true });
        await refresh.click();
        return await report.isVisible();
      },
      { timeout: qtiReadyTimeoutMs },
    )
    .toBe(true);
  await expect(
    page.getByText("Canvas QTI 1.2 static single choice", { exact: true }),
  ).toBeVisible();
  await expect(page.getByRole("heading", { name: "Accepted: Favorite color" })).toBeVisible();

  await page.getByRole("radio", { name: "Select this item for conversion" }).check();
  await page.getByRole("checkbox", { name: /I reviewed the profile/u }).check();
  await page.getByRole("button", { name: "Convert selected item", exact: true }).click();
}

async function expectConvertedDraft(page: Page): Promise<void> {
  const oldEditor = page.locator('[data-route-surface="flatQuestionEditor"]');
  await expect(oldEditor).toHaveAttribute("aria-busy", "true");
  await expect(oldEditor).toHaveAttribute("inert", "");
  await expect(page.getByLabel("Question title")).toBeDisabled();
  await expect(page.getByRole("heading", { name: "Flat question", exact: true })).toBeFocused();
  await expect(page.getByLabel("Question title")).toHaveValue("Favorite color");
  await expect(oldEditor).not.toHaveAttribute("inert", "");
  await expect(page.getByLabel("Question title")).toBeEditable();
}

test.describe("QTI profile import on the production PLE stack", () => {
  test.skip(
    configuredLiveDemoInputs === undefined,
    "the disposable production browser-suite owner supplies this scenario input",
  );

  test("Elena imports a Canvas QTI archive and a fresh session observes the converted draft", async ({
    browser,
  }) => {
    test.setTimeout(300_000);
    const scenarioInput = requireScenarioInput(configuredLiveDemoInputs);
    expect(scenarioInput.scenarioId).toBe("qti_profile_import");
    expect(scenarioInput.namespace).toMatch(/^bs1-[0-9a-f]{12}-qti_profile_import$/u);
    expect(scenarioInput.sysadminRequirement).toBe("not_required");
    const expectedOrigin = new URL(scenarioInput.baseUrl).origin;
    const origins = {
      initial: { pageOrigins: new Set<string>(), requestOrigins: new Set<string>() },
      fresh: { pageOrigins: new Set<string>(), requestOrigins: new Set<string>() },
    };
    const contexts: BrowserContext[] = [];
    let originEvidenceVerified = false;

    try {
      const elenaContext = await browser.newContext({
        viewport: { width: 1280, height: 800 },
        ignoreHTTPSErrors: true,
      });
      contexts.push(elenaContext);
      observeContextOrigins(
        elenaContext,
        origins.initial.pageOrigins,
        origins.initial.requestOrigins,
      );
      const elena = await elenaContext.newPage();
      configure(elenaContext, elena);

      await chooseSeededIdentity(elena, /Elena Rivera/u);
      await selectVisibleCourse(elena, "Biochemistry Base Course");
      await createPrivateWorkspace(elena, scenarioInput.namespace);
      await importCanvasFixture(elena, scenarioInput.namespace);
      await expectConvertedDraft(elena);

      const freshElenaContext = await browser.newContext({
        viewport: { width: 1280, height: 800 },
        ignoreHTTPSErrors: true,
      });
      contexts.push(freshElenaContext);
      observeContextOrigins(
        freshElenaContext,
        origins.fresh.pageOrigins,
        origins.fresh.requestOrigins,
      );
      await expect(freshElenaContext.storageState()).resolves.toEqual({ cookies: [], origins: [] });
      const freshElena = await freshElenaContext.newPage();
      configure(freshElenaContext, freshElena);
      await chooseSeededIdentity(freshElena, /Elena Rivera/u);
      await selectVisibleCourse(freshElena, "Biochemistry Base Course");
      await freshElena.getByRole("link", { name: "Workspace", exact: true }).click();
      await expect(freshElena.getByText("Favorite color", { exact: true }).first()).toBeVisible();
      await freshElena.getByRole("button", { name: /^Favorite color\b/u }).click();
      await expect(freshElena.getByLabel("Question title")).toHaveValue("Favorite color");

      expectScenarioOrigin(origins.initial, expectedOrigin);
      expectScenarioOrigin(origins.fresh, expectedOrigin);
      originEvidenceVerified = true;
    } finally {
      try {
        await Promise.all(contexts.map((context) => context.close()));
      } finally {
        if (originEvidenceVerified) writeOriginReceipt(origins);
      }
    }
  });
});
