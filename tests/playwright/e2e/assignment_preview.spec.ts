// Assignment delivery checks on the one production PLE stack.
//
// Selector contract:
// - src/pages/course_assignments_page.tsx:86 owns assignment cards and editor/preview links.
// - src/pages/assignment_editor_page.tsx:522 owns the persisted delivery-check entry link.
// - src/pages/assignment_preview_page.tsx:409 owns the preview cue, builder, and results.
// - src/pages/teaching_operations/course_groups_panel.tsx:336 owns visible group creation.
import { expect, test, type BrowserContext, type Locator, type Page } from "@playwright/test";

import { configuredLiveDemoInputs } from "../../../playwright.config";
import { CORPUS_VIEWPORT_SIZES } from "../ui_corpus_manifest";
import { BIOCHEMISTRY_COURSE_TITLE } from "./helper_course_titles";
import { captureRealStackScreenshot } from "./real_stack_screenshot_capture";
import {
  chooseSeededIdentity,
  configureContextAndPage,
  expectObservedOrigin,
  observeContextOrigins,
  requireScenarioInput,
  restoreViewportOrigin,
  selectVisibleCourse,
  writeContextOriginReceipt,
} from "./real_stack_ui";

const actionTimeoutMs = 30_000;
const scenarioTimeoutMs = 300_000;
const seededQuestionTitle = "Peptide bond resonance and planarity";
const previewMoment = "2080-01-01T09:00";
const dueAt = "2090-06-01T17:00";
const closesAt = "2090-06-02T17:00";

const syntheticResolvedArtifacts = [
  { artifactId: "preview_plane_synthetic_resolved_laptop", viewport: "laptop" },
] as const;

const denialArtifacts = [
  { artifactId: "preview_plane_assignment_preview_denial_laptop", viewport: "laptop" },
  { artifactId: "preview_plane_assignment_preview_denial_tablet", viewport: "tablet" },
  { artifactId: "preview_plane_assignment_preview_denial_iphone_pro", viewport: "iphone_pro" },
  { artifactId: "preview_plane_assignment_preview_denial_square", viewport: "square" },
] as const;

function assignmentCard(page: Page, assignmentTitle: string): Locator {
  return page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: assignmentTitle, exact: true }) });
}

async function expectNoHorizontalOverflow(page: Page): Promise<void> {
  const layout = await page.evaluate(() => ({
    documentWidth: document.documentElement.scrollWidth,
    viewportWidth: window.innerWidth,
  }));
  expect(layout.documentWidth).toBeLessThanOrEqual(layout.viewportWidth);
}

async function createAccommodationGroup(page: Page, groupTitle: string): Promise<void> {
  await page.getByRole("link", { name: "Teaching operations", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Teaching operations" })).toBeVisible();
  const groups = page.getByRole("region", { name: "Groups and sections" });
  const createForm = groups.getByRole("form", { name: "Create group", exact: true });
  const purpose = createForm.getByRole("combobox", { name: "Group purpose", exact: true });
  const members = createForm.getByRole("listbox", { name: "Group members", exact: true });
  await expect(purpose).toBeEnabled();
  await expect(members).toBeEnabled();
  await expect(members.getByRole("option", { name: "Mary Okafor" })).toHaveCount(1);
  await createForm.getByLabel("Group name", { exact: true }).fill(groupTitle);
  await purpose.selectOption("accommodation");
  await members.selectOption({ label: "Mary Okafor" });
  await createForm.getByRole("button", { name: "Create group", exact: true }).click();
  await expect(groups.getByRole("button", { name: groupTitle, exact: true })).toBeVisible();
}

async function createAssignment(page: Page, assignmentTitle: string): Promise<void> {
  await page.getByRole("link", { name: "New assignment", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Create assignment", exact: true })).toBeVisible();
  await page.getByLabel("Assignment title").fill(assignmentTitle);
  await page.getByRole("button", { name: "Choose questions", exact: true }).click();
  const picker = page.getByRole("dialog", { name: "Choose assignment questions", exact: true });
  await expect(picker).toBeVisible();
  await picker.getByLabel("Search questions", { exact: true }).fill(seededQuestionTitle);
  await picker.getByRole("button", { name: "Search questions", exact: true }).click();
  await picker.getByRole("checkbox", { name: new RegExp(seededQuestionTitle) }).check();
  await picker.getByRole("button", { name: "Add selected questions", exact: true }).click();
  await expect(picker).toHaveCount(0);
  await expect(page.locator(".assignment-editor-list")).toContainText(seededQuestionTitle);
  await page.getByRole("button", { name: "Create assignment" }).click();
  await expect(page.getByText(`${assignmentTitle} now appears in this course.`)).toBeVisible();
}

async function enterAssignmentEditorFromList(page: Page, assignmentTitle: string): Promise<void> {
  await page.getByRole("link", { name: "Assignments", exact: true }).click();
  const card = assignmentCard(page, assignmentTitle);
  await expect(card).toHaveCount(1);
  const editorLink = card.getByRole("link", { name: "Edit assignment", exact: true });
  await editorLink.focus();
  await expect(editorLink).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("heading", { name: "Assignment editor", exact: true })).toBeVisible();
}

async function publishScheduledAssignment(page: Page): Promise<void> {
  const teaching = page.getByRole("region", { name: "Teaching operations" });
  await teaching.getByLabel("Lifecycle").selectOption("published");
  await teaching.getByLabel("Due").fill(dueAt);
  await teaching.getByLabel("Closes").fill(closesAt);
  await teaching.getByLabel("Whole-run seconds").fill("1800");
  await teaching.getByLabel("Attempt limit").fill("2");
  await teaching.getByLabel("Late work").selectOption("markLate");
  await teaching.getByRole("button", { name: "Save teaching operations" }).click();
  await expect(page.getByText("Teaching operations saved.", { exact: true })).toBeVisible();
  await expect(page.getByTestId("assignment-current-state")).toHaveText("Published, open now.");
}

async function openDeliveryCheckFromEditor(page: Page): Promise<string> {
  const previewLink = page.getByRole("link", { name: "Check assignment delivery", exact: true });
  await expect(previewLink).toBeVisible();
  await previewLink.focus();
  await expect(previewLink).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("heading", { name: "Assignment delivery check" })).toBeVisible();
  return page.url();
}

async function assertScheduleAndEntitlement(page: Page): Promise<void> {
  await expect(
    page.getByText("Preview only - no learner work or grades are created."),
  ).toBeVisible();
  const schedule = page.getByRole("heading", { name: "Schedule and entitlement" }).locator("..");
  const mary = schedule.getByRole("row").filter({ hasText: "Mary Okafor" });
  const jack = schedule.getByRole("row").filter({ hasText: "Jack Chen" });
  await expect(mary).toContainText("Course wide");
  await expect(mary).toContainText("2090-06-01 17:00:00");
  await expect(mary).toContainText("Base");
  await expect(jack).toContainText("Course wide");
  await expectNoHorizontalOverflow(page);
}

function resultRegion(page: Page): Locator {
  return page.getByRole("heading", { name: "Resolved delivery", exact: true }).locator("..");
}

async function activateDeliveryCheck(page: Page, expectedStatus = 200): Promise<void> {
  const check = page.getByRole("button", { name: "Check assignment delivery", exact: true });
  const responsePromise = page.waitForResponse((response) =>
    new URL(response.url()).pathname.includes("/preview-subjects/"),
  );
  await check.focus();
  await expect(check).toBeFocused();
  await page.keyboard.press("Enter");
  const response = await responsePromise;
  if (response.status() !== expectedStatus) {
    throw new Error(
      `Preview request returned HTTP ${response.status()} instead of ${expectedStatus}: ${await response.text()}`,
    );
  }
}

async function assertDerivedResult(page: Page): Promise<void> {
  const heading = page.getByRole("heading", { name: "Resolved delivery", exact: true });
  await expect(heading).toBeFocused();
  const result = resultRegion(page);
  await expect(result).toContainText("Derived subject; entitlement: Course wide.");
  await expect(result).not.toContainText("Mary Okafor");
  await expect(
    result
      .getByRole("list", { name: "Role-only subject groups" })
      .getByRole("listitem")
      .filter({ hasText: "Accommodation recipient" }),
  ).toBeVisible();
  await expect(result.getByRole("heading", { name: "Before" })).toBeVisible();
  await expect(result.getByRole("heading", { name: "After" })).toBeVisible();
}

async function changeDisclosureInSecondSession(page: Page): Promise<void> {
  const disclosure = page.getByRole("group", { name: "What students can see" });
  await disclosure.getByLabel("Score").selectOption("duringAttempt");
  await disclosure.getByLabel("Per-item correctness").selectOption("afterDue");
  await disclosure.getByLabel("Feedback text").selectOption("afterClose");
  await disclosure.getByLabel("Correct answer or solution").selectOption("never");
  await disclosure.getByLabel("Class statistics").selectOption("never");
  await page.getByRole("button", { name: "Save title, order, and settings" }).click();
  await expect(
    page.getByRole("status").filter({ hasText: "Assignment title, order, and settings saved." }),
  ).toBeVisible();
}

async function assertSyntheticResult(page: Page): Promise<void> {
  const heading = page.getByRole("heading", { name: "Resolved delivery", exact: true });
  await expect(heading).toBeFocused();
  const result = resultRegion(page);
  await expect(result).toContainText("Synthetic subject; entitlement: Course wide.");
  await expect(result).not.toContainText("Mary Okafor");
  await expect(
    result
      .getByRole("list", { name: "Role-only subject groups" })
      .getByRole("listitem")
      .filter({ hasText: "Accommodation recipient" }),
  ).toBeVisible();

  const before = page.getByRole("table", { name: "Delivery before accommodation" });
  const after = page.getByRole("table", { name: "Delivery after accommodation" });
  await expect(before.getByRole("row").filter({ hasText: "Time limit" })).toContainText(
    "1800 seconds",
  );
  await expect(before.getByRole("row").filter({ hasText: "Attempt limit" })).toContainText("2");
  await expect(after.getByRole("row").filter({ hasText: "Time limit" })).toContainText(
    "3600 seconds",
  );
  await expect(after.getByRole("row").filter({ hasText: "Attempt limit" })).toContainText("4");

  const now = result.getByRole("listitem").filter({ hasText: /^Now/u });
  const due = result.getByRole("listitem").filter({ hasText: /^Due/u });
  const close = result.getByRole("listitem").filter({ hasText: /^Close/u });
  await expect(now).toContainText(
    "Score shown; correctness withheld; feedback withheld; solution withheld; statistics withheld.",
  );
  await expect(due).toContainText(
    "Score shown; correctness shown; feedback withheld; solution withheld; statistics withheld.",
  );
  await expect(close).toContainText(
    "Score shown; correctness shown; feedback shown; solution withheld; statistics withheld.",
  );
}

async function captureLaptopSyntheticResult(
  page: Page,
  scenarioInput: ReturnType<typeof requireScenarioInput>,
): Promise<void> {
  const heading = page.getByRole("heading", { name: "Resolved delivery", exact: true });
  for (const artifact of syntheticResolvedArtifacts) {
    await page.setViewportSize(CORPUS_VIEWPORT_SIZES[artifact.viewport]);
    await restoreViewportOrigin(page);
    await heading.scrollIntoViewIfNeeded();
    await expectNoHorizontalOverflow(page);
    await captureRealStackScreenshot(page, scenarioInput, artifact.artifactId);
  }
}

function observeProtectedPreviewCalls(context: BrowserContext): string[] {
  const calls: string[] = [];
  context.on("request", (request) => {
    const path = new URL(request.url()).pathname;
    if (path.endsWith("/preview-schedule") || path.includes("/preview-subjects/")) {
      calls.push(path);
    }
  });
  return calls;
}

async function assertProtectedPreviewDenied(
  page: Page,
  previewUrl: string,
  calls: ReadonlyArray<string>,
): Promise<void> {
  // ASVS 8.2.1 and 8.3.1: direct navigation must stop before protected preview transport mounts.
  await page.goto(previewUrl);
  const denial = page.locator('[data-route-surface="routeAccessDenied"]');
  await expect(denial).toHaveAttribute("data-denied-route", "assignmentPreview");
  await expect(
    denial.getByRole("heading", { name: "This page is available to instructors only" }),
  ).toBeFocused();
  await expect(page.locator('[data-route-surface="assignmentPreview"]')).toHaveCount(0);
  expect(calls).toEqual([]);
}

async function assertUnauthenticatedPreviewBoundary(
  page: Page,
  previewUrl: string,
  calls: ReadonlyArray<string>,
): Promise<void> {
  // A browser without a session receives the shell-owned 401 recovery state.
  // The protected route therefore never mounts and never reaches preview transport.
  await page.goto(previewUrl);
  const recovery = page.locator('[data-session-state="expired"]');
  await expect(recovery).toBeVisible();
  await expect(
    recovery.getByRole("heading", { name: "Your session needs to be renewed" }),
  ).toBeVisible();
  await expect(
    recovery.getByRole("link", { name: "Sign in with a passkey or email" }),
  ).toHaveAttribute("href", "/sign-in");
  await expect(page.locator('[data-route-surface="assignmentPreview"]')).toHaveCount(0);
  expect(calls).toEqual([]);
}

async function captureResponsiveDenial(
  page: Page,
  scenarioInput: ReturnType<typeof requireScenarioInput>,
): Promise<void> {
  for (const artifact of denialArtifacts) {
    await page.setViewportSize(CORPUS_VIEWPORT_SIZES[artifact.viewport]);
    await restoreViewportOrigin(page);
    await expectNoHorizontalOverflow(page);
    await captureRealStackScreenshot(page, scenarioInput, artifact.artifactId);
  }
}

test.describe("assignment delivery preview on the production PLE stack", () => {
  test.skip(
    configuredLiveDemoInputs === undefined,
    "the disposable production browser-suite owner supplies this scenario input",
  );

  test("derived and synthetic previews recover a preserved stale draft without mounting for outsiders", async ({
    browser,
  }) => {
    test.setTimeout(scenarioTimeoutMs);
    const scenarioInput = requireScenarioInput(configuredLiveDemoInputs);
    expect(scenarioInput.scenarioId).toBe("preview_plane");
    const assignmentTitle = "Peptide Bond Structure Practice";
    const groupTitle = "Extended-time learners";
    const expectedOrigin = new URL(scenarioInput.baseUrl).origin;
    const origins = {
      local: { pageOrigins: new Set<string>(), requestOrigins: new Set<string>() },
      remote: { pageOrigins: new Set<string>(), requestOrigins: new Set<string>() },
      mary: { pageOrigins: new Set<string>(), requestOrigins: new Set<string>() },
      outsider: { pageOrigins: new Set<string>(), requestOrigins: new Set<string>() },
    };
    const contexts: BrowserContext[] = [];
    let originEvidenceVerified = false;
    let previewUrl = "";

    try {
      const localContext = await browser.newContext({
        viewport: CORPUS_VIEWPORT_SIZES.laptop,
        ignoreHTTPSErrors: true,
      });
      contexts.push(localContext);
      observeContextOrigins(localContext, origins.local.pageOrigins, origins.local.requestOrigins);
      const local = await localContext.newPage();
      configureContextAndPage(localContext, local, actionTimeoutMs);

      await test.step("Elena creates scenario-owned teaching state through the visible installed Biochemistry course UI", async () => {
        await chooseSeededIdentity(local, /Elena Rivera/u);
        await selectVisibleCourse(local, BIOCHEMISTRY_COURSE_TITLE);
        await createAccommodationGroup(local, groupTitle);
        await createAssignment(local, assignmentTitle);
        await enterAssignmentEditorFromList(local, assignmentTitle);
        await publishScheduledAssignment(local);
        previewUrl = await openDeliveryCheckFromEditor(local);
      });

      await test.step("the inspection table and a derived role-only subject use persisted memberships", async () => {
        await assertScheduleAndEntitlement(local);
        await restoreViewportOrigin(local);
        await captureRealStackScreenshot(
          local,
          scenarioInput,
          "preview_plane_schedule_entitlement",
        );
        await local.getByLabel("Student membership reference").selectOption({
          label: "Mary Okafor",
        });
        await local.getByLabel("Selected course-local moment").fill(previewMoment);
        await activateDeliveryCheck(local);
        await assertDerivedResult(local);
        await captureRealStackScreenshot(local, scenarioInput, "preview_plane_derived_resolved");
      });

      await test.step("a second ordinary Elena session creates a real stale assignment revision", async () => {
        await local.getByLabel("Construct a synthetic group subject", { exact: true }).check();
        await local.getByLabel(`${groupTitle} (accommodation)`, { exact: true }).check();
        const hypothetical = local.getByRole("group", { name: "Hypothetical accommodation" });
        await hypothetical.getByLabel("Override", { exact: true }).check();
        await hypothetical.getByLabel("Whole-run seconds").fill("3600");
        await hypothetical.getByLabel("Attempt limit").fill("4");

        const remoteContext = await browser.newContext({
          viewport: CORPUS_VIEWPORT_SIZES.laptop,
          ignoreHTTPSErrors: true,
        });
        contexts.push(remoteContext);
        observeContextOrigins(
          remoteContext,
          origins.remote.pageOrigins,
          origins.remote.requestOrigins,
        );
        const remote = await remoteContext.newPage();
        configureContextAndPage(remoteContext, remote, actionTimeoutMs);
        await chooseSeededIdentity(remote, /Elena Rivera/u);
        await selectVisibleCourse(remote, BIOCHEMISTRY_COURSE_TITLE);
        await enterAssignmentEditorFromList(remote, assignmentTitle);
        await changeDisclosureInSecondSession(remote);

        await activateDeliveryCheck(local, 412);
        await expect(
          local.getByRole("status").filter({ hasText: "Your hypothetical draft is preserved" }),
        ).toBeVisible();
        await expect(
          local.getByRole("button", { name: "Reload latest assignment revision" }),
        ).toBeVisible();
        await expect(
          local.getByLabel(`${groupTitle} (accommodation)`, { exact: true }),
        ).toBeChecked();
        await expect(hypothetical.getByLabel("Override", { exact: true })).toBeChecked();
        await expect(hypothetical.getByLabel("Whole-run seconds")).toHaveValue("3600");
        await expect(hypothetical.getByLabel("Attempt limit")).toHaveValue("4");
        await restoreViewportOrigin(local);
        await captureRealStackScreenshot(local, scenarioInput, "preview_plane_revision_conflict");
      });

      await test.step("keyboard reload and retry preserve the draft and resolve authoritative policy", async () => {
        // ASVS 2.3.1: the stale workflow reloads the authoritative revision before retrying.
        const reload = local.getByRole("button", { name: "Reload latest assignment revision" });
        await reload.focus();
        await expect(reload).toBeFocused();
        await local.keyboard.press("Enter");
        await expect(
          local.getByRole("status").filter({ hasText: "Your hypothetical draft is preserved" }),
        ).toContainText("The latest assignment revision is loaded.");
        await expect(
          local.getByLabel(`${groupTitle} (accommodation)`, { exact: true }),
        ).toBeChecked();
        const hypothetical = local.getByRole("group", { name: "Hypothetical accommodation" });
        await expect(hypothetical.getByLabel("Override", { exact: true })).toBeChecked();
        await expect(hypothetical.getByLabel("Whole-run seconds")).toHaveValue("3600");
        await expect(hypothetical.getByLabel("Attempt limit")).toHaveValue("4");
        await restoreViewportOrigin(local);
        await captureRealStackScreenshot(local, scenarioInput, "preview_plane_revision_reloaded");

        await activateDeliveryCheck(local);
        await assertSyntheticResult(local);
        await captureLaptopSyntheticResult(local, scenarioInput);
      });

      await test.step("a reload observes persisted policy while the hypothetical result stays ephemeral", async () => {
        await local.reload();
        await assertScheduleAndEntitlement(local);
        await expect(local.getByRole("heading", { name: "Resolved delivery" })).toHaveCount(0);
      });

      await test.step("Mary and an unauthenticated outsider never mount or call protected preview transport", async () => {
        const maryContext = await browser.newContext({
          viewport: CORPUS_VIEWPORT_SIZES.laptop,
          ignoreHTTPSErrors: true,
        });
        contexts.push(maryContext);
        observeContextOrigins(maryContext, origins.mary.pageOrigins, origins.mary.requestOrigins);
        const maryCalls = observeProtectedPreviewCalls(maryContext);
        const mary = await maryContext.newPage();
        configureContextAndPage(maryContext, mary, actionTimeoutMs);
        await chooseSeededIdentity(mary, /Mary Okafor/u);
        await selectVisibleCourse(mary, BIOCHEMISTRY_COURSE_TITLE);
        await assertProtectedPreviewDenied(mary, previewUrl, maryCalls);
        await captureResponsiveDenial(mary, scenarioInput);

        const outsiderContext = await browser.newContext({
          viewport: CORPUS_VIEWPORT_SIZES.laptop,
          ignoreHTTPSErrors: true,
        });
        contexts.push(outsiderContext);
        observeContextOrigins(
          outsiderContext,
          origins.outsider.pageOrigins,
          origins.outsider.requestOrigins,
        );
        const outsiderCalls = observeProtectedPreviewCalls(outsiderContext);
        const outsider = await outsiderContext.newPage();
        configureContextAndPage(outsiderContext, outsider, actionTimeoutMs);
        await assertUnauthenticatedPreviewBoundary(outsider, previewUrl, outsiderCalls);
      });

      for (const observed of Object.values(origins)) {
        expectObservedOrigin(observed, expectedOrigin);
      }
      originEvidenceVerified = true;
    } finally {
      try {
        await Promise.all(contexts.map((context) => context.close()));
      } finally {
        if (originEvidenceVerified) writeContextOriginReceipt(origins, false);
      }
    }
  });
});
