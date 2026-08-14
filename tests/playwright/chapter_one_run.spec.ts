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
  await tabTo(page, credential);
  await expect(credential).toBeFocused();
  await credential.fill(credentialValue);
  const signIn = page.getByRole("button", { name: "Sign in locally", exact: true });
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

async function completeChapter(page: Page, chapter: ChapterJourney): Promise<void> {
  await openCourseFromDashboard(page, chapter.course);

  const assignment = page.locator(".course-card").filter({
    has: page.getByRole("heading", { name: chapter.assignment }),
  });
  const assignmentLink = assignment.getByRole("link", { name: "Review assignment" });
  await tabToTargetThroughVisiblePagination(page, {
    target: assignment,
    keyboardTarget: assignmentLink,
    renderedItems: page.locator(".course-card"),
    firstAppendedControl: (index) =>
      page.locator(".course-card").nth(index).getByRole("link", { name: "Review assignment" }),
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
  await activateWithKeyboard(page, page.getByRole("button", { name: "Start or resume practice" }));
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
  await expect(summary.getByRole("button", { name: "Start another practice run" })).toBeVisible();
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
    const searchCatalog = page.getByRole("button", { name: "Search catalog", exact: true });
    await tabTo(page, searchCatalog);
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
    const addByQuestionId = page.getByText("Add by question ID", { exact: true });
    await expect(addByQuestionId).toBeVisible();
    await tabTo(page, addByQuestionId);
    await page.keyboard.press("Enter");
    const questionIds = page.getByLabel("Question IDs");
    await expect(questionIds).toBeVisible();
    await tabTo(page, questionIds);
    await page.keyboard.press("ControlOrMeta+V");
    await expect(questionIds).toHaveValue(displayId);
    const addById = page.getByRole("button", { name: "Add questions by ID" });
    await tabTo(page, addById);
    await page.keyboard.press("Enter");
    await expect(page.locator(".assignment-editor-import-success")).toHaveText(
      `Added ${displayId} to the unsaved selection.`,
    );
    await expect(page.locator(".assignment-editor-list")).toContainText(`${displayId}WeBWorK`);
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
