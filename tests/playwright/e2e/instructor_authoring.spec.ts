// Production-stack instructor authoring journey. Product state is created through visible PLE UI.
import { expect, test, type BrowserContext, type Locator, type Page } from "@playwright/test";
import { writeFileSync } from "node:fs";

import { configuredLiveDemoInputs } from "../../../playwright.config";
import { liveDemoOriginReceiptPathFromEnvironment } from "../browser_suite_live_config";
import {
  chooseSeededIdentity,
  observeContextOrigins,
  requireScenarioInput,
  restoreViewportOrigin,
  selectVisibleCourse,
} from "./real_stack_ui";
import { captureRealStackScreenshot } from "./real_stack_screenshot_capture";

const maryEmail = "mary.okafor@live-demo.ple.example";
const actionTimeoutMs = 30_000;
const scenarioTimeoutMs = 300_000;

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

function writeOriginReceipt(pageOrigins: Set<string>, requestOrigins: Set<string>): void {
  const receiptPath = liveDemoOriginReceiptPathFromEnvironment(process.env);
  writeFileSync(
    receiptPath,
    JSON.stringify({
      pageOrigins: [...pageOrigins].sort(),
      requestOrigins: [...requestOrigins].sort(),
    }),
    { encoding: "ascii", flag: "wx", mode: 0o600 },
  );
}

async function captureInstructorState(
  page: Page,
  scenarioInput: ReturnType<typeof requireScenarioInput>,
  artifactId: string,
  focus?: Locator,
): Promise<void> {
  await restoreViewportOrigin(page);
  if (focus !== undefined) {
    await expect(focus).toBeVisible();
    await focus.scrollIntoViewIfNeeded();
  }
  await captureRealStackScreenshot(page, scenarioInput, artifactId);
}

test.describe("instructor authoring on the production PLE stack", () => {
  test.skip(
    configuredLiveDemoInputs === undefined,
    "the disposable production browser-suite owner supplies this scenario input",
  );

  test("instructor authoring persists after reload and a fresh Elena session", async ({
    browser,
  }) => {
    test.setTimeout(scenarioTimeoutMs);
    const scenarioInput = requireScenarioInput(configuredLiveDemoInputs);
    expect(scenarioInput.scenarioId).toBe("instructor_authoring");
    const tag = scenarioInput.namespace;
    const questionTitle = `Instructor question ${tag}`;
    const correctChoice = `Correct peptide bond ${tag}`;
    const courseTitle = `Instructor course ${tag}`;
    const assignmentTitle = `Instructor assignment ${tag}`;
    const rosterId = `mary-${tag}`;
    const pageOrigins = new Set<string>();
    const requestOrigins = new Set<string>();
    const contexts: BrowserContext[] = [];

    try {
      const initialContext = await browser.newContext({
        viewport: { width: 1280, height: 800 },
        ignoreHTTPSErrors: true,
      });
      contexts.push(initialContext);
      observeContextOrigins(initialContext, pageOrigins, requestOrigins);
      const elena = await initialContext.newPage();
      configure(initialContext, elena);

      await chooseSeededIdentity(elena, /Elena Rivera/u);
      await selectVisibleCourse(elena, "Biochemistry Base Course");
      const wasmRuntime = elena.getByRole("status", {
        name: "Response tools runtime: WebAssembly",
      });
      await expect(wasmRuntime).toHaveAttribute("data-runtime-mode", "wasm");
      await expect(wasmRuntime).toHaveText("Response tools are running locally in this browser.");
      await elena.getByRole("link", { name: "Workspace" }).click();
      await expect(
        elena.getByRole("heading", { name: "Draft, preview, and publish a learning question" }),
      ).toBeVisible();
      await captureInstructorState(elena, scenarioInput, "instructor_authoring_workspace");
      await elena.getByRole("button", { name: "Create flat question" }).click();
      await expect(elena.getByLabel("Question title")).toBeVisible();
      await captureInstructorState(elena, scenarioInput, "instructor_authoring_question_editor");
      await elena.getByLabel("Question title").fill(questionTitle);
      await elena.getByLabel("Learner-facing prompt").fill(`Which statement is correct? ${tag}`);
      await elena.getByLabel("Question format").selectOption("multipleAnswer");
      await elena.getByLabel("Choice text").nth(0).fill(correctChoice);
      await elena.getByLabel("Choice text").nth(1).fill(`Incorrect choice ${tag}`);
      await elena.getByRole("checkbox", { name: "Correct answer", exact: true }).first().check();
      await elena.getByRole("button", { name: "Save private draft" }).click();
      await expect(elena.getByRole("button", { name: "Review publication changes" })).toBeEnabled();
      const studentPreview = elena
        .getByRole("heading", { name: "Student preview", exact: true })
        .locator("..")
        .locator("article");
      await expect(studentPreview).toContainText(`Which statement is correct? ${tag}`);
      await expect(
        studentPreview.getByRole("checkbox", { name: correctChoice, exact: true }),
      ).toBeVisible();
      await expect(studentPreview).not.toContainText("Correct answer");
      await captureInstructorState(
        elena,
        scenarioInput,
        "instructor_authoring_workspace_draft_saved",
      );
      await elena.getByRole("button", { name: "Review publication changes" }).click();
      await elena.getByLabel("Reviewed public byline").fill("Dr. Elena Rivera");
      await elena.getByRole("button", { name: "Confirm and publish" }).click();
      await expect(elena.getByRole("heading", { name: "Published" })).toBeVisible();
      const publicationResult = elena.getByRole("status").filter({ hasText: questionTitle });
      await expect(publicationResult).toContainText("Question ID:");
      await expect(publicationResult).toContainText("By: Dr. Elena Rivera");
      await captureInstructorState(
        elena,
        scenarioInput,
        "instructor_authoring_publication_success",
      );

      await elena.getByRole("link", { name: "Library", exact: true }).click();
      await expect(
        elena.getByRole("heading", { name: "Question library", exact: true }),
      ).toBeVisible();
      await captureInstructorState(elena, scenarioInput, "instructor_authoring_library");
      await elena.getByLabel("Search published questions").fill(questionTitle);
      const questionCard = elena
        .getByRole("region", { name: "Published questions" })
        .getByText(questionTitle)
        .locator("..");
      await expect(questionCard).toBeVisible();
      const questionId = await questionCard.locator("code").innerText();
      expect(questionId).toMatch(/^[A-Z0-9]{3}-[A-Z0-9]{4}$/u);
      await questionCard.getByRole("link", { name: "Open question", exact: true }).click();
      await expect(elena.getByRole("heading", { name: questionTitle, exact: true })).toBeVisible();
      await expect(elena.getByRole("region", { name: "Problem prompt" })).toContainText(tag);
      await captureInstructorState(elena, scenarioInput, "instructor_authoring_question_detail");

      await elena.getByRole("link", { name: "Return to problem library", exact: true }).click();
      await expect(
        elena.getByRole("heading", { name: "Question library", exact: true }),
      ).toBeVisible();
      await elena.getByRole("link", { name: "Courses" }).click();
      await elena.getByLabel("Course title").fill(courseTitle);
      await elena.getByLabel("Start date").fill(isoDate(-30));
      await elena.getByLabel("End date").fill(isoDate(365));
      await elena.getByLabel("Time zone (IANA)").fill("America/Chicago");
      await elena.getByRole("button", { name: "Create course" }).click();
      const courseCard = elena
        .getByRole("article")
        .filter({ has: elena.getByRole("heading", { name: courseTitle, exact: true }) });
      await expect(courseCard).toHaveCount(1);
      await captureInstructorState(elena, scenarioInput, "instructor_authoring_course_created");
      await courseCard.getByRole("link", { name: "Open course", exact: true }).click();
      await elena.getByRole("link", { name: "Assignments" }).click();
      await expect(elena.getByRole("heading", { name: "Assignments", exact: true })).toBeVisible();
      await expect(elena.getByRole("link", { name: "Create the first assignment" })).toBeVisible();
      await captureInstructorState(elena, scenarioInput, "instructor_authoring_course_assignments");
      await elena.getByRole("link", { name: "Create the first assignment" }).click();
      await expect(
        elena.getByRole("heading", { name: "Create assignment", exact: true }),
      ).toBeVisible();
      await expect(
        elena.getByRole("link", { name: "View learner-facing assignment overview" }),
      ).toHaveCount(0);
      await captureInstructorState(elena, scenarioInput, "instructor_authoring_assignment_create");
      await elena.getByLabel("Assignment title").fill(assignmentTitle);
      await elena.getByText("Add several Question IDs", { exact: true }).click();
      await expect(elena.getByLabel("Question IDs")).toBeVisible();
      await elena.getByLabel("Search published questions").fill(questionTitle);
      await elena.getByRole("button", { name: "Search library", exact: true }).click();
      const catalogQuestion = elena
        .getByRole("article")
        .filter({ has: elena.getByRole("heading", { name: questionTitle, exact: true }) });
      await expect(catalogQuestion).toHaveCount(1);
      await expect(catalogQuestion.locator("code")).toHaveText(questionId);
      await captureInstructorState(
        elena,
        scenarioInput,
        "instructor_authoring_problem_catalog",
        catalogQuestion,
      );
      await elena.getByLabel("Question IDs").fill(questionId);
      await elena.getByRole("button", { name: "Add questions by ID" }).click();
      await elena.getByRole("button", { name: "Create assignment" }).click();
      await expect(elena.getByText(`${assignmentTitle} now appears in this course.`)).toBeVisible();
      await expect(
        elena.getByRole("link", { name: "View learner-facing assignment overview" }),
      ).toBeVisible();
      await captureInstructorState(elena, scenarioInput, "instructor_authoring_assignment_created");
      await elena.getByRole("link", { name: `Open ${assignmentTitle}` }).click();
      await expect(
        elena.getByRole("heading", { name: "Assignment editor", exact: true }),
      ).toBeVisible();
      await expect(elena.getByLabel("Lifecycle")).toHaveValue("draft");
      await captureInstructorState(elena, scenarioInput, "instructor_authoring_assignment_editor");
      await elena.getByLabel("Lifecycle").selectOption("published");
      await elena.getByRole("button", { name: "Save teaching operations" }).click();
      const publishedState = elena.getByTestId("assignment-current-state");
      await expect(publishedState).toHaveText("Published, open now.");
      const publishedResult = elena
        .getByRole("status")
        .filter({ hasText: `${assignmentTitle} is saved.` });
      await expect(publishedResult).toContainText("Published, open now.");
      await expect(elena.getByRole("heading", { name: "Assignment editor" })).toBeVisible();
      await expect(elena.getByLabel("Lifecycle")).toHaveValue("published");
      await captureInstructorState(
        elena,
        scenarioInput,
        "instructor_authoring_assignment_published",
        publishedResult,
      );

      await elena.getByRole("link", { name: "View learner-facing assignment overview" }).click();
      await expect(elena.locator('[data-route-surface="assignmentOverview"]')).toBeVisible();
      await expect(
        elena.getByRole("heading", { name: assignmentTitle, exact: true }),
      ).toBeVisible();
      await expect(
        elena.getByRole("heading", { name: "Delivery details", exact: true }),
      ).toBeVisible();
      await captureInstructorState(
        elena,
        scenarioInput,
        "instructor_authoring_assignment_overview",
      );

      await elena.getByRole("link", { name: "Edit assignment", exact: true }).click();
      await expect(
        elena.getByRole("heading", { name: "Assignment editor", exact: true }),
      ).toBeVisible();
      await elena.getByRole("link", { name: "Assignments", exact: true }).click();
      await elena.getByRole("link", { name: "Students" }).click();
      await elena.getByLabel("Institutional email").fill(maryEmail);
      await elena.getByLabel("Institutional student ID").fill(rosterId);
      await elena.getByRole("button", { name: "Create invitation" }).click();
      await expect(elena.getByLabel("Invitation link")).toBeVisible();
      await expect(elena.getByRole("heading", { name: "Pending invitations" })).toBeVisible();
      await expect(elena.getByRole("row").filter({ hasText: rosterId })).toHaveCount(1);

      await elena.reload();
      await expect(elena.getByRole("heading", { name: "Students" })).toBeVisible();
      await expect(elena.getByRole("row").filter({ hasText: rosterId })).toHaveCount(1);
      await expect(elena.getByLabel("Invitation link")).toHaveCount(0);
      await captureInstructorState(elena, scenarioInput, "instructor_authoring_invitation_pending");

      const freshElenaContext = await browser.newContext({
        viewport: { width: 1280, height: 800 },
        ignoreHTTPSErrors: true,
      });
      contexts.push(freshElenaContext);
      observeContextOrigins(freshElenaContext, pageOrigins, requestOrigins);
      const freshElena = await freshElenaContext.newPage();
      configure(freshElenaContext, freshElena);
      await chooseSeededIdentity(freshElena, /Elena Rivera/u);
      await selectVisibleCourse(freshElena, courseTitle);
      await freshElena.getByRole("link", { name: "Assignments" }).click();
      const assignmentCard = freshElena
        .getByRole("article")
        .filter({ has: freshElena.getByRole("heading", { name: assignmentTitle, exact: true }) });
      await expect(assignmentCard).toHaveCount(1);
      await captureInstructorState(
        freshElena,
        scenarioInput,
        "instructor_authoring_fresh_session_assignment",
        assignmentCard,
      );
      const editAssignment = assignmentCard.getByRole("link", {
        name: "Edit assignment",
        exact: true,
      });
      await editAssignment.focus();
      await freshElena.keyboard.press("Enter");
      await expect(freshElena.getByLabel("Lifecycle")).toHaveValue("published");
      await expect(freshElena.getByTestId("assignment-current-state")).toHaveText(
        "Published, open now.",
      );
      await freshElena.getByRole("link", { name: "Students" }).click();
      await expect(freshElena.getByRole("heading", { name: "Pending invitations" })).toBeVisible();
      const persistedInvitation = freshElena.getByRole("row").filter({ hasText: rosterId });
      await expect(persistedInvitation).toHaveCount(1);
    } finally {
      try {
        await Promise.all(contexts.map((context) => context.close()));
      } finally {
        writeOriginReceipt(pageOrigins, requestOrigins);
      }
    }
  });
});
