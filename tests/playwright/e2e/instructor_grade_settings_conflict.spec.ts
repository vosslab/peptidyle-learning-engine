// Two ordinary instructor sessions resolve a real optimistic-concurrency conflict through PLE.
import { expect, test, type BrowserContext, type Locator, type Page } from "@playwright/test";
import { writeFileSync } from "node:fs";

import { configuredLiveDemoInputs } from "../../../playwright.config";
import { CORPUS_VIEWPORT_SIZES } from "../ui_corpus_manifest";
import { liveDemoOriginReceiptPathFromEnvironment } from "../browser_suite_live_config";
import {
  chooseSeededIdentity,
  observeContextOrigins,
  requireScenarioInput,
  restoreViewportOrigin,
  selectVisibleCourse,
} from "./real_stack_ui";
import { captureRealStackScreenshot } from "./real_stack_screenshot_capture";

const actionTimeoutMs = 30_000;

const remoteObservedResponsiveArtifacts = [
  { artifactId: "grade_settings_remote_observed_laptop", viewport: "laptop" },
  { artifactId: "grade_settings_remote_observed_tablet", viewport: "tablet" },
  { artifactId: "grade_settings_remote_observed_iphone_pro", viewport: "iphone_pro" },
  { artifactId: "grade_settings_remote_observed_square", viewport: "square" },
] as const;

interface ObservedOrigins {
  readonly pageOrigins: Set<string>;
  readonly requestOrigins: Set<string>;
}

function isoDate(offsetDays: number): string {
  const date = new Date();
  date.setDate(date.getDate() + offsetDays);
  return date.toISOString().slice(0, 10);
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

async function openGradeSettings(page: Page): Promise<void> {
  await page.getByRole("link", { name: "Grade settings", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Course grade settings" })).toBeVisible();
}

function gradeSettingsStatus(page: Page): Locator {
  return page.locator("#grade-settings-status");
}

async function addLetterBand(page: Page, label: string): Promise<void> {
  await page.getByRole("button", { name: "Add letter band" }).click();
  const labels = page.getByRole("textbox", { name: "Label" });
  await expect(labels.last()).toBeVisible();
  await labels.last().fill(label);
}

async function expectRemoteObservedResponsiveState(page: Page, localLabel: string): Promise<void> {
  await expect(page.getByRole("heading", { name: "Course grade settings" })).toBeVisible();
  await expect(page.getByRole("textbox", { name: "Label" }).last()).toBeVisible();
  await expect(page.getByRole("textbox", { name: "Label" }).last()).toHaveValue(localLabel);
  await expect(page.getByRole("button", { name: "Reload current settings" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Save grade settings" })).toBeVisible();
  const layout = await page.evaluate(() => ({
    documentWidth: document.documentElement.scrollWidth,
    viewportWidth: window.innerWidth,
  }));
  expect(layout.documentWidth).toBeLessThanOrEqual(layout.viewportWidth);
}

async function captureRemoteObservedResponsiveStates(
  page: Page,
  scenarioInput: ReturnType<typeof requireScenarioInput>,
  localLabel: string,
): Promise<void> {
  for (const artifact of remoteObservedResponsiveArtifacts) {
    await page.setViewportSize(CORPUS_VIEWPORT_SIZES[artifact.viewport]);
    await expectRemoteObservedResponsiveState(page, localLabel);
    await captureRealStackScreenshot(page, scenarioInput, artifact.artifactId);
  }
}

test.describe("instructor grade-settings conflicts on the production PLE stack", () => {
  test.skip(
    configuredLiveDemoInputs === undefined,
    "the disposable production browser-suite owner supplies this scenario input",
  );

  test("a stale instructor draft is retained, retried by keyboard, and observed after reload", async ({
    browser,
  }) => {
    test.setTimeout(180_000);
    const scenarioInput = requireScenarioInput(configuredLiveDemoInputs);
    expect(scenarioInput.scenarioId).toBe("grade_settings_conflict");
    expect(scenarioInput.namespace).toMatch(/^bs1-[0-9a-f]{12}-grade_settings_conflict$/u);
    expect(scenarioInput.sysadminRequirement).toBe("not_required");
    const courseTitle = `Conflict course ${scenarioInput.namespace}`;
    const namespaceToken = scenarioInput.namespace.slice(4, 16);
    expect(namespaceToken).toMatch(/^[0-9a-f]{12}$/u);
    const remoteLabel = `Remote ${namespaceToken}`;
    const localLabel = `Local ${namespaceToken}`;
    expect(remoteLabel.length).toBeLessThanOrEqual(32);
    expect(localLabel.length).toBeLessThanOrEqual(32);
    const expectedOrigin = new URL(scenarioInput.baseUrl).origin;
    const origins = {
      local: { pageOrigins: new Set<string>(), requestOrigins: new Set<string>() },
      remote: { pageOrigins: new Set<string>(), requestOrigins: new Set<string>() },
    };
    const contexts: BrowserContext[] = [];
    let originEvidenceVerified = false;

    try {
      const localContext = await browser.newContext({
        viewport: { width: 1280, height: 800 },
        ignoreHTTPSErrors: true,
      });
      const remoteContext = await browser.newContext({
        viewport: { width: 1280, height: 800 },
        ignoreHTTPSErrors: true,
      });
      contexts.push(localContext, remoteContext);
      observeContextOrigins(localContext, origins.local.pageOrigins, origins.local.requestOrigins);
      observeContextOrigins(
        remoteContext,
        origins.remote.pageOrigins,
        origins.remote.requestOrigins,
      );
      const local = await localContext.newPage();
      const remote = await remoteContext.newPage();
      configure(localContext, local);
      configure(remoteContext, remote);

      await test.step("Elena creates and opens this scenario's course through the visible UI", async () => {
        await chooseSeededIdentity(local, /Elena Rivera/u);
        await selectVisibleCourse(local, "Biochemistry Base Course");
        await local.getByRole("link", { name: "Courses", exact: true }).click();
        await local.getByLabel("Course title").fill(courseTitle);
        await local.getByLabel("Start date").fill(isoDate(-30));
        await local.getByLabel("End date").fill(isoDate(365));
        await local.getByLabel("Time zone (IANA)").fill("America/Chicago");
        await local.getByRole("button", { name: "Create course" }).click();
        const createdCourse = local
          .getByRole("article")
          .filter({ has: local.getByRole("heading", { name: courseTitle, exact: true }) });
        await expect(createdCourse).toHaveCount(1);
        await createdCourse.getByRole("link", { name: "Open course", exact: true }).click();
        await openGradeSettings(local);
      });

      await test.step("a second Elena session opens the scenario-owned course", async () => {
        await chooseSeededIdentity(remote, /Elena Rivera/u);
        await selectVisibleCourse(remote, courseTitle);
        await openGradeSettings(remote);
      });

      await test.step("the remote instructor saves an ordinary visible change", async () => {
        await addLetterBand(remote, remoteLabel);
        await remote.getByRole("button", { name: "Save grade settings" }).click();
        await expect(gradeSettingsStatus(remote)).toHaveText("Grade settings saved.");
      });

      await test.step("the stale instructor sees retained work and deliberately retries it", async () => {
        await addLetterBand(local, localLabel);
        await local.getByRole("button", { name: "Save grade settings" }).click();
        await expect(gradeSettingsStatus(local)).toContainText("Your draft is preserved");
        const preservedDraft = local.getByRole("textbox", { name: "Label" }).last();
        await expect(preservedDraft).toHaveValue(localLabel);
        await restoreViewportOrigin(local);
        await gradeSettingsStatus(local).scrollIntoViewIfNeeded();
        await captureRealStackScreenshot(local, scenarioInput, "grade_settings_conflict_detected");

        const retry = local.getByRole("button", { name: "Save grade settings" });
        await retry.focus();
        await expect(retry).toBeFocused();
        await local.keyboard.press("Enter");
        await expect(gradeSettingsStatus(local)).toHaveText("Grade settings saved.");
        await restoreViewportOrigin(local);
        await expect(local.getByRole("heading", { name: "Course grade settings" })).toBeVisible();
        await gradeSettingsStatus(local).scrollIntoViewIfNeeded();
        await captureRealStackScreenshot(local, scenarioInput, "grade_settings_retry_saved");
      });

      await test.step("the second session reloads and observes the authoritative result", async () => {
        await remote.getByRole("button", { name: "Reload current settings" }).click();
        await expect(remote.getByRole("textbox", { name: "Label" }).last()).toHaveValue(localLabel);
        await expect(gradeSettingsStatus(remote)).toHaveText("");
        await captureRemoteObservedResponsiveStates(remote, scenarioInput, localLabel);
      });
      expectScenarioOrigin(origins.local, expectedOrigin);
      expectScenarioOrigin(origins.remote, expectedOrigin);
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
