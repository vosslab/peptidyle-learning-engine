// Real-stack WP-PROF-D2 curation and reusable-question journey.
//
// Selector contract:
// - src/features/problem_curation/problem_curation_panel.tsx owns the reuse workspace,
//   its recovery text, and the owner/institution-reader collection presentations.
// - src/features/problem_picker/problem_picker.tsx owns the accessible shared picker dialog.
// - src/pages/assignment_editor_page.tsx and assignment_pool_editor.tsx own assignment reuse.
// - src/pages/account_security_page.tsx owns ordinary passkey enrollment and reauthentication.

import { expect, test, type BrowserContext, type Locator, type Page } from "@playwright/test";

import { configuredLiveDemoInputs } from "../../../playwright.config";
import { installVirtualAuthenticator, removeVirtualAuthenticator } from "../helper_live_demo";
import { CORPUS_VIEWPORT_SIZES } from "../ui_corpus_manifest";
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
const baseCourseTitle = "Biochemistry Base Course";
const sysadminCourseTitle = "Genetics Practice Course";
const nativeQuestionTitle = "Peptide bond resonance and planarity";
const favoritesQuestionTitle = "Biochemistry Chapter 1: Charged functional groups";
const privateCollectionQuestionTitle = "Biochemistry Chapter 1: Functional group matching";
const institutionCollectionQuestionTitle = "Genetics Chapter 1: Phenylalanine metabolism";

const workspaceArtifacts = [
  { artifactId: "problem_curation_workspace_laptop", viewport: "laptop" },
  { artifactId: "problem_curation_workspace_tablet", viewport: "tablet" },
  { artifactId: "problem_curation_workspace_iphone_pro", viewport: "iphone_pro" },
  { artifactId: "problem_curation_workspace_square", viewport: "square" },
] as const;
const recoveryArtifacts = [
  { artifactId: "problem_curation_revision_recovery_laptop", viewport: "laptop" },
  { artifactId: "problem_curation_revision_recovery_tablet", viewport: "tablet" },
  { artifactId: "problem_curation_revision_recovery_iphone_pro", viewport: "iphone_pro" },
  { artifactId: "problem_curation_revision_recovery_square", viewport: "square" },
] as const;
const pickerArtifacts = [
  { artifactId: "problem_curation_assignment_picker_laptop", viewport: "laptop" },
  { artifactId: "problem_curation_assignment_picker_tablet", viewport: "tablet" },
  { artifactId: "problem_curation_assignment_picker_iphone_pro", viewport: "iphone_pro" },
  { artifactId: "problem_curation_assignment_picker_square", viewport: "square" },
] as const;
const institutionArtifacts = [
  { artifactId: "problem_curation_institution_projection_laptop", viewport: "laptop" },
  { artifactId: "problem_curation_institution_projection_tablet", viewport: "tablet" },
  { artifactId: "problem_curation_institution_projection_iphone_pro", viewport: "iphone_pro" },
  { artifactId: "problem_curation_institution_projection_square", viewport: "square" },
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
    .filter({ hasText: title });
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

async function captureResponsive(
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
  return path.startsWith("/api/problem-collections") ||
    path.startsWith("/api/saved-problem-searches")
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

async function signInWithPasskey(
  page: Page,
  name: RegExp,
  course: string,
  label: string,
): Promise<{
  readonly authenticator: Awaited<ReturnType<typeof installVirtualAuthenticator>>;
}> {
  const authenticator = await installVirtualAuthenticator(page);
  await chooseSeededIdentity(page, name);
  await selectVisibleCourse(page, course);
  await page.getByRole("link", { name: "Account", exact: true }).click();
  const account = page.locator('[data-route-surface="accountSecurity"]');
  await expect(account).toBeVisible();
  await account.getByLabel("Passkey name").fill(label);
  await account.getByRole("button", { name: "Add passkey", exact: true }).click();
  await expect(account.getByRole("status")).toHaveText("Passkey added.");
  await signOutVisible(page);
  await page.getByRole("button", { name: "Sign in with a passkey", exact: true }).click();
  await selectVisibleCourse(page, course);
  return { authenticator };
}

async function openLibrary(page: Page, personalCuration = true): Promise<Locator> {
  await page.getByRole("link", { name: "Library", exact: true }).click();
  const panel = curationPanel(page);
  await expect(panel).toBeVisible();
  if (personalCuration) {
    const favorites = collectionItem(panel, "Favorites");
    await expect(favorites).toBeVisible();
    await expect(favorites.getByRole("button", { name: "Open", exact: true })).toBeVisible();
  }
  return panel;
}

async function selectQuestionInPicker(
  page: Page,
  sourceLabel: string,
  search: string,
  confirmLabel: string,
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
  visibility: "private" | "institution",
): Promise<void> {
  await panel.getByRole("button", { name: "Create collection", exact: true }).click();
  const editor = panel
    .getByRole("heading", { name: "Create collection", exact: true })
    .locator("..");
  await editor.getByLabel("Collection name").fill(title);
  await editor.getByLabel("Visibility").selectOption(visibility);
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

async function createCourseAndInvitation(
  page: Page,
  title: string,
  namespace: string,
): Promise<string> {
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
  await page.getByLabel("Institutional email").fill("mary.okafor@live-demo.ple.example");
  await page.getByLabel("Institutional student ID").fill(`d2-mary-${namespace}`);
  await page.getByRole("button", { name: "Create invitation", exact: true }).click();
  const invitation = page.getByLabel("Invitation link");
  await expect(invitation).toBeVisible();
  return await invitation.inputValue();
}

test.describe("problem curation on the production PLE stack", () => {
  test.skip(
    configuredLiveDemoInputs === undefined,
    "the disposable production browser-suite owner supplies this scenario input",
  );

  test("Instructor curates public questions into reusable work while Sysadmin browses the institution collection", async ({
    browser,
  }) => {
    test.setTimeout(scenarioTimeoutMs);
    const scenarioInput = requireScenarioInput(configuredLiveDemoInputs);
    expect(scenarioInput.scenarioId).toBe("problem_curation");
    expect(scenarioInput.namespace).toMatch(/^bs1-[0-9a-f]{12}-problem_curation$/u);

    const contexts: BrowserContext[] = [];
    const origins: Record<string, ObservedOrigins> = {
      elena: { pageOrigins: new Set<string>(), requestOrigins: new Set<string>() },
      concurrent_elena: { pageOrigins: new Set<string>(), requestOrigins: new Set<string>() },
      mary: { pageOrigins: new Set<string>(), requestOrigins: new Set<string>() },
      morgan: { pageOrigins: new Set<string>(), requestOrigins: new Set<string>() },
    };
    const curationWire: CurationWireValue[] = [];
    const pendingCurationResponses: Promise<void>[] = [];
    const options = { ignoreHTTPSErrors: true, viewport: CORPUS_VIEWPORT_SIZES.laptop };
    let elenaAuthenticator: Awaited<ReturnType<typeof installVirtualAuthenticator>> | undefined;
    let morganAuthenticator: Awaited<ReturnType<typeof installVirtualAuthenticator>> | undefined;
    let originEvidenceVerified = false;
    try {
      const elenaContext = await browser.newContext(options);
      const concurrentElenaContext = await browser.newContext(options);
      const maryContext = await browser.newContext(options);
      const morganContext = await browser.newContext(options);
      contexts.push(elenaContext, concurrentElenaContext, maryContext, morganContext);
      for (const [name, context] of Object.entries({
        elena: elenaContext,
        concurrent_elena: concurrentElenaContext,
        mary: maryContext,
        morgan: morganContext,
      })) {
        observeContextOrigins(context, origins[name]!.pageOrigins, origins[name]!.requestOrigins);
        observeCurationWire(context, curationWire, pendingCurationResponses);
      }
      const elena = await elenaContext.newPage();
      const concurrentElena = await concurrentElenaContext.newPage();
      const mary = await maryContext.newPage();
      const morgan = await morganContext.newPage();
      for (const [context, page] of [
        [elenaContext, elena],
        [concurrentElenaContext, concurrentElena],
        [maryContext, mary],
        [morganContext, morgan],
      ] as const) {
        configureContextAndPage(context, page, actionTimeoutMs);
      }

      const privateTitle = `D2 private set ${scenarioInput.namespace}`;
      const institutionTitle = `D2 institution set ${scenarioInput.namespace}`;
      const remoteInstitutionTitle = `${institutionTitle} refreshed`;
      const savedSearchTitle = `D2 peptide search ${scenarioInput.namespace}`;
      const courseTitle = `D2 reusable course ${scenarioInput.namespace}`;
      const assignmentTitle = `D2 reusable assignment ${scenarioInput.namespace}`;

      await test.step("Elena enters through the live Instructor and passkey path, then curates the current Library", async () => {
        const passkey = await signInWithPasskey(
          elena,
          /Elena Rivera/u,
          baseCourseTitle,
          `Elena D2 key ${scenarioInput.namespace}`,
        );
        elenaAuthenticator = passkey.authenticator;
        await expect(
          elena.getByRole("link", { name: "Teaching operations", exact: true }),
        ).toBeVisible();
        const panel = await openLibrary(elena);
        await stageCurrentLibraryQuestion(elena, panel, favoritesQuestionTitle);
        await panel.getByRole("button", { name: "Add to Favorites", exact: true }).click();
        await panel.getByRole("button", { name: "Save collection", exact: true }).click();
        await expect(panel.getByRole("status")).toContainText(
          "Favorites now contains 1 ordered questions.",
        );

        await createNamedCollection(panel, privateTitle, "private");
        await stageCurrentLibraryQuestion(elena, panel, privateCollectionQuestionTitle);
        await appendStagedQuestion(panel, privateTitle);
        await createNamedCollection(panel, institutionTitle, "institution");
        await stageCurrentLibraryQuestion(elena, panel, institutionCollectionQuestionTitle);
        await appendStagedQuestion(panel, institutionTitle);
        await expect(collectionItem(panel, institutionTitle)).toContainText(
          "Institution collection",
        );
        await captureResponsive(elena, scenarioInput, panel, workspaceArtifacts);

        await elena.getByLabel("Search published questions").fill(nativeQuestionTitle);
        await panel.getByRole("button", { name: "Save current search", exact: true }).click();
        const savedSearch = panel
          .getByRole("heading", { name: "Save this current search", exact: true })
          .locator("..");
        await savedSearch.getByLabel("Search name").fill(savedSearchTitle);
        await savedSearch.getByRole("button", { name: "Save search", exact: true }).click();
        await expect(panel.getByRole("status")).toContainText(
          `${savedSearchTitle} now reruns this search against the current catalog.`,
        );
        await savedSearchItem(panel, savedSearchTitle)
          .getByRole("button", { name: "Run search", exact: true })
          .click();
        await expect(panel.getByRole("status")).toContainText(
          `${savedSearchTitle} is running against the current catalog.`,
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
        await collectionItem(panel, institutionTitle)
          .getByRole("button", { name: "Open", exact: true })
          .click();
        const localEditor = panel
          .getByRole("heading", { name: "Update collection", exact: true })
          .locator("..");
        await localEditor.getByLabel("Collection name").fill(`${institutionTitle} local revision`);
        await expect(localEditor).toContainText("1 questions in this ordered collection.");

        await chooseSeededIdentity(concurrentElena, /Elena Rivera/u);
        await selectVisibleCourse(concurrentElena, baseCourseTitle);
        const concurrentPanel = await openLibrary(concurrentElena);
        await collectionItem(concurrentPanel, institutionTitle)
          .getByRole("button", { name: "Open", exact: true })
          .click();
        const remoteEditor = concurrentPanel
          .getByRole("heading", { name: "Update collection", exact: true })
          .locator("..");
        await remoteEditor.getByLabel("Collection name").fill(remoteInstitutionTitle);
        await remoteEditor.getByRole("button", { name: "Save collection", exact: true }).click();
        await expect(concurrentPanel.getByRole("status")).toContainText(
          `${remoteInstitutionTitle} now contains 1 ordered questions.`,
        );

        await localEditor.getByRole("button", { name: "Save collection", exact: true }).click();
        await expect(panel.getByRole("status")).toContainText(
          "Someone saved a newer version first.",
        );
        await expect(localEditor.getByLabel("Collection name")).toHaveValue(
          `${institutionTitle} local revision`,
        );
        await expect(localEditor).toContainText("1 questions in this ordered collection.");
        await captureResponsive(elena, scenarioInput, panel, recoveryArtifacts);
        await panel.getByRole("button", { name: "Reload curation", exact: true }).click();
        await expect(collectionItem(panel, remoteInstitutionTitle)).toBeVisible();
      });

      let invitationUrl = "";
      await test.step("The shared assignment picker reuses Favorites, a named collection, and My published questions", async () => {
        invitationUrl = await createCourseAndInvitation(
          elena,
          courseTitle,
          scenarioInput.namespace,
        );
        await elena.getByRole("link", { name: "Assignments", exact: true }).click();
        await elena.getByRole("link", { name: "Create the first assignment", exact: true }).click();
        const editor = elena.locator('[data-route-surface="assignmentEditor"]');
        await expect(editor).toBeVisible();
        await editor.getByLabel("Assignment title").fill(assignmentTitle);

        await editor.getByRole("button", { name: "Choose questions", exact: true }).click();
        await selectQuestionInPicker(
          elena,
          "Favorites",
          favoritesQuestionTitle,
          "Add selected questions",
        );
        await expect(editor).toContainText(favoritesQuestionTitle);

        await editor.getByRole("button", { name: "Add question pool", exact: true }).click();
        const firstPool = editor
          .getByRole("listitem", { name: /Question pool at position/u })
          .first();
        await firstPool.getByRole("button", { name: "Choose candidates", exact: true }).click();
        await selectQuestionInPicker(
          elena,
          privateTitle,
          privateCollectionQuestionTitle,
          "Add selected candidates",
        );
        await expect(firstPool).toContainText(privateCollectionQuestionTitle);

        await editor.getByRole("button", { name: "Add question pool", exact: true }).click();
        const secondPool = editor
          .getByRole("listitem", { name: /Question pool at position/u })
          .nth(1);
        await secondPool.getByRole("button", { name: "Choose candidates", exact: true }).click();
        const myPublishedPicker = elena.getByRole("dialog");
        await expect(myPublishedPicker.getByLabel("Question source")).toContainText(
          "My published questions",
        );
        await selectQuestionInPicker(
          elena,
          "My published questions",
          nativeQuestionTitle,
          "Add selected candidates",
        );
        await expect(secondPool).toContainText(nativeQuestionTitle);
        await captureResponsive(elena, scenarioInput, editor, pickerArtifacts);

        await editor.getByRole("button", { name: "Create assignment", exact: true }).click();
        await expect(
          editor.getByRole("heading", { name: "Assignment created", exact: true }),
        ).toBeVisible();
        await editor.getByRole("link", { name: `Open ${assignmentTitle}`, exact: true }).click();
        await elena.getByLabel("Lifecycle").selectOption("published");
        await elena.getByRole("button", { name: "Save teaching operations", exact: true }).click();
        await expect(elena.getByTestId("assignment-current-state")).toHaveText(
          "Published, open now.",
        );
      });

      await test.step("Mary claims the ordinary invitation and sees the saved reusable assignment as learner work", async () => {
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

      await test.step("Morgan uses the ordinary Sysadmin passkey path to browse and reuse an institution collection", async () => {
        const passkey = await signInWithPasskey(
          morgan,
          /Morgan Reyes/u,
          sysadminCourseTitle,
          `Morgan D2 key ${scenarioInput.namespace}`,
        );
        morganAuthenticator = passkey.authenticator;
        const panel = await openLibrary(morgan, false);
        const institutionItem = collectionItem(panel, remoteInstitutionTitle);
        await expect(institutionItem).toContainText("Institution collection");
        await expect(
          institutionItem.getByRole("button", { name: "Delete", exact: true }),
        ).toHaveCount(0);
        await institutionItem.getByRole("button", { name: "Open", exact: true }).click();
        const projection = panel.getByRole("region", {
          name: remoteInstitutionTitle,
          exact: true,
        });
        await expect(projection).toContainText("Ready to reuse");
        await expect(projection).toContainText("collection owner controls its name");
        await expect(projection.getByLabel("Collection name")).toHaveCount(0);
        await expect(projection.getByLabel("Visibility")).toHaveCount(0);
        await expect(
          projection.getByRole("button", { name: "Save collection", exact: true }),
        ).toHaveCount(0);
        await captureResponsive(morgan, scenarioInput, panel, institutionArtifacts);
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
      try {
        if (elenaAuthenticator !== undefined) await removeVirtualAuthenticator(elenaAuthenticator);
        if (morganAuthenticator !== undefined)
          await removeVirtualAuthenticator(morganAuthenticator);
      } finally {
        await Promise.all(contexts.map(async (context) => await context.close()));
        if (originEvidenceVerified) writeContextOriginReceipt(origins);
      }
    }
  });
});
