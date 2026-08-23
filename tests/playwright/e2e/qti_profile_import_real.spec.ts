// Elena imports the tracked Canvas QTI fixture through the visible production PLE interface.
//
// Selector contract:
// - src/features/flat_question_authoring/flat_question_editor_page.tsx:535 owns the seed draft
//   editor and question title field.
// - src/features/qti_profile_import/qti_profile_import_page.tsx:381 owns the QTI archive input,
//   import actions, report heading, item selection, and conversion controls.
// - src/features/flat_question_authoring/flat_question_editor_page.tsx:535 owns the converted
//   question editor surface and disabled/editable title behavior.
import { expect, test, type BrowserContext, type Page } from "@playwright/test";

import { configuredLiveDemoInputs } from "../../../playwright.config";
import {
  chooseSeededIdentity,
  configureContextAndPage,
  expectObservedOrigin,
  observeContextOrigins,
  requireScenarioInput,
  selectVisibleCourse,
  writeContextOriginReceipt,
} from "./real_stack_ui";
import { canvasQtiFixtureArchive } from "./qti_fixture_archive";

const actionTimeoutMs = 30_000;
const qtiReadyTimeoutMs = 180_000;

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
      configureContextAndPage(elenaContext, elena, actionTimeoutMs);

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
      configureContextAndPage(freshElenaContext, freshElena, actionTimeoutMs);
      await chooseSeededIdentity(freshElena, /Elena Rivera/u);
      await selectVisibleCourse(freshElena, "Biochemistry Base Course");
      await freshElena.getByRole("link", { name: "Workspace", exact: true }).click();
      await expect(freshElena.getByText("Favorite color", { exact: true }).first()).toBeVisible();
      await freshElena.getByRole("button", { name: /^Favorite color\b/u }).click();
      await expect(freshElena.getByLabel("Question title")).toHaveValue("Favorite color");

      expectObservedOrigin(origins.initial, expectedOrigin);
      expectObservedOrigin(origins.fresh, expectedOrigin);
      originEvidenceVerified = true;
    } finally {
      try {
        await Promise.all(contexts.map((context) => context.close()));
      } finally {
        if (originEvidenceVerified) writeContextOriginReceipt(origins);
      }
    }
  });
});
