// chapter_one_run.spec.ts - opt-in real-browser acceptance for the first teaching corpus.

import { configuredLiveWebworkInputs } from "../../playwright.config";

import { expect, test, type Locator, type Page } from "@playwright/test";

import { tabTo, tabToTargetThroughVisiblePagination } from "./simulator/keyboard_walkthrough";
import {
  answerAndSubmitVisibleQuestion,
  continueFromVisibleFeedback,
} from "./simulator/chapter_question_responses";
import { catalogResultByQuestionId } from "./simulator/instructor_catalog_binding";
import { credentialFromValidatedFile } from "./ui_walkthrough_config_factory";

test.describe.configure({ mode: "serial" });

interface ChapterJourney {
  readonly course: string;
  readonly assignment: string;
  readonly questions: ReadonlyArray<{ readonly title: string; readonly matching: boolean }>;
}

const CHAPTERS: ReadonlyArray<ChapterJourney> = [
  {
    course: "Genetics Fall 2026 pilot",
    assignment: "Genetics Chapter 1 Mastery",
    questions: [
      { title: "Genetic disorders: Which one?", matching: false },
      { title: "Genetic disorders: Matching", matching: true },
      { title: "Genetics Chapter 1: Phenylalanine metabolism", matching: false },
      { title: "Genetics Chapter 1: Genetic disorder matching", matching: true },
    ],
  },
  {
    course: "Biochemistry Fall 2026 pilot",
    assignment: "Biochemistry Chapter 1 Mastery",
    questions: [
      { title: "Biochemical functional groups: Which one?", matching: false },
      { title: "Biochemical functional groups: Matching", matching: true },
      { title: "Biochemistry Chapter 1: Charged functional groups", matching: false },
      { title: "Biochemistry Chapter 1: Functional group matching", matching: true },
    ],
  },
];

async function activateWithKeyboard(page: Page, control: Locator): Promise<void> {
  await control.focus();
  await expect(control).toBeFocused();
  await page.keyboard.press("Space");
}

function liveInputs(): {
  readonly baseUrl: string;
  readonly studentCredential: string;
  readonly instructorCredential: string;
} {
  if (configuredLiveWebworkInputs !== undefined) {
    return {
      baseUrl: configuredLiveWebworkInputs.baseUrl,
      studentCredential: configuredLiveWebworkInputs.studentCredential,
      instructorCredential: credentialFromValidatedFile(
        configuredLiveWebworkInputs.credentialFile,
        "instructor",
      ),
    };
  }
  throw new Error("live Chapter 1 inputs are unavailable");
}

async function signIn(
  page: Page,
  credentialValue: string,
  workspaceHeading: "Courses you teach" | "Pick up where you left off",
): Promise<void> {
  const inputs = liveInputs();
  await page.goto(inputs.baseUrl);
  const credential = page.getByLabel("Local development credential");
  await expect(credential).toBeVisible();
  await tabTo(page, credential);
  await expect(credential).toBeFocused();
  await credential.fill(credentialValue);
  const signIn = page.getByRole("button", { name: "Sign in locally", exact: true });
  await expect(signIn).toBeVisible();
  await tabTo(page, signIn);
  await expect(signIn).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.locator("[data-route-surface=courses]")).toBeVisible();
  await expect(page.getByRole("heading", { name: workspaceHeading })).toBeVisible();
}

async function openCourseFromDashboard(page: Page, courseTitle: string): Promise<void> {
  const course = page.locator(".course-card").filter({
    has: page.getByRole("heading", { name: courseTitle }),
  });
  const courseLink = course.getByRole("link", { name: "Open course" });
  await tabToTargetThroughVisiblePagination(page, {
    target: course,
    keyboardTarget: courseLink,
    renderedItems: page.locator(".course-card"),
    firstAppendedControl: (index) =>
      page.locator(".course-card").nth(index).getByRole("link", { name: "Open course" }),
    itemName: "courses",
  });
  await expect(course).toHaveCount(1);
  await page.keyboard.press("Enter");
  await expect(page.locator("[data-route-surface=courseAssignments]")).toBeVisible({
    timeout: 30_000,
  });
}

async function openLibrary(page: Page): Promise<void> {
  const library = page.getByRole("link", { name: "Library", exact: true });
  await tabTo(page, library);
  await page.keyboard.press("Enter");
  await expect(page.locator("[data-route-surface=library]")).toBeVisible();
}

async function searchLibrary(page: Page, search: string): Promise<void> {
  const input = page.getByLabel("Search published questions");
  await tabTo(page, input);
  await input.fill(search);
}

async function openLibraryQuestion(page: Page, title: string): Promise<string> {
  const row = page.locator(".catalog-row", {
    has: page.getByRole("heading", { name: title, exact: true }),
  });
  await expect(row).toBeVisible();
  const questionId = row.locator("code");
  await expect(questionId).toHaveText(/^[0-9A-HJKMNP-TV-Z]{3}-[0-9A-HJKMNP-TV-Z]{4}$/u);
  const displayId = await questionId.innerText();
  const open = row.getByRole("link", { name: "Open question", exact: true });
  await tabTo(page, open);
  await page.keyboard.press("Enter");
  await expect(page.locator("[data-route-surface=problemDetail]")).toBeVisible();
  return displayId;
}

async function completeChapter(page: Page, chapter: ChapterJourney): Promise<void> {
  await openCourseFromDashboard(page, chapter.course);

  const assignment = page.locator(".course-card").filter({
    has: page.getByRole("heading", { name: chapter.assignment }),
  });
  const assignmentLink = assignment.getByRole("link", { name: "Start assignment" });
  await tabToTargetThroughVisiblePagination(page, {
    target: assignment,
    keyboardTarget: assignmentLink,
    renderedItems: page.locator(".course-card"),
    firstAppendedControl: (index) =>
      page.locator(".course-card").nth(index).getByRole("link", { name: "Start assignment" }),
    itemName: "assignments",
  });
  await expect(assignment).toContainText("4 questions in each new run");
  await page.keyboard.press("Enter");
  await expect(page.getByRole("heading", { name: chapter.assignment })).toBeVisible();
  await expect(page.getByText("4", { exact: true })).toBeVisible();
  const runResponse = page.waitForResponse(
    (response) =>
      new URL(response.url()).pathname === "/api/runs" && response.request().method() === "POST",
  );
  await activateWithKeyboard(
    page,
    page.getByRole("button", { name: "Start or continue practice" }),
  );
  const started = await runResponse;
  if (started.status() !== 201) {
    await expect(page.getByRole("alert")).toContainText("Could not open the practice run");
    throw new Error(`Chapter 1 run start returned HTTP ${started.status()}`);
  }

  for (const [index, question] of chapter.questions.entries()) {
    await expect(page.getByRole("heading", { name: question.title })).toBeVisible();
    expect(await answerAndSubmitVisibleQuestion(page)).toBe(
      question.matching ? "matching" : "multiple-choice",
    );
    await continueFromVisibleFeedback(page);
    if (index < chapter.questions.length - 1) {
      await expect(
        page.getByRole("heading", { name: chapter.questions[index + 1]?.title }),
      ).toBeVisible();
    }
  }

  const summary = page.locator(".attempt-summary");
  await expect(
    summary.getByRole("heading", { name: "Keep practicing with a fresh variation" }),
  ).toBeVisible();
  await expect(summary.getByRole("button", { name: "Start another practice" })).toBeVisible();
}

/**
 * Selector contract: the instructor assignment overview exposes an accessible Edit assignment
 * link, and assignment_editor_page.tsx exposes Assignment content, Replace, Replacement Question
 * ID, Check Question ID, Replace with selected question, Reload assignment, and its live status.
 */
async function openAssignmentEditor(page: Page, chapter: ChapterJourney): Promise<void> {
  await openCourseFromDashboard(page, chapter.course);
  const assignment = page.locator(".course-card").filter({
    has: page.getByRole("heading", { name: chapter.assignment }),
  });
  const review = assignment.getByRole("link", { name: "Start assignment", exact: true });
  await expect(review).toBeVisible();
  await tabTo(page, review);
  await page.keyboard.press("Enter");
  await expect(page.locator("[data-route-surface=assignmentOverview]")).toBeVisible();
  const edit = page.getByRole("link", { name: "Edit assignment", exact: true });
  await expect(edit).toBeVisible();
  await tabTo(page, edit);
  await page.keyboard.press("Enter");
  await expect(page.locator("[data-route-surface=assignmentEditor]")).toBeVisible();
  await expect(page.getByRole("heading", { name: "Assignment editor", exact: true })).toBeVisible();
}

async function selectDifferentPublishedQuestion(
  page: Page,
  originalQuestionId: string,
): Promise<string> {
  const search = page.getByLabel("Search published questions");
  await tabTo(page, search);
  await search.fill("Genetics Chapter 1");
  const searchButton = page.getByRole("button", { name: "Search library", exact: true });
  await tabTo(page, searchButton);
  await page.keyboard.press("Enter");
  const choices = page.locator("article.assignment-editor-row");
  await expect(choices.first()).toBeVisible();
  for (let index = 0; index < (await choices.count()); index += 1) {
    const choice = choices.nth(index);
    const candidate = await choice.locator("code").innerText();
    if (candidate === originalQuestionId) continue;
    const choose = choice.getByRole("button", { name: "Use this Question ID", exact: true });
    await tabTo(page, choose);
    await page.keyboard.press("Enter");
    await expect(
      page.getByRole("textbox", { name: "Replacement Question ID", exact: true }),
    ).toHaveValue(candidate);
    return candidate;
  }
  throw new Error("The real Chapter 1 catalog needs a replacement Question ID.");
}

async function prepareReplacement(page: Page, originalQuestionId: string): Promise<string> {
  await page.bringToFront();
  const replace = page.getByRole("button", { name: "Replace", exact: true }).first();
  // The route shell is visible before its asynchronous assignment projection has rendered.
  // Wait for the enabled, visible control before beginning the bounded native-tab traversal;
  // otherwise the traversal can finish while there is no target in the document.
  await expect(replace).toBeVisible();
  await expect(replace).toBeEnabled();
  await tabTo(page, replace);
  await page.keyboard.press("Enter");
  await expect(
    page.getByRole("heading", { name: "Replace assigned question", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText(
      "Future runs use the replacement. Already issued work stays with its original question.",
    ),
  ).toBeVisible();
  const replacementQuestionId = await selectDifferentPublishedQuestion(page, originalQuestionId);
  const check = page.getByRole("button", { name: "Check Question ID", exact: true });
  await tabTo(page, check, "backward");
  await page.keyboard.press("Enter");
  await expect(
    page.getByText(`Selected: ${replacementQuestionId}`, { exact: false }),
  ).toBeVisible();
  return replacementQuestionId;
}

async function issueRunAndReadFirstQuestion(page: Page, chapter: ChapterJourney): Promise<string> {
  await openCourseFromDashboard(page, chapter.course);
  const assignment = page.locator(".course-card").filter({
    has: page.getByRole("heading", { name: chapter.assignment }),
  });
  const review = assignment.getByRole("link", { name: "Start assignment", exact: true });
  await expect(review).toBeVisible();
  await tabTo(page, review);
  await page.keyboard.press("Enter");
  const openedRun = page.waitForResponse(
    (response) =>
      new URL(response.url()).pathname === "/api/runs" && response.request().method() === "POST",
  );
  const start = page.getByRole("button", { name: "Start or continue practice", exact: true });
  await activateWithKeyboard(page, start);
  expect((await openedRun).status()).toBe(201);
  const question = chapter.questions[0];
  if (question === undefined) throw new Error("Chapter 1 needs one issued question.");
  await expect(page.getByRole("heading", { name: question.title, exact: true })).toBeVisible();
  return question.title;
}

function assignmentEditorStatus(page: Page): Locator {
  return page.locator('[data-route-surface="assignmentEditor"] > p[role="status"]');
}

test.describe("private live Chapter 1 browser acceptance", () => {
  test.skip(
    configuredLiveWebworkInputs === undefined,
    "requires the explicit private live-stack invocation",
  );

  test("instructor sees a seeded Chapter 1 question under its human identity", async ({ page }) => {
    const inputs = liveInputs();

    await signIn(page, inputs.instructorCredential, "Courses you teach");
    await openCourseFromDashboard(page, CHAPTERS[0]?.course ?? "Genetics Fall 2026 pilot");
    const newAssignment = page.getByRole("link", { name: "New assignment", exact: true });
    await expect(newAssignment).toBeVisible();
    await tabTo(page, newAssignment);
    await page.keyboard.press("Enter");
    await expect(
      page.getByRole("heading", { name: "Create assignment", exact: true }),
    ).toBeVisible();
    const title = "Genetic disorders: Matching";
    const search = page.getByLabel("Search published questions");
    await tabTo(page, search);
    await search.fill(title);
    const searchLibrary = page.getByRole("button", { name: "Search library", exact: true });
    await tabTo(page, searchLibrary);
    await page.keyboard.press("Enter");
    const catalogRow = await catalogResultByQuestionId(page, title);
    const humanReference = catalogRow.locator("code");
    await expect(humanReference).toHaveText(/^[0-9A-HJKMNP-TV-Z]{3}-[0-9A-HJKMNP-TV-Z]{4}$/u);
    const displayId = await humanReference.innerText();
    const copyId = catalogRow.getByRole("button", { name: `Copy question ID ${displayId}` });
    await page.context().grantPermissions(["clipboard-read", "clipboard-write"], {
      origin: new URL(inputs.baseUrl).origin,
    });
    await tabTo(page, copyId);
    await page.keyboard.press("Enter");
    await expect(catalogRow.getByRole("status")).toHaveText(`Copied ${displayId}.`);
    const addSeveralQuestionIds = page.getByText("Add several Question IDs", { exact: true });
    await expect(addSeveralQuestionIds).toBeVisible();
    await tabTo(page, addSeveralQuestionIds);
    await page.keyboard.press("Enter");
    const questionIds = page.getByLabel("Question IDs");
    await expect(questionIds).toBeVisible();
    await tabTo(page, questionIds);
    await page.keyboard.press("ControlOrMeta+V");
    await expect(questionIds).toHaveValue(displayId);
    const addById = page.getByRole("button", { name: "Add questions by ID" });
    await tabTo(page, addById);
    await page.keyboard.press("Enter");
    await expect(assignmentEditorStatus(page)).toHaveText(
      `Added ${displayId} to the unsaved selection.`,
    );
    const addedRow = page.locator(".assignment-editor-list .assignment-editor-row", {
      has: page.getByRole("heading", { name: title, exact: true }),
    });
    await expect(addedRow).toHaveCount(1);
    await expect(addedRow.locator("code")).toHaveText(displayId);
    await expect(addedRow.getByRole("heading", { name: title, exact: true })).toBeVisible();
    await expect(addedRow.locator("p")).toContainText("WeBWorK");
  });

  test("instructor discovers the real Chapter 1 library through concepts, a typo, and disclosed evidence", async ({
    page,
  }) => {
    const inputs = liveInputs();

    await signIn(page, inputs.instructorCredential, "Courses you teach");
    await openLibrary(page);
    await searchLibrary(page, "genetic disorder");
    await expect(
      page.getByRole("heading", { name: "Genetic disorders: Which one?", exact: true }),
    ).toBeVisible();
    await expect(
      page.getByRole("heading", {
        name: "Genetics Chapter 1: Phenylalanine metabolism",
        exact: true,
      }),
    ).toBeVisible();

    await searchLibrary(page, "phenylalnine");
    await expect(
      page.getByRole("heading", { name: "Genetic disorders: Which one?", exact: true }),
    ).not.toBeVisible();
    await expect(
      page.getByRole("heading", {
        name: "Genetics Chapter 1: Phenylalanine metabolism",
        exact: true,
      }),
    ).toBeVisible();

    await searchLibrary(page, "genetic disorder");
    const evidence = page.getByLabel("Evidence");
    const disclosedEvidenceOption = evidence.locator('option[value="available"]');
    await expect(disclosedEvidenceOption).toHaveCount(1);
    await expect(disclosedEvidenceOption).toHaveText("Has disclosed evidence (1)");
    await tabTo(page, evidence);
    await evidence.selectOption("available");
    await expect(evidence).toHaveValue("available");
    await expect(
      page.getByRole("heading", { name: "Genetic disorders: Which one?", exact: true }),
    ).not.toBeVisible();
    const availableId = await openLibraryQuestion(
      page,
      "Genetics Chapter 1: Phenylalanine metabolism",
    );
    await expect(page.locator(".catalog-statistics-panel")).toContainText(
      "Anonymous learning evidence",
    );
    await expect(page.locator(".catalog-statistics-panel")).toContainText("Cohort size");
    await expect(page.locator(".catalog-statistics-panel")).toContainText("learners");
    await expect(page.locator("body")).not.toContainText(
      /tenant|student|response|answer key|source|grading/iu,
    );
    await expect(page.locator("body")).not.toContainText(
      /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/iu,
    );
    await expect(page.getByText(availableId, { exact: true })).toBeVisible();

    const returnToLibrary = page.getByRole("link", {
      name: "Return to problem library",
      exact: true,
    });
    await tabTo(page, returnToLibrary);
    await page.keyboard.press("Enter");
    await expect(page.locator("[data-route-surface=library]")).toBeVisible();
    await evidence.selectOption("");
    await expect(evidence).toHaveValue("");
    const unavailableId = await openLibraryQuestion(page, "Genetic disorders: Which one?");
    await expect(page.locator(".catalog-statistics-panel")).toContainText("Insufficient evidence");
    await expect(page.locator("body")).not.toContainText(
      /tenant|student|response|answer key|source|grading/iu,
    );
    await expect(page.locator("body")).not.toContainText(
      /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/iu,
    );
    await expect(page.getByText(unavailableId, { exact: true })).toBeVisible();
  });

  test("Question-ID replacement preserves an issued learner run and recovers truthfully from a stale editor", async ({
    page,
  }) => {
    test.setTimeout(120_000);
    const inputs = liveInputs();
    const chapter = CHAPTERS[0];
    if (chapter === undefined) throw new Error("Chapter 1 journey is unavailable.");
    const browser = page.context().browser();
    if (browser === null) throw new Error("Live browser is unavailable.");
    const studentContext = await browser.newContext();
    const replacingContext = await browser.newContext();
    const studentPage = await studentContext.newPage();
    const replacingPage = await replacingContext.newPage();
    try {
      await studentPage.bringToFront();
      await signIn(studentPage, inputs.studentCredential, "Pick up where you left off");
      const issuedQuestionTitle = await issueRunAndReadFirstQuestion(studentPage, chapter);

      await page.bringToFront();
      await signIn(page, inputs.instructorCredential, "Courses you teach");
      await openAssignmentEditor(page, chapter);
      const originalQuestionId = await page
        .locator(".assignment-editor-list")
        .getByRole("listitem")
        .first()
        .locator("code")
        .innerText();
      await expect(page.locator(".assignment-editor-list")).toContainText(originalQuestionId);

      await replacingPage.bringToFront();
      await signIn(replacingPage, inputs.instructorCredential, "Courses you teach");
      await openAssignmentEditor(replacingPage, chapter);
      const replacementQuestionId = await prepareReplacement(replacingPage, originalQuestionId);
      const replacementRequest = replacingPage.waitForRequest(
        (request) =>
          request.method() === "PUT" &&
          request.url().includes("/question") &&
          request.postData() === JSON.stringify({ questionId: replacementQuestionId }),
      );
      const replace = replacingPage.getByRole("button", {
        name: "Replace with selected question",
        exact: true,
      });
      await tabTo(replacingPage, replace);
      await replacingPage.keyboard.press("Enter");
      const sent = await replacementRequest;
      expect(JSON.parse(sent.postData() ?? "{}")).toEqual({ questionId: replacementQuestionId });
      await expect(assignmentEditorStatus(replacingPage)).toHaveText(
        "Replacement saved. Future runs use the replacement; issued work stays with its original question.",
      );
      await expect(replacingPage.locator(".assignment-editor-list")).toContainText(
        replacementQuestionId,
      );
      await expect(replacingPage.locator("body")).not.toContainText(
        /problemId|versionId|[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/iu,
      );

      await page.bringToFront();
      await prepareReplacement(page, originalQuestionId);
      const staleQuestionId = await page
        .getByRole("textbox", { name: "Replacement Question ID", exact: true })
        .inputValue();
      const staleReplace = page.getByRole("button", {
        name: "Replace with selected question",
        exact: true,
      });
      await tabTo(page, staleReplace);
      await page.keyboard.press("Enter");
      await expect(page.getByRole("alert")).toContainText(
        "A newer assignment revision is available.",
      );
      await expect(
        page.getByRole("textbox", { name: "Replacement Question ID", exact: true }),
      ).toHaveValue(staleQuestionId);
      const reload = page.getByRole("button", { name: "Reload assignment", exact: true });
      await tabTo(page, reload);
      await page.keyboard.press("Enter");
      await expect(page.locator(".assignment-editor-list")).toContainText(replacementQuestionId);

      await studentPage.bringToFront();
      await studentPage.goto(inputs.baseUrl);
      await expect(
        studentPage.getByRole("heading", { name: "Pick up where you left off" }),
      ).toBeVisible();
      expect(await issueRunAndReadFirstQuestion(studentPage, chapter)).toBe(issuedQuestionTitle);
      await expect(studentPage.locator("body")).not.toContainText(
        /problemId|versionId|[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/iu,
      );
    } finally {
      await replacingContext.close();
      await studentContext.close();
    }
  });

  test("student completes the selected exact four-question chapters with visible keyboard controls", async ({
    page,
  }) => {
    test.setTimeout(120_000);
    const inputs = liveInputs();

    await signIn(page, inputs.studentCredential, "Pick up where you left off");
    for (const chapter of CHAPTERS) {
      await completeChapter(page, chapter);
      await page.goto(inputs.baseUrl);
      await expect(page.getByRole("heading", { name: "Pick up where you left off" })).toBeVisible();
    }
  });
});
