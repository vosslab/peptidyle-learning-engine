// Real-stack WP-PROF-D1 discovery journey. The installed native question is reused through
// visible course and learner workflows so published evidence reflects ordinary completed work.
//
// Selector contract:
// - src/pages/library_page.tsx owns search, metadata filters, and the Library result region.
// - src/pages/problem_detail_page.tsx and catalog_statistics_panel.tsx own evidence and usage.
// - src/pages/course_assignments_page.tsx, assignment_editor_page.tsx, and course_roster_page.tsx
//   own assignment creation, publishing, and visible invitation links.
// - src/pages/course_invitation_page.tsx, assignment_overview_page.tsx, and run_page.tsx own
//   learner claim, start, submission, feedback, and completion.

import { expect, test, type BrowserContext, type Locator, type Page } from "@playwright/test";

import { configuredLiveDemoInputs } from "../../../playwright.config";
import { CORPUS_VIEWPORT_SIZES } from "../ui_corpus_manifest";
import { captureRealStackScreenshot } from "./real_stack_screenshot_capture";
import {
  chooseSeededIdentity,
  configureContextAndPage,
  observeContextOrigins,
  requireScenarioInput,
  selectVisibleCourse,
  signOutVisible,
  writeOriginReceipt,
} from "./real_stack_ui";

const actionTimeoutMs = 30_000;
const scenarioTimeoutMs = 420_000;
const baseCourseTitle = "Biochemistry Base Course";
const geneticsCourseTitle = "Genetics Practice Course";
const baseAssignmentTitle = "Peptide Bonds: Structure and Resonance";
const nativeQuestionTitle = "Peptide bond resonance and planarity";
const firstVisibleResponseIndex = 0;

const evidenceArtifacts = [
  { artifactId: "catalog_discovery_disclosed_evidence_laptop", viewport: "laptop" },
] as const;
const usageArtifacts = [
  { artifactId: "catalog_discovery_authorized_usage_laptop", viewport: "laptop" },
] as const;
const libraryArtifacts = [
  { artifactId: "catalog_discovery_filtered_library_laptop", viewport: "laptop" },
] as const;

const emails = {
  elena: "elena.rivera@live-demo.ple.example",
  morgan: "morgan.reyes@live-demo.ple.example",
} as const;

async function libraryQuestionId(page: Page): Promise<string> {
  await page.getByRole("link", { name: "Library", exact: true }).click();
  await page.getByLabel("Search published questions").fill(nativeQuestionTitle);
  await expect(page.locator('[data-route-surface="library"] .route-error')).toHaveCount(0);
  const card = page
    .getByRole("region", { name: "Published questions" })
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: nativeQuestionTitle, exact: true }) });
  await expect(card).toHaveCount(1);
  const questionId = await card.locator("code").innerText();
  expect(questionId).toMatch(/^[A-Z0-9]{3}-[A-Z0-9]{4}$/u);
  return questionId;
}

async function openLibraryDetail(page: Page): Promise<void> {
  const card = page
    .getByRole("region", { name: "Published questions" })
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: nativeQuestionTitle, exact: true }) });
  await card.getByRole("link", { name: "Open question", exact: true }).click();
  await expect(page.locator('[data-route-surface="problemDetail"]')).toBeVisible();
}

async function expectUsageOnlyInCourse(page: Page, allowedCourse: string): Promise<void> {
  const usage = page
    .getByRole("heading", { name: "Usage in your institution", exact: true })
    .locator("..");
  await expect(usage).toBeVisible();
  await expect(
    usage.getByText("Your courses", { exact: true }).locator("..").locator("dd"),
  ).toHaveText("1 course");
  await expect(usage.getByRole("listitem")).toHaveCount(1);
  await expect(usage.getByRole("link", { name: allowedCourse, exact: true })).toHaveCount(1);
  const otherCourse = allowedCourse === baseCourseTitle ? geneticsCourseTitle : baseCourseTitle;
  await expect(usage.getByRole("link", { name: otherCourse, exact: true })).toHaveCount(0);
}

async function createPublishedGeneticsAssignment(
  page: Page,
  questionId: string,
  assignmentTitle: string,
): Promise<void> {
  await page.getByRole("link", { name: "Courses", exact: true }).click();
  const course = page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: geneticsCourseTitle, exact: true }) });
  await expect(course).toHaveCount(1);
  await course.getByRole("link", { name: "Open course", exact: true }).click();
  await expect(page.getByRole("heading", { level: 1, name: geneticsCourseTitle })).toBeVisible();
  await page.getByRole("link", { name: "Assignments", exact: true }).click();
  await expect(page.locator("[data-route-surface=courseAssignments]")).toBeVisible();
  await page.getByRole("link", { name: "Create the first assignment", exact: true }).click();
  await page.getByLabel("Assignment title").fill(assignmentTitle);
  await page.getByText("Add several Question IDs", { exact: true }).click();
  await page.getByLabel("Question IDs").fill(questionId);
  await page.getByRole("button", { name: "Add questions by ID", exact: true }).click();
  await page.getByRole("button", { name: "Create assignment", exact: true }).click();
  await expect(page.getByText(`${assignmentTitle} now appears in this course.`)).toBeVisible();
  await page.getByRole("link", { name: `Open ${assignmentTitle}`, exact: true }).click();
  await page.getByLabel("Lifecycle").selectOption("published");
  await page.getByRole("button", { name: "Save teaching operations", exact: true }).click();
  await expect(page.getByTestId("assignment-current-state")).toHaveText("Published, open now.");
}

async function createInvitation(page: Page, email: string, rosterId: string): Promise<string> {
  await page.getByRole("link", { name: "Students", exact: true }).click();
  await expect(page.locator("[data-route-surface=courseRoster]")).toBeVisible();
  await page.getByLabel("Institutional email").fill(email);
  await page.getByLabel("Institutional student ID").fill(rosterId);
  await page.getByRole("button", { name: "Create invitation", exact: true }).click();
  const invitation = page.getByLabel("Invitation link");
  await expect(invitation).toBeVisible();
  const url = await invitation.inputValue();
  expect(new URL(url).origin).toBe(new URL(page.url()).origin);
  return url;
}

async function claimInvitation(
  page: Page,
  person: RegExp,
  invitationUrl: string,
  expectedCourse: string,
): Promise<void> {
  await signOutVisible(page);
  await chooseSeededIdentity(page, person);
  await page.goto(invitationUrl);
  await expect(
    page.getByRole("heading", { name: "Join your PLE course", exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Claim this course", exact: true }).click();
  await expect(page.getByRole("heading", { name: expectedCourse, exact: true })).toBeVisible();
}

async function completeAssignment(page: Page, assignmentTitle: string): Promise<void> {
  const card = page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: assignmentTitle, exact: true }) });
  await expect(card).toHaveCount(1);
  await card.getByRole("link", { name: "Start assignment", exact: true }).click();
  await expect(page.locator('[data-route-surface="assignmentOverview"]')).toBeVisible();
  await page.getByRole("button", { name: "Start or continue practice", exact: true }).click();
  await expect(page.locator('[data-route-surface="runAttempt"]')).toBeVisible();
  const response = page.getByRole("radio").nth(firstVisibleResponseIndex);
  await expect(response).toBeVisible();
  await response.check();
  await page.getByRole("button", { name: "Submit answer", exact: true }).click();
  const feedback = page.getByRole("heading", { name: "Feedback", exact: true }).locator("..");
  await expect(feedback).toBeVisible();
  await page.getByRole("button", { name: "View completed run", exact: true }).click();
  await expect(
    page.locator(".attempt-summary").getByText("Your completed run is recorded."),
  ).toBeVisible();
}

async function assertInitialInsufficientEvidence(page: Page): Promise<void> {
  const evidence = page
    .getByRole("heading", { name: "Learning evidence", exact: true })
    .locator("..");
  await expect(evidence).toContainText("More evidence is needed");
}

async function assertGeneratedCatalogPrompt(page: Page): Promise<void> {
  const prompt = page.getByRole("region", { name: "Problem prompt", exact: true });
  await expect(prompt).toBeVisible();
  await expect(prompt).toContainText(/(glycine|alanine|proline) peptide example/u);
  await expect(prompt).not.toContainText("{{residue}}");
  const notice = page.getByRole("complementary", { name: "Generated example", exact: true });
  await expect(notice).toBeVisible();
  await expect(notice).toContainText("Assigned versions may use different values.");
}

async function assertFiveLearnerEvidence(page: Page): Promise<void> {
  await expect
    .poll(
      async () => {
        await page.reload();
        const evidence = page
          .getByRole("heading", { name: "Learning evidence", exact: true })
          .locator("..");
        return {
          courses: await evidence
            .getByText("Observed courses", { exact: true })
            .locator("..")
            .locator("dd")
            .innerText(),
          observations: await evidence
            .getByText("Independent learner observations", { exact: true })
            .locator("..")
            .locator("dd")
            .innerText(),
        };
      },
      { timeout: 60_000, intervals: [1_000, 2_000, 5_000] },
    )
    .toEqual({ courses: "2 courses", observations: "5 observations" });
}

async function expectNoHorizontalOverflow(page: Page): Promise<void> {
  const layout = await page.evaluate(() => ({
    documentWidth: document.documentElement.scrollWidth,
    viewportWidth: window.innerWidth,
  }));
  expect(layout.documentWidth).toBeLessThanOrEqual(layout.viewportWidth);
}

async function captureLaptopState(
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
}

async function verifyLibraryFilters(page: Page): Promise<void> {
  await page.getByRole("link", { name: "Return to problem library", exact: true }).click();
  const library = page.locator('[data-route-surface="library"]');
  await expect(library).toBeVisible();
  await page.getByLabel("Search published questions").fill(nativeQuestionTitle);
  await page.getByLabel("Byline").selectOption("Dr. Elena Rivera");
  await page.getByLabel("Backend").selectOption("native");
  await page.getByLabel("Tag").selectOption("peptide-bond");
  await page.getByLabel("Response family").selectOption("multipleChoice");
  await page.getByLabel("Used in my courses").selectOption("used");
  const result = library
    .getByRole("region", { name: "Published questions" })
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: nativeQuestionTitle, exact: true }) });
  await expect(result).toHaveCount(1);
}

async function verifyLaptopLibraryKeyboardPath(page: Page): Promise<void> {
  await page.setViewportSize(CORPUS_VIEWPORT_SIZES.laptop);
  const results = page.getByRole("region", { name: "Published questions" });
  const result = results
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: nativeQuestionTitle, exact: true }) });
  const copyId = result.getByRole("button", { name: /^Copy question ID /u });
  const openQuestion = result.getByRole("link", { name: "Open question", exact: true });

  await result.scrollIntoViewIfNeeded();
  await expect(copyId).toBeVisible();
  await expect(openQuestion).toBeVisible();

  await results.focus();
  await expect(results).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(copyId).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(openQuestion).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.locator('[data-route-surface="problemDetail"]')).toBeVisible();
}

test.describe("catalog discovery evidence on the production PLE stack", () => {
  test.skip(
    configuredLiveDemoInputs === undefined,
    "the disposable production browser-suite owner supplies this scenario input",
  );

  test("five independent learner observations disclose evidence after visible course work", async ({
    browser,
  }) => {
    test.setTimeout(scenarioTimeoutMs);
    const scenarioInput = requireScenarioInput(configuredLiveDemoInputs);
    expect(scenarioInput.scenarioId).toBe("catalog_discovery_evidence");
    expect(scenarioInput.namespace).toMatch(/^bs1-[0-9a-f]{12}-catalog_discovery_evidence$/u);

    const contexts: BrowserContext[] = [];
    const pageOrigins = new Set<string>();
    const requestOrigins = new Set<string>();
    const options = { ignoreHTTPSErrors: true, viewport: { width: 1280, height: 800 } };
    try {
      const elenaContext = await browser.newContext(options);
      const maryContext = await browser.newContext(options);
      const jackContext = await browser.newContext(options);
      const morganContext = await browser.newContext(options);
      const averyContext = await browser.newContext(options);
      contexts.push(elenaContext, maryContext, jackContext, morganContext, averyContext);
      for (const context of contexts) observeContextOrigins(context, pageOrigins, requestOrigins);
      const elena = await elenaContext.newPage();
      const mary = await maryContext.newPage();
      const jack = await jackContext.newPage();
      const morgan = await morganContext.newPage();
      const avery = await averyContext.newPage();
      for (const [context, page] of [
        [elenaContext, elena],
        [maryContext, mary],
        [jackContext, jack],
        [morganContext, morgan],
        [averyContext, avery],
      ] as const) {
        configureContextAndPage(context, page, actionTimeoutMs);
      }

      await test.step("Elena finds the installed native question and sees initial evidence plus Base-only usage", async () => {
        await chooseSeededIdentity(elena, /Elena Rivera/u);
        await selectVisibleCourse(elena, baseCourseTitle);
        await libraryQuestionId(elena);
        await openLibraryDetail(elena);
        await assertGeneratedCatalogPrompt(elena);
        await assertInitialInsufficientEvidence(elena);
        await expectUsageOnlyInCourse(elena, baseCourseTitle);
      });

      await test.step("Mary visibly confirms her seeded Base-course completion", async () => {
        await chooseSeededIdentity(mary, /Mary Okafor/u);
        await selectVisibleCourse(mary, baseCourseTitle);
        const card = mary
          .getByRole("article")
          .filter({ has: mary.getByRole("heading", { name: baseAssignmentTitle, exact: true }) });
        await card.getByRole("link", { name: "Start assignment", exact: true }).click();
        await expect(mary.locator('[data-route-surface="assignmentOverview"]')).toBeVisible();
        await expect(mary.getByText("Completed runs", { exact: true })).toBeVisible();
      });

      const geneticsAssignmentTitle = `Evidence genetics assignment ${scenarioInput.namespace}`;
      let elenaGeneticsInvitation = "";
      await test.step("Morgan publishes the installed question in Genetics and sees Genetics-only usage", async () => {
        await chooseSeededIdentity(morgan, /Morgan Reyes/u);
        await selectVisibleCourse(morgan, geneticsCourseTitle);
        const questionId = await libraryQuestionId(morgan);
        await createPublishedGeneticsAssignment(morgan, questionId, geneticsAssignmentTitle);
        await morgan.getByRole("link", { name: "Library", exact: true }).click();
        await morgan.getByLabel("Search published questions").fill(nativeQuestionTitle);
        await openLibraryDetail(morgan);
        await expectUsageOnlyInCourse(morgan, geneticsCourseTitle);
        await signOutVisible(morgan);
        await chooseSeededIdentity(morgan, /Morgan Reyes/u);
        await selectVisibleCourse(morgan, geneticsCourseTitle);
        elenaGeneticsInvitation = await createInvitation(
          morgan,
          emails.elena,
          `elena-genetics-${scenarioInput.namespace}`,
        );
      });

      let morganBaseInvitation = "";
      await test.step("Elena invites Morgan into the Base Course through the roster", async () => {
        await signOutVisible(elena);
        await chooseSeededIdentity(elena, /Elena Rivera/u);
        await selectVisibleCourse(elena, baseCourseTitle);
        morganBaseInvitation = await createInvitation(
          elena,
          emails.morgan,
          `morgan-base-${scenarioInput.namespace}`,
        );
      });

      await test.step("Jack completes the installed open Base-course run", async () => {
        await chooseSeededIdentity(jack, /Jack Chen/u);
        await selectVisibleCourse(jack, baseCourseTitle);
        await completeAssignment(jack, baseAssignmentTitle);
      });

      await test.step("Morgan claims Base and submits the installed question", async () => {
        await claimInvitation(morgan, /Morgan Reyes/u, morganBaseInvitation, baseCourseTitle);
        await completeAssignment(morgan, baseAssignmentTitle);
      });

      await test.step("Avery submits the Genetics assignment", async () => {
        await chooseSeededIdentity(avery, /Avery Singh/u);
        await selectVisibleCourse(avery, geneticsCourseTitle);
        await completeAssignment(avery, geneticsAssignmentTitle);
      });

      await test.step("Elena claims Genetics as a learner and completes its published assignment", async () => {
        await claimInvitation(elena, /Elena Rivera/u, elenaGeneticsInvitation, geneticsCourseTitle);
        await completeAssignment(elena, geneticsAssignmentTitle);
      });

      await test.step("Library filters expose the five-learner, two-course evidence", async () => {
        await signOutVisible(elena);
        await chooseSeededIdentity(elena, /Elena Rivera/u);
        await selectVisibleCourse(elena, baseCourseTitle);
        await libraryQuestionId(elena);
        await openLibraryDetail(elena);
        await assertFiveLearnerEvidence(elena);
        await expectUsageOnlyInCourse(elena, baseCourseTitle);
        await captureLaptopState(
          elena,
          scenarioInput,
          elena.getByRole("heading", { name: "Learning evidence", exact: true }),
          evidenceArtifacts,
        );
        await captureLaptopState(
          elena,
          scenarioInput,
          elena.getByRole("heading", { name: "Usage in your institution", exact: true }),
          usageArtifacts,
        );
        await verifyLibraryFilters(elena);
        await captureLaptopState(
          elena,
          scenarioInput,
          elena.locator('[data-route-surface="library"]'),
          libraryArtifacts,
        );
        await verifyLaptopLibraryKeyboardPath(elena);
      });

      await test.step("Student membership keeps instructor usage detail course-scoped", async () => {
        await signOutVisible(morgan);
        await chooseSeededIdentity(morgan, /Morgan Reyes/u);
        await selectVisibleCourse(morgan, geneticsCourseTitle);
        await libraryQuestionId(morgan);
        await openLibraryDetail(morgan);
        await expectUsageOnlyInCourse(morgan, geneticsCourseTitle);
      });
    } finally {
      await Promise.all(contexts.map((context) => context.close()));
      writeOriginReceipt(pageOrigins, requestOrigins);
    }
  });
});
