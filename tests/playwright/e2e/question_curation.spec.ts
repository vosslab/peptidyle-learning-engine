// Real-stack WP-INST-D2 Question Curation and reusable-question journey.
//
// Selector contract:
// - src/features/question_curation/question_curation_panel.tsx owns the reuse workspace,
//   its recovery text, and the owner/private-owner collection presentations.
// - src/features/question_picker/question_picker.tsx owns the accessible shared picker dialog.
// - src/pages/assignment_workspace/ owns focused assignment Questions and Policies reuse.
// - src/pages/account_security_page.tsx owns ordinary passkey enrollment and reauthentication.

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
  relativeIsoDate,
  requireScenarioInput,
  selectVisibleCourse,
  signOutVisible,
  writeContextOriginReceipt,
  type ObservedOrigins,
} from "./real_stack_ui";

const actionTimeoutMs = 30_000;
const scenarioTimeoutMs = 600_000;
const nativeQuestionTitle = "Peptide bond resonance and planarity";
const privateFolderQuestionTitle = "Biochemistry Chapter 1: Functional group matching";
const concurrentFolderQuestionTitle = "Genetics Chapter 1: Phenylalanine metabolism";

const workspaceArtifacts = [
  { artifactId: "question_curation_workspace_laptop", viewport: "laptop" },
] as const;
const recoveryArtifacts = [
  { artifactId: "question_curation_revision_recovery_laptop", viewport: "laptop" },
] as const;
const pickerArtifacts = [
  { artifactId: "question_curation_assignment_picker_laptop", viewport: "laptop" },
] as const;

interface CurationWireValue {
  readonly direction: "request" | "response";
  readonly path: string;
  readonly value: unknown;
}

function curationPanel(page: Page): Locator {
  return page.getByRole("region", {
    name: "Organize questions for teaching",
    exact: true,
  });
}

function collectionItem(panel: Locator, title: string): Locator {
  return panel
    .getByRole("region", { name: "Collections", exact: true })
    .getByRole("listitem")
    .filter({ has: panel.page().getByText(title, { exact: true }) });
}

function savedSearchItem(panel: Locator, title: string): Locator {
  return panel
    .getByRole("region", { name: "Saved searches", exact: true })
    .getByRole("listitem")
    .filter({ hasText: title });
}

async function expectNoHorizontalOverflow(page: Page): Promise<void> {
  await expect
    .poll(() =>
      page.evaluate(() => ({
        width: document.documentElement.scrollWidth,
        viewport: window.innerWidth,
      })),
    )
    .toMatchObject({ width: expect.any(Number), viewport: expect.any(Number) });
  const dimensions = await page.evaluate(() => ({
    width: document.documentElement.scrollWidth,
    viewport: window.innerWidth,
  }));
  expect(dimensions.width).toBeLessThanOrEqual(dimensions.viewport);
}

async function captureLaptop(
  page: Page,
  scenarioInput: ReturnType<typeof requireScenarioInput>,
  target: Locator,
  artifacts: ReadonlyArray<{
    readonly artifactId: string;
    readonly viewport: keyof typeof CORPUS_VIEWPORT_SIZES;
  }>,
): Promise<void> {
  for (const artifact of artifacts) {
    await page.setViewportSize(CORPUS_VIEWPORT_SIZES[artifact.viewport]);
    await target.scrollIntoViewIfNeeded();
    await expect(target).toBeVisible();
    await expectNoHorizontalOverflow(page);
    await captureRealStackScreenshot(page, scenarioInput, artifact.artifactId);
  }
  await page.setViewportSize(CORPUS_VIEWPORT_SIZES.laptop);
}

function curationPath(url: string): string | null {
  const path = new URL(url).pathname;
  return path.startsWith("/api/question-collections") ||
    path.startsWith("/api/saved-question-searches")
    ? path
    : null;
}

function observeCurationWire(
  context: BrowserContext,
  values: CurationWireValue[],
  pendingResponses: Promise<void>[],
): void {
  context.on("request", (request) => {
    const path = curationPath(request.url());
    if (path === null || request.postData() === null) return;
    values.push({ direction: "request", path, value: JSON.parse(request.postData()!) });
  });
  context.on("response", (response) => {
    const path = curationPath(response.url());
    if (path === null || response.status() >= 400) return;
    pendingResponses.push(
      response.text().then((body) => {
        if (body !== "") values.push({ direction: "response", path, value: JSON.parse(body) });
      }),
    );
  });
}

async function enterSeededCourse(page: Page, name: RegExp, course: string): Promise<void> {
  await chooseSeededIdentity(page, name);
  await selectVisibleCourse(page, course);
}

async function openLibrary(page: Page): Promise<Locator> {
  await page.getByRole("link", { name: "Library", exact: true }).click();
  const panel = curationPanel(page);
  await expect(panel).toBeVisible();
  return panel;
}

async function selectQuestionInPicker(
  page: Page,
  sourceLabel: string,
  search: string,
  confirmLabel: string,
  beforeConfirm?: (picker: Locator) => Promise<void>,
): Promise<Locator> {
  const picker = page.getByRole("dialog");
  await expect(picker).toBeVisible();
  await picker.getByLabel("Question source").selectOption({ label: sourceLabel });
  await picker.getByLabel("Search questions").fill(search);
  await picker.getByRole("button", { name: "Search questions", exact: true }).click();
  const results = picker.getByRole("region", { name: "Question results", exact: true });
  await expect(results).toBeVisible();
  const choice = results.getByRole("checkbox").first();
  await expect(choice).toBeVisible();
  await choice.focus();
  await page.keyboard.press("Space");
  await expect(choice).toBeChecked();
  if (beforeConfirm !== undefined) await beforeConfirm(picker);
  await picker.getByRole("button", { name: confirmLabel, exact: true }).click();
  await expect(picker).toHaveCount(0);
  return choice;
}

async function stageCurrentLibraryQuestion(
  page: Page,
  panel: Locator,
  questionTitle: string,
): Promise<void> {
  await panel.getByRole("button", { name: "Select questions", exact: true }).click();
  await selectQuestionInPicker(
    page,
    "Current library",
    questionTitle,
    "Prepare selected questions",
  );
  await expect(
    panel.getByRole("heading", { name: "Selected questions ready to save" }),
  ).toBeVisible();
}

async function createNamedCollection(
  panel: Locator,
  title: string,
): Promise<void> {
  await panel.getByRole("button", { name: "Create collection", exact: true }).click();
  const editor = panel
    .getByRole("heading", { name: "Create collection", exact: true })
    .locator("..");
  await editor.getByLabel("Collection name").fill(title);
  await editor.getByRole("button", { name: "Save collection", exact: true }).click();
  await expect(panel.getByRole("status")).toContainText(
    `${title} now contains 0 ordered questions.`,
  );
}

async function appendStagedQuestion(panel: Locator, title: string): Promise<void> {
  await panel.getByRole("button", { name: `Add to ${title}`, exact: true }).click();
  await expect(
    panel.getByRole("heading", { name: "Update collection", exact: true }),
  ).toBeVisible();
  await panel.getByRole("button", { name: "Save collection", exact: true }).click();
  await expect(panel.getByRole("status")).toContainText(
    `${title} now contains 1 ordered questions.`,
  );
}

async function createCourseAndInvitation(page: Page, title: string): Promise<string> {
  await page.getByRole("link", { name: "Courses", exact: true }).click();
  const courses = page.locator('[data-route-surface="courses"]');
  await expect(courses).toBeVisible();
  await courses.getByLabel("Course title").fill(title);
  await courses.getByLabel("Start date").fill(relativeIsoDate(-1));
  await courses.getByLabel("End date").fill(relativeIsoDate(30));
  await courses.getByLabel("Time zone (IANA)").fill("America/Chicago");
  await courses.getByRole("button", { name: "Create course", exact: true }).click();
  const course = courses.getByRole("article").filter({ hasText: title });
  await expect(course).toBeVisible();
  await expect(course.getByRole("heading", { name: title, exact: true })).toBeVisible();
  await course.getByRole("link", { name: "Open course", exact: true }).click();
  await expect(page.getByRole("heading", { level: 1, name: title, exact: true })).toBeVisible();
  await page.getByRole("link", { name: "Students", exact: true }).click();
  await page.getByLabel("Course roster email").fill("mary.okafor@live-demo.ple.example");
  await page.getByLabel("Course roster ID").fill("BIO-MARY-004");
  await page.getByRole("button", { name: "Create invitation", exact: true }).click();
  const invitation = page.getByLabel("Invitation link");
  await expect(invitation).toBeVisible();
  return await invitation.inputValue();
}

test.describe("question curation on the production PLE stack", () => {
  test.skip(
    configuredLiveDemoInputs === undefined,
    "the disposable production browser-suite owner supplies this scenario input",
  );

  test("Instructor curates public questions into reusable work while Sysadmin browses the private collection", async ({
    browser,
  }) => {
    test.setTimeout(scenarioTimeoutMs);
    const scenarioInput = requireScenarioInput(configuredLiveDemoInputs);
    expect(scenarioInput.scenarioId).toBe("question_curation");
    expect(scenarioInput.namespace).toMatch(/^bs1-[0-9a-f]{12}-question_curation$/u);

    const contexts: BrowserContext[] = [];
    const origins: Record<string, ObservedOrigins> = {
      elena: { pageOrigins: new Set<string>(), requestOrigins: new Set<string>() },
      concurrent_elena: { pageOrigins: new Set<string>(), requestOrigins: new Set<string>() },
      mary: { pageOrigins: new Set<string>(), requestOrigins: new Set<string>() },
    };
    const curationWire: CurationWireValue[] = [];
    const pendingCurationResponses: Promise<void>[] = [];
    const options = { ignoreHTTPSErrors: true, viewport: CORPUS_VIEWPORT_SIZES.laptop };
    let originEvidenceVerified = false;
    try {
      const elenaContext = await browser.newContext(options);
      const concurrentElenaContext = await browser.newContext(options);
      const maryContext = await browser.newContext(options);
      contexts.push(elenaContext, concurrentElenaContext, maryContext);
      for (const [name, context] of Object.entries({
        elena: elenaContext,
        concurrent_elena: concurrentElenaContext,
        mary: maryContext,
      })) {
        observeContextOrigins(context, origins[name]!.pageOrigins, origins[name]!.requestOrigins);
        observeCurationWire(context, curationWire, pendingCurationResponses);
      }
      const elena = await elenaContext.newPage();
      const concurrentElena = await concurrentElenaContext.newPage();
      const mary = await maryContext.newPage();
      for (const [context, page] of [
        [elenaContext, elena],
        [concurrentElenaContext, concurrentElena],
        [maryContext, mary],
      ] as const) {
        configureContextAndPage(context, page, actionTimeoutMs);
      }

      const privateTitle = "Peptide Bond Study Set";
      const concurrentTitle = "Biochemistry Core Question Set";
      const remoteConcurrentTitle = `${concurrentTitle} Updated`;
      const savedSearchTitle = "Peptide Bond Questions";
      const courseTitle = "Biochemistry: Question Reuse Workshop";
      const assignmentTitle = "Peptide Bond Reuse Practice";

      await test.step("Elena enters the seeded Instructor session, then curates the current Library", async () => {
        await enterSeededCourse(elena, /Elena Rivera/u, BIOCHEMISTRY_COURSE_TITLE);
        await expect(
          elena.getByRole("link", { name: "Teaching operations", exact: true }),
        ).toBeVisible();
        const panel = await openLibrary(elena);
        await createNamedCollection(panel, privateTitle);
        await stageCurrentLibraryQuestion(elena, panel, privateFolderQuestionTitle);
        await appendStagedQuestion(panel, privateTitle);
        await createNamedCollection(panel, concurrentTitle);
        await stageCurrentLibraryQuestion(elena, panel, concurrentFolderQuestionTitle);
        await appendStagedQuestion(panel, concurrentTitle);
        await expect(collectionItem(panel, concurrentTitle)).toContainText(
          "Private collection",
        );
        await captureLaptop(elena, scenarioInput, panel, workspaceArtifacts);

        await elena.getByLabel("Search published questions").fill(nativeQuestionTitle);
        await panel.getByRole("button", { name: "Save current search", exact: true }).click();
        const savedSearch = panel
          .getByRole("heading", { name: "Save this current search", exact: true })
          .locator("..");
        await savedSearch.getByLabel("Search name").fill(savedSearchTitle);
        await savedSearch.getByRole("button", { name: "Save search", exact: true }).click();
        await expect(panel.getByRole("status")).toContainText(
          `${savedSearchTitle} now reruns this search against the current Question Library.`,
        );
        await savedSearchItem(panel, savedSearchTitle)
          .getByRole("button", { name: "Run search", exact: true })
          .click();
        await expect(panel.getByRole("status")).toContainText(
          `${savedSearchTitle} is running against the current Question Library.`,
        );

        const deleteSavedSearch = savedSearchItem(panel, savedSearchTitle).getByRole("button", {
          name: `Delete saved search ${savedSearchTitle}`,
          exact: true,
        });
        await deleteSavedSearch.click();
        const confirmation = elena.getByRole("dialog", {
          name: `Delete saved search "${savedSearchTitle}"?`,
          exact: true,
        });
        const cancelDeletion = confirmation.getByRole("button", { name: "Cancel", exact: true });
        await expect(cancelDeletion).toBeFocused();
        await cancelDeletion.click();
        await expect(deleteSavedSearch).toBeFocused();
        await deleteSavedSearch.click();
        await confirmation
          .getByRole("button", { name: "Delete saved search", exact: true })
          .click();
        await expect(panel.getByRole("status")).toContainText(`${savedSearchTitle} was deleted.`);
      });

      await test.step("Two ordinary Elena contexts make the revision recovery visible without losing her selected list", async () => {
        const panel = curationPanel(elena);
        await collectionItem(panel, concurrentTitle)
          .getByRole("button", { name: "Open", exact: true })
          .click();
        const localEditor = panel
          .getByRole("heading", { name: "Update collection", exact: true })
          .locator("..");
        await localEditor.getByLabel("Collection name").fill(`${concurrentTitle} local revision`);
        await expect(localEditor).toContainText("1 questions in this ordered collection.");

        await chooseSeededIdentity(concurrentElena, /Elena Rivera/u);
        await selectVisibleCourse(concurrentElena, BIOCHEMISTRY_COURSE_TITLE);
        const concurrentPanel = await openLibrary(concurrentElena);
        await collectionItem(concurrentPanel, concurrentTitle)
          .getByRole("button", { name: "Open", exact: true })
          .click();
        const remoteEditor = concurrentPanel
          .getByRole("heading", { name: "Update collection", exact: true })
          .locator("..");
        await remoteEditor.getByLabel("Collection name").fill(remoteConcurrentTitle);
        await remoteEditor.getByRole("button", { name: "Save collection", exact: true }).click();
        await expect(concurrentPanel.getByRole("status")).toContainText(
          `${remoteConcurrentTitle} now contains 1 ordered questions.`,
        );

        await localEditor.getByRole("button", { name: "Save collection", exact: true }).click();
        await expect(panel.getByRole("status")).toContainText(
          "Someone saved a newer version first.",
        );
        await expect(localEditor.getByLabel("Collection name")).toHaveValue(
          `${concurrentTitle} local revision`,
        );
        await expect(localEditor).toContainText("1 questions in this ordered collection.");
        await captureLaptop(elena, scenarioInput, panel, recoveryArtifacts);
        await panel.getByRole("button", { name: "Reload curation", exact: true }).click();
        await expect(collectionItem(panel, remoteConcurrentTitle)).toBeVisible();
      });

      let invitationUrl = "";
      await test.step("The shared assignment picker reuses private Question Folders and My Questions", async () => {
        invitationUrl = await createCourseAndInvitation(elena, courseTitle);
        await elena.getByRole("link", { name: "Assignments", exact: true }).click();
        await elena.getByRole("link", { name: "Create the first assignment", exact: true }).click();
        const createDraft = elena.locator('[data-route-surface="assignmentCreate"]');
        await expect(createDraft).toBeVisible();
        await createDraft.getByLabel("Assignment title").fill(assignmentTitle);
        await createDraft
          .getByRole("button", { name: "Create assignment draft", exact: true })
          .click();
        const workspace = elena.locator('[data-route-surface="assignmentWorkspace"]');
        await expect(
          workspace.getByRole("heading", { name: "Questions", exact: true }),
        ).toBeVisible();

        await workspace.getByRole("button", { name: "Add question pool", exact: true }).click();
        const firstPool = workspace
          .getByRole("listitem", { name: /Question pool at position/u })
          .first();
        await firstPool.getByRole("button", { name: "Choose candidates", exact: true }).click();
        await selectQuestionInPicker(
          elena,
          privateTitle,
          privateFolderQuestionTitle,
          "Add selected candidates",
        );
        await expect(firstPool).toContainText(privateFolderQuestionTitle);

        await workspace.getByRole("button", { name: "Add question pool", exact: true }).click();
        const secondPool = workspace
          .getByRole("listitem", { name: /Question pool at position/u })
          .nth(1);
        await secondPool.getByRole("button", { name: "Choose candidates", exact: true }).click();
        const myPublishedPicker = elena.getByRole("dialog");
        await expect(myPublishedPicker.getByLabel("Question source")).toContainText(
          "My Questions",
        );
        await selectQuestionInPicker(
          elena,
          "My Questions",
          nativeQuestionTitle,
          "Add selected candidates",
          async (picker) => {
            await captureLaptop(elena, scenarioInput, picker, pickerArtifacts);
          },
        );
        await expect(secondPool).toContainText(nativeQuestionTitle);

        await workspace
          .getByRole("button", { name: "Save questions and order", exact: true })
          .click();
        await expect(
          workspace.getByRole("status").filter({ hasText: "Questions and order saved." }),
        ).toBeVisible();
        await workspace.getByRole("link", { name: "Policies", exact: true }).click();
        await expect(
          workspace.getByRole("heading", { name: "Policies", exact: true }),
        ).toBeVisible();
        await elena.getByLabel("Lifecycle").selectOption("published");
        await elena.getByRole("button", { name: "Save assignment policies", exact: true }).click();
        await expect(
          workspace.getByRole("status").filter({ hasText: "Assignment policies saved." }),
        ).toBeVisible();
      });

      await test.step("Mary claims the ordinary invitation and sees the saved reusable assignment as student work", async () => {
        expect(invitationUrl).not.toBe("");
        await chooseSeededIdentity(mary, /Mary Okafor/u);
        await mary.goto(invitationUrl);
        await mary.getByRole("button", { name: "Claim this course", exact: true }).click();
        await expect(mary.getByRole("heading", { name: courseTitle, exact: true })).toBeVisible();
        const assignment = mary.getByRole("article").filter({ hasText: assignmentTitle });
        await expect(
          assignment.getByRole("heading", { name: assignmentTitle, exact: true }),
        ).toBeVisible();
        await assignment.getByRole("link", { name: "Start assignment", exact: true }).click();
        await expect(mary.locator('[data-route-surface="assignmentOverview"]')).toBeVisible();
        await expect(
          mary.getByRole("heading", { name: assignmentTitle, exact: true }),
        ).toBeVisible();
      });

      await Promise.all(pendingCurationResponses);
      expect(curationWire.some((value) => value.direction === "request")).toBe(true);
      expect(curationWire.some((value) => value.direction === "response")).toBe(true);
      const expectedOrigin = new URL(scenarioInput.baseUrl).origin;
      for (const observed of Object.values(origins)) {
        expectObservedOrigin(observed, expectedOrigin);
      }
      originEvidenceVerified = true;
    } finally {
      await Promise.all(contexts.map(async (context) => await context.close()));
      if (originEvidenceVerified) writeContextOriginReceipt(origins);
    }
  });
});
