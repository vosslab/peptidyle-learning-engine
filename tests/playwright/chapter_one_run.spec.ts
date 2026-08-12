// chapter_one_run.spec.ts - opt-in real-browser acceptance for the first teaching corpus.

import {
  configuredLiveWebworkInputs,
  configuredUiWalkthroughInputs,
} from "../../playwright.config";

import { expect, test, type Locator, type Page } from "@playwright/test";

import { tabTo, tabToTargetThroughVisiblePagination } from "./simulator/keyboard_walkthrough";
import { exactCatalogResult } from "./simulator/instructor_catalog_binding";
import {
  instructorCredentialFromValidatedFile,
  studentCredentialFromValidatedFile,
} from "./ui_walkthrough_live_config";

test.describe.configure({ mode: "serial" });

interface ChapterJourney {
  readonly course: string;
  readonly assignment: string;
  readonly questions: ReadonlyArray<{ readonly title: string; readonly matching: boolean }>;
}

const CHAPTERS: ReadonlyArray<ChapterJourney> = [
  {
    course: "Genetics Fall 2026 pilot",
    assignment: "Genetics Chapter 1 Practice",
    questions: [
      { title: "Genetic disorders: Which one?", matching: false },
      { title: "Genetic disorders: Matching", matching: true },
      { title: "Genetics Chapter 1: Phenylalanine metabolism", matching: false },
      { title: "Genetics Chapter 1: Genetic disorder matching", matching: true },
    ],
  },
  {
    course: "Biochemistry Fall 2026 pilot",
    assignment: "Biochemistry Chapter 1 Practice",
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
      instructorCredential: instructorCredentialFromValidatedFile(
        configuredLiveWebworkInputs.credentialFile,
      ),
    };
  }
  if (configuredUiWalkthroughInputs !== undefined) {
    return {
      baseUrl: configuredUiWalkthroughInputs.baseUrl,
      studentCredential: studentCredentialFromValidatedFile(
        configuredUiWalkthroughInputs.credentialFile,
      ),
      instructorCredential: instructorCredentialFromValidatedFile(
        configuredUiWalkthroughInputs.credentialFile,
      ),
    };
  }
  throw new Error("live Chapter 1 inputs are unavailable");
}

function selectedChapters(): ReadonlyArray<ChapterJourney> {
  const scope = process.env["PLE_CHAPTER_ONE_BROWSER_SCOPE"] ?? "all";
  if (scope === "all") return CHAPTERS;
  if (scope === "genetics") return CHAPTERS.slice(0, 1);
  throw new Error("PLE_CHAPTER_ONE_BROWSER_SCOPE must be exactly all or genetics when set");
}

async function signIn(page: Page, credentialValue: string): Promise<void> {
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
  await expect(page.getByRole("heading", { name: "Pick up where you left off" })).toBeVisible();
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
}

async function choosePlausibleMultipleChoice(page: Page): Promise<void> {
  const radios = page.getByRole("radio");
  await expect(radios).toHaveCount(4);
  await activateWithKeyboard(page, radios.first());
  await expect(radios.first()).toBeChecked();
}

async function chooseOneDistinctMatchPerPrompt(page: Page): Promise<void> {
  const groups = page.locator(".matching-group");
  await expect(groups).toHaveCount(4);
  const selectedValues = new Set<string>();
  for (let index = 0; index < 4; index += 1) {
    const choices = groups.nth(index).getByRole("radio");
    const count = await choices.count();
    expect(count).toBeGreaterThanOrEqual(4);
    const choice = choices.nth(index);
    const value = await choice.getAttribute("value");
    expect(value).not.toBeNull();
    selectedValues.add(value ?? "");
    await activateWithKeyboard(page, choice);
    await expect(choice).toBeChecked();
  }
  expect(selectedValues.size).toBe(4);
}

async function answerCurrentQuestion(page: Page, matching: boolean): Promise<void> {
  if (matching) await chooseOneDistinctMatchPerPrompt(page);
  else await choosePlausibleMultipleChoice(page);
  await expect(page.getByRole("status", { name: "Response format" })).toContainText(
    "ready to submit",
  );
  const submissionResponse = page.waitForResponse(
    (response) =>
      new URL(response.url()).pathname.startsWith("/api/submissions/") &&
      response.request().method() === "POST",
  );
  await activateWithKeyboard(page, page.getByRole("button", { name: "Submit answer" }));
  const submitted = await submissionResponse;
  if (!submitted.ok()) {
    await expect(page.getByRole("button", { name: "Retry saved response" })).toBeVisible();
    const detail = await Promise.race([
      submitted.text(),
      page.waitForTimeout(1_000).then(() => "response body unavailable"),
    ]);
    throw new Error(`Chapter 1 submission returned HTTP ${submitted.status()}: ${detail}`);
  }
  const feedback = page.getByRole("region", { name: "Feedback" });
  await expect(feedback.getByRole("heading", { name: "Feedback" })).toBeVisible();
  await expect(feedback.getByRole("heading", { name: "Your response" })).toBeVisible();
  await expect(feedback.getByRole("status")).toContainText(
    /Feedback released|Your response was recorded/u,
  );
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
    await answerCurrentQuestion(page, question.matching);
    await activateWithKeyboard(page, page.getByRole("button", { name: "Continue" }));
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
    configuredLiveWebworkInputs === undefined && configuredUiWalkthroughInputs === undefined,
    "requires the explicit private live-stack invocation",
  );

  test("instructor sees a seeded Chapter 1 question under its human identity", async ({ page }) => {
    const inputs = liveInputs();

    await signIn(page, inputs.instructorCredential);
    await openCourseFromDashboard(page, CHAPTERS[0]?.course ?? "Genetics Fall 2026 pilot");
    const newAssignment = page.getByRole("link", { name: "New assignment", exact: true });
    await tabTo(page, newAssignment);
    await page.keyboard.press("Enter");
    await expect(
      page.getByRole("heading", { name: "Create assignment", exact: true }),
    ).toBeVisible();
    const title = "Genetic disorders: Matching";
    const search = page.getByLabel("Search published problems");
    await tabTo(page, search);
    await search.fill(title);
    const searchCatalog = page.getByRole("button", { name: "Search catalog", exact: true });
    await tabTo(page, searchCatalog);
    await page.keyboard.press("Enter");
    const catalogRow = await exactCatalogResult(page, title);
    await expect(catalogRow.locator("p[data-problem-id][data-version-id]")).toHaveText(
      /^P-[1-9][0-9]*-v1 · WeBWorK$/u,
    );
  });

  test("student completes the selected exact four-question chapters with visible keyboard controls", async ({
    page,
  }) => {
    test.setTimeout(120_000);
    const inputs = liveInputs();

    await signIn(page, inputs.studentCredential);
    for (const chapter of selectedChapters()) {
      await completeChapter(page, chapter);
      await page.goto(inputs.baseUrl);
      await expect(page.getByRole("heading", { name: "Pick up where you left off" })).toBeVisible();
    }
  });
});
