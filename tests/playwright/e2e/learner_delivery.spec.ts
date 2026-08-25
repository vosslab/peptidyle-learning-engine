// Connected learner delivery proof. Product state is created through the visible production UI.
//
// Selector contract:
// - src/pages/course_assignments_page.tsx:324 owns the assignments surface and assignment links.
// - src/features/flat_question_authoring/flat_question_editor_page.tsx:535 owns question creation
//   fields and publication controls used to seed the journey.
// - src/pages/course_list_page.tsx:330, src/pages/assignment_editor_page.tsx:481, and
//   src/pages/course_roster_page.tsx:423 own course, assignment, and invitation controls.
// - src/pages/course_invitation_page.tsx:62 and src/pages/assignment_overview_page.tsx:114 own
//   learner claiming and assignment entry; data-route-surface is defined at
//   course_assignments_page.tsx:324.
import { expect, test, type BrowserContext, type Locator, type Page } from "@playwright/test";

import { configuredLiveDemoInputs } from "../../../playwright.config";
import {
  chooseSeededIdentity,
  observeContextOrigins,
  relativeIsoDate,
  requireScenarioInput,
  selectVisibleCourse,
  signOutVisible,
  writeOriginReceipt,
} from "./real_stack_ui";
import { captureRealStackScreenshot } from "./real_stack_screenshot_capture";

const maryEmail = "mary.okafor@live-demo.ple.example";
const journeyTimeoutMs = 600_000;
const actionTimeoutMs = 30_000;
const contextOptions = { viewport: { width: 1280, height: 800 }, ignoreHTTPSErrors: true };

async function createDeterministicBannerPng(page: Page): Promise<Buffer> {
  const encoded = await page.evaluate(() => {
    const canvas = document.createElement("canvas");
    canvas.width = 1_200;
    canvas.height = 328;
    const context = canvas.getContext("2d");
    if (context === null) {
      throw new Error("browser canvas is unavailable for the synthetic banner fixture");
    }

    const background = context.createLinearGradient(0, 0, canvas.width, canvas.height);
    background.addColorStop(0, "#123f35");
    background.addColorStop(0.55, "#26745b");
    background.addColorStop(1, "#8bad63");
    context.fillStyle = background;
    context.fillRect(0, 0, canvas.width, canvas.height);

    context.lineWidth = 10;
    context.strokeStyle = "rgba(235, 245, 218, 0.72)";
    context.fillStyle = "rgba(18, 63, 53, 0.88)";
    const points = [
      [160, 190],
      [370, 105],
      [600, 205],
      [835, 100],
      [1_050, 185],
    ] as const;
    context.beginPath();
    context.moveTo(points[0][0], points[0][1]);
    for (const [x, y] of points.slice(1)) {
      context.lineTo(x, y);
    }
    context.stroke();
    for (const [x, y] of points) {
      context.beginPath();
      context.arc(x, y, 35, 0, Math.PI * 2);
      context.fill();
      context.stroke();
    }

    const base64 = canvas.toDataURL("image/png").split(",", 2)[1];
    if (base64 === undefined) throw new Error("browser canvas produced an invalid PNG data URL");
    return base64;
  });
  return Buffer.from(encoded, "base64");
}

function configureContext(context: BrowserContext): void {
  context.setDefaultTimeout(actionTimeoutMs);
  context.setDefaultNavigationTimeout(actionTimeoutMs);
}

function configurePage(page: Page): void {
  page.setDefaultTimeout(actionTimeoutMs);
  page.setDefaultNavigationTimeout(actionTimeoutMs);
}

async function openCourseAssignments(page: Page): Promise<void> {
  await page.getByRole("link", { name: "Assignments", exact: true }).click();
  await expect(page.locator("[data-route-surface=courseAssignments]")).toBeVisible();
}

async function captureVisibleState(
  page: Page,
  scenarioInput: ReturnType<typeof requireScenarioInput>,
  artifactId: string,
  focus: Locator,
): Promise<void> {
  await expect(focus).toBeVisible();
  await focus.scrollIntoViewIfNeeded();
  await captureRealStackScreenshot(page, scenarioInput, artifactId);
}

async function assertLoadedSameOriginBlobImage(page: Page, image: Locator): Promise<void> {
  await expect(image).toBeVisible();
  await expect
    .poll(async () =>
      image.evaluate((element) => {
        if (!(element instanceof HTMLImageElement)) return false;
        return element.complete && element.naturalWidth > 0;
      }),
    )
    .toBe(true);
  const source = await image.evaluate((element) =>
    element instanceof HTMLImageElement ? element.currentSrc : "",
  );
  if (source === "") throw new Error("the persisted banner image has no current source");
  const sourceUrl = new URL(source);
  expect(sourceUrl.protocol).toBe("blob:");
  expect(sourceUrl.origin).toBe(new URL(page.url()).origin);
}

async function restoreViewportOrigin(page: Page): Promise<void> {
  await page.evaluate(() => window.scrollTo(0, 0));
  await expect
    .poll(() => page.evaluate(() => ({ x: window.scrollX, y: window.scrollY })))
    .toEqual({ x: 0, y: 0 });
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          document.documentElement.scrollWidth <= window.innerWidth &&
          document.body.scrollWidth <= window.innerWidth,
      ),
    )
    .toBe(true);
}

async function createPublishedQuestion(
  page: Page,
  title: string,
  correctChoice: string,
): Promise<string> {
  await page.getByRole("link", { name: "Workspace" }).click();
  await page.getByRole("button", { name: "Create flat question" }).click();
  await page.getByLabel("Question title").fill(title);
  await page.getByLabel("Learner-facing prompt").fill(`Choose the supported statement: ${title}`);
  await page.getByLabel("Choice text").nth(0).fill(correctChoice);
  await page.getByLabel("Choice text").nth(1).fill(`Alternative choice for ${title}`);
  await page
    .getByRole("radio", { name: new RegExp(`Mark choice 1 as correct: ${correctChoice}`) })
    .check();
  await page.getByRole("button", { name: "Save private draft" }).click();
  await page.getByRole("button", { name: "Review publication changes" }).click();
  await page.getByLabel("Reviewed public byline").fill("Dr. Elena Rivera");
  await page.getByRole("button", { name: "Confirm and publish" }).click();
  await expect(page.getByRole("heading", { name: "Published" })).toBeVisible();

  await page.getByRole("link", { name: "Library", exact: true }).click();
  await page.getByLabel("Search published questions").fill(title);
  const questionCard = page
    .getByRole("region", { name: "Published questions" })
    .getByText(title)
    .locator("..");
  await expect(questionCard).toBeVisible();
  const questionId = await questionCard.locator("code").innerText();
  expect(questionId).toMatch(/^[A-Z0-9]{3}-[A-Z0-9]{4}$/u);
  return questionId;
}

async function createPublishedCourseAssignment(
  page: Page,
  courseTitle: string,
  assignmentTitle: string,
  questionId: string,
  namespace: string,
): Promise<string> {
  await page.getByRole("link", { name: "Courses" }).click();
  await page.getByLabel("Course title").fill(courseTitle);
  await page.getByLabel("Start date").fill(relativeIsoDate(-30));
  await page.getByLabel("End date").fill(relativeIsoDate(365));
  await page.getByLabel("Time zone (IANA)").fill("America/Chicago");
  await page.getByRole("button", { name: "Create course" }).click();
  const courseCard = page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: courseTitle, exact: true }) });
  await expect(courseCard).toHaveCount(1);
  await courseCard.getByRole("link", { name: "Open course", exact: true }).click();

  await openCourseAssignments(page);
  await page.getByRole("link", { name: "Create the first assignment" }).click();
  await expect(page.getByRole("heading", { name: "Create assignment" })).toBeVisible();
  await page.getByLabel("Assignment title").fill(assignmentTitle);
  await page.getByText("Add several Question IDs", { exact: true }).click();
  await page.getByLabel("Question IDs").fill(questionId);
  await page.getByRole("button", { name: "Add questions by ID" }).click();
  await page.getByRole("button", { name: "Create assignment" }).click();
  await expect(page.getByText(`${assignmentTitle} now appears in this course.`)).toBeVisible();
  await page.getByRole("link", { name: `Open ${assignmentTitle}` }).click();
  await page.getByLabel("Lifecycle").selectOption("published");
  await page.getByRole("button", { name: "Save teaching operations" }).click();
  await expect(page.getByTestId("assignment-current-state")).toHaveText("Published, open now.");

  await page.getByRole("link", { name: "Students" }).click();
  await page.getByLabel("Institutional email").fill(maryEmail);
  await page.getByLabel("Institutional student ID").fill(`mary-${namespace}`);
  await page.getByRole("button", { name: "Create invitation" }).click();
  const invitation = page.getByLabel("Invitation link");
  await expect(invitation).toBeVisible();
  const invitationUrl = await invitation.inputValue();
  expect(new URL(invitationUrl).origin).toBe(new URL(page.url()).origin);
  return invitationUrl;
}

async function claimCourseAndCompleteAssignment(
  page: Page,
  invitationUrl: string,
  courseTitle: string,
  assignmentTitle: string,
  correctChoice: string,
  scenarioInput: ReturnType<typeof requireScenarioInput>,
): Promise<void> {
  await chooseSeededIdentity(page, /Mary Okafor/u);
  await page.goto(invitationUrl);
  await expect(page.getByRole("heading", { name: "Join your PLE course" })).toBeVisible();
  await page.getByRole("button", { name: "Claim this course" }).click();
  await expect(page.getByRole("heading", { name: courseTitle, exact: true })).toBeVisible();
  const assignmentCard = page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: assignmentTitle, exact: true }) });
  await expect(assignmentCard).toHaveCount(1);
  await expect(assignmentCard.locator(".course-card-description")).toHaveText(
    "Open this assignment to review its instructions and delivery details.",
  );
  await expect(assignmentCard.locator(".course-card-progress")).toHaveText(
    "No score yet. Submit a response to record scored progress.",
  );
  await expect
    .poll(() =>
      assignmentCard.evaluate((card) => {
        const description = card.querySelector(".course-card-description");
        const progress = card.querySelector(".course-card-progress");
        if (!(description instanceof HTMLElement) || !(progress instanceof HTMLElement)) {
          return null;
        }
        return {
          description: getComputedStyle(description).gridArea,
          progress: getComputedStyle(progress).gridArea,
        };
      }),
    )
    .toEqual({ description: "summary", progress: "progress" });
  await captureRealStackScreenshot(page, scenarioInput, "learner_delivery_assignment_list");
  await assignmentCard.getByRole("link", { name: "Start assignment", exact: true }).click();
  await expect(page.locator("[data-route-surface=assignmentOverview]")).toBeVisible();
  await captureRealStackScreenshot(
    page,
    scenarioInput,
    "learner_delivery_assignment_overview_laptop",
  );
  await captureRealStackScreenshot(
    page,
    scenarioInput,
    "learner_delivery_assignment_overview_tablet",
  );
  await captureRealStackScreenshot(
    page,
    scenarioInput,
    "learner_delivery_assignment_overview_iphone_pro",
  );
  await captureRealStackScreenshot(
    page,
    scenarioInput,
    "learner_delivery_assignment_overview_square",
  );
  await page.getByRole("button", { name: "Start or continue practice" }).click();
  await expect(page.locator("[data-route-surface=runAttempt]")).toBeVisible();
  await captureRealStackScreenshot(page, scenarioInput, "learner_delivery_problem_ready");
  const selectedResponse = page.getByRole("radio", { name: correctChoice, exact: true });
  await selectedResponse.focus();
  await page.keyboard.press("Space");
  await expect(selectedResponse).toBeChecked();
  await captureVisibleState(
    page,
    scenarioInput,
    "learner_delivery_response_selected",
    selectedResponse,
  );
  const submitAnswer = page.getByRole("button", { name: "Submit answer" });
  await submitAnswer.focus();
  await page.keyboard.press("Enter");
  const feedback = page.getByRole("heading", { name: "Feedback", exact: true }).locator("..");
  await expect(feedback).toBeVisible();
  await expect(feedback.getByRole("heading", { name: "Correct", exact: true })).toBeVisible();
  await expect(feedback.getByRole("heading", { name: "Your response", exact: true })).toBeVisible();
  await expect(
    feedback.getByRole("heading", { name: "Correct response", exact: true }),
  ).toBeVisible();
  await captureVisibleState(page, scenarioInput, "learner_delivery_feedback_correct", feedback);
  await page.getByRole("button", { name: "View completed run", exact: true }).click();
  const summary = page.locator(".attempt-summary");
  await expect(summary.getByText("Your completed run is recorded.")).toBeVisible();
  const assignmentScore = summary.getByRole("region", { name: "Assignment score" });
  await expect(assignmentScore).toContainText(
    "Score available: Current 100%, Latest 100%, Best 100%.",
  );
  await expect(assignmentScore).toContainText("This run: 100%");
  await captureVisibleState(page, scenarioInput, "learner_delivery_completion", summary);
  await summary.getByRole("button", { name: "Start fresh practice" }).click();
  await expect(page.locator("[data-route-surface=runAttempt]")).toBeVisible();
  const runHeader = page.locator(".run-header");
  await expect(runHeader).toBeVisible();
  await expect(runHeader.getByText("Practice run 2", { exact: true })).toBeVisible();
  await expect(runHeader.getByRole("heading")).toBeVisible();
  await expect(page.locator("header.site-header")).toBeVisible();
  await expect(page.getByRole("navigation", { name: "Primary navigation" })).toBeVisible();
  await restoreViewportOrigin(page);
  await expect(page.getByRole("radio", { name: correctChoice, exact: true })).not.toBeChecked();
  await captureRealStackScreenshot(page, scenarioInput, "learner_delivery_repeat_run");
}

async function observeCompletedRunInFreshSession(
  page: Page,
  courseTitle: string,
  assignmentTitle: string,
  scenarioInput: ReturnType<typeof requireScenarioInput>,
): Promise<void> {
  await chooseSeededIdentity(page, /Mary Okafor/u);
  await selectVisibleCourse(page, courseTitle);
  const assignmentCard = page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: assignmentTitle, exact: true }) });
  await assignmentCard.getByRole("link", { name: "Start assignment", exact: true }).click();
  const overview = page.locator("[data-route-surface=assignmentOverview]");
  await expect(overview).toBeVisible();
  const facts = overview.locator(".assignment-facts");
  const scoreStatus = facts
    .getByText("Score status", { exact: true })
    .locator("..")
    .getByRole("status");
  await expect(scoreStatus).toHaveText("Score available: Current 100%, Latest 100%, Best 100%.");
  await expect(facts.getByText("Completed runs", { exact: true })).toBeVisible();
  const completedRuns = facts
    .getByText("Completed runs", { exact: true })
    .locator("..")
    .locator("dd");
  await expect(completedRuns).toHaveText(/[1-9]\d*/u);
  await captureVisibleState(
    page,
    scenarioInput,
    "learner_delivery_fresh_session_score",
    scoreStatus,
  );
}

async function observeInstructorOutcomesAndAccess(
  page: Page,
  assignmentTitle: string,
  scenarioInput: ReturnType<typeof requireScenarioInput>,
): Promise<void> {
  // Leave the invitation-era roster instance so the visible return performs a
  // fresh server read after Mary's claim and completed run.
  await openCourseAssignments(page);
  await page.getByRole("link", { name: "Students" }).click();
  await expect(page.locator("[data-route-surface=courseRoster]")).toBeVisible();
  const activeLearner = page.getByRole("row", { name: /Mary Okafor/u });
  await expect(activeLearner).toBeVisible();
  await captureVisibleState(
    page,
    scenarioInput,
    "learner_delivery_instructor_active_roster",
    activeLearner,
  );

  await page.getByRole("link", { name: "Gradebook" }).click();
  await expect(page.locator("[data-route-surface=gradebook]")).toBeVisible();
  await expect(page.getByRole("row", { name: new RegExp(assignmentTitle) })).toBeVisible();
  await captureRealStackScreenshot(page, scenarioInput, "learner_delivery_instructor_gradebook");

  await page.getByRole("link", { name: "Appearance" }).click();
  await expect(page.locator("[data-route-surface=courseAppearance]")).toBeVisible();
  await page.getByRole("radio", { name: "Forest" }).check();
  const deterministicBannerPng = await createDeterministicBannerPng(page);
  await page.getByLabel(/Choose a banner image/u).setInputFiles({
    name: "learner-delivery-banner.png",
    mimeType: "image/png",
    buffer: deterministicBannerPng,
  });
  await expect(page.getByText("Selected: learner-delivery-banner.png")).toBeVisible();
  await page.getByRole("button", { name: "Save appearance" }).click();
  const appearanceSaved = page.getByText("Course appearance saved.");
  await expect(appearanceSaved).toBeVisible();
  await expect(page.locator("img.course-appearance-banner")).toHaveCount(2);
  const savedBannerPreview = page.locator("figure.course-appearance-preview").first();
  await assertLoadedSameOriginBlobImage(
    page,
    savedBannerPreview.locator("img.course-appearance-banner"),
  );
  await captureVisibleState(
    page,
    scenarioInput,
    "learner_delivery_appearance_saved",
    appearanceSaved,
  );
  await page.reload();
  await expect(page.locator("[data-route-surface=courseAppearance]")).toBeVisible();
  await expect(page.getByRole("radio", { name: "Forest", exact: true })).toBeChecked();
  await expect(page.locator("img.course-appearance-banner")).toHaveCount(2);
  const renderedBannerPreview = page.locator("figure.course-appearance-preview").first();
  const renderedBanner = renderedBannerPreview.locator("img.course-appearance-banner");
  await assertLoadedSameOriginBlobImage(page, renderedBanner);

  await openCourseAssignments(page);
  const assignmentCard = page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: assignmentTitle, exact: true }) });
  await expect(assignmentCard).toHaveCount(1);
  await assignmentCard.getByRole("link", { name: "Access and modifiers" }).click();
  await expect(page.locator("[data-route-surface=assignmentAccess]")).toBeVisible();
  const learner = page.getByRole("combobox", { name: "Learner", exact: true });
  await learner.selectOption({ label: "Mary Okafor" });
  const allowedPreview = page.getByRole("region", { name: "Resolved learner preview" });
  await expect(allowedPreview).toBeVisible();
  await expect(allowedPreview).toContainText("Course time zone:");
  await expect(page.getByText("This learner is not entitled to this assignment.")).toHaveCount(0);
  await captureVisibleState(
    page,
    scenarioInput,
    "learner_delivery_access_preview_allowed",
    allowedPreview,
  );

  await page.getByRole("link", { name: "Edit assignment", exact: true }).click();
  await expect(page.locator("[data-route-surface=assignmentEditor]")).toBeVisible();
  await page.getByLabel("Lifecycle").selectOption("archived");
  await page.getByRole("button", { name: "Save teaching operations" }).click();
  await expect(page.getByTestId("assignment-current-state")).toHaveText(
    "Archived. Students cannot access this assignment.",
  );

  await openCourseAssignments(page);
  const retiredCard = page
    .getByRole("article")
    .filter({ has: page.getByRole("heading", { name: assignmentTitle, exact: true }) });
  await retiredCard.getByRole("link", { name: "Access and modifiers" }).click();
  await expect(page.locator("[data-route-surface=assignmentAccess]")).toBeVisible();
  await page
    .getByRole("combobox", { name: "Learner", exact: true })
    .selectOption({ label: "Mary Okafor" });
  const deniedPreview = page.getByText("This learner is not entitled to this assignment.");
  await expect(deniedPreview).toBeVisible();
  await captureVisibleState(
    page,
    scenarioInput,
    "learner_delivery_access_preview_denied",
    deniedPreview,
  );
}

test.describe.configure({ mode: "serial" });

test("learner delivery: Mary completes and revisits an instructor-created assignment", async ({
  browser,
}) => {
  test.setTimeout(journeyTimeoutMs);
  const scenarioInput = requireScenarioInput(configuredLiveDemoInputs);
  expect(scenarioInput.scenarioId).toBe("learner_delivery");
  const tag = scenarioInput.namespace;
  const questionTitle = `Learner delivery question ${tag}`;
  const correctChoice = `Supported learner choice ${tag}`;
  const courseTitle = `Learner delivery course ${tag}`;
  const assignmentTitle = `Learner delivery assignment ${tag}`;
  const pageOrigins = new Set<string>();
  const requestOrigins = new Set<string>();
  const expectedOrigin = new URL(scenarioInput.baseUrl).origin;
  const contexts: BrowserContext[] = [];

  try {
    const elenaContext = await browser.newContext(contextOptions);
    const maryContext = await browser.newContext(contextOptions);
    contexts.push(elenaContext, maryContext);
    for (const context of contexts) {
      configureContext(context);
      observeContextOrigins(context, pageOrigins, requestOrigins);
    }
    const elena = await elenaContext.newPage();
    const mary = await maryContext.newPage();
    configurePage(elena);
    configurePage(mary);

    await chooseSeededIdentity(elena, /Elena Rivera/u);
    await selectVisibleCourse(elena, "Biochemistry Base Course");
    const questionId = await createPublishedQuestion(elena, questionTitle, correctChoice);
    const invitationUrl = await createPublishedCourseAssignment(
      elena,
      courseTitle,
      assignmentTitle,
      questionId,
      tag,
    );
    await claimCourseAndCompleteAssignment(
      mary,
      invitationUrl,
      courseTitle,
      assignmentTitle,
      correctChoice,
      scenarioInput,
    );
    await signOutVisible(mary);
    await maryContext.close();
    contexts.splice(contexts.indexOf(maryContext), 1);

    const freshMaryContext = await browser.newContext(contextOptions);
    contexts.push(freshMaryContext);
    configureContext(freshMaryContext);
    observeContextOrigins(freshMaryContext, pageOrigins, requestOrigins);
    expect(await freshMaryContext.storageState()).toEqual({ cookies: [], origins: [] });
    const freshMary = await freshMaryContext.newPage();
    configurePage(freshMary);
    await observeCompletedRunInFreshSession(freshMary, courseTitle, assignmentTitle, scenarioInput);
    await signOutVisible(freshMary);
    await freshMaryContext.close();
    contexts.splice(contexts.indexOf(freshMaryContext), 1);

    await observeInstructorOutcomesAndAccess(elena, assignmentTitle, scenarioInput);

    expect(pageOrigins.size).toBeGreaterThan(0);
    expect(requestOrigins.size).toBeGreaterThan(0);
    expect([...pageOrigins]).toEqual([expectedOrigin]);
    expect([...requestOrigins]).toEqual([expectedOrigin]);
    writeOriginReceipt(pageOrigins, requestOrigins);
  } finally {
    await Promise.all(contexts.map(async (context) => context.close()));
  }
});
