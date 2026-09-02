// Production-stack instructor authoring journey. Product state is created through visible PLE UI.
//
// Selector contract:
// - src/wasm/context.tsx:46 owns the WebAssembly runtime status label and data attribute.
// - src/features/ple_question_json_authoring/question_json_editor_page.tsx:535 owns the editor fields,
//   publication buttons, and published status.
// - src/pages/library_page.tsx:117 and src/pages/question_detail_page.tsx:24 own library search,
//   question cards, and prompt regions.
// - src/pages/course_list_page.tsx, src/pages/course_assignments_page.tsx, and
//   src/pages/assignment_workspace/ own the visible course and assignment workflow.
// - src/pages/gradebook_page.tsx:151 owns the calculated assignment-cell score observed here.
import { expect, test, type BrowserContext } from "@playwright/test";

import { configuredLiveDemoInputs } from "../../../playwright.config";
import { waitForAutomatedStudentFeedback } from "./automated_grading_ui";
import { BIOCHEMISTRY_COURSE_TITLE } from "./helper_course_titles";
import {
  chooseSeededIdentity,
  configureContextAndPage,
  observeContextOrigins,
  relativeIsoDate,
  requireScenarioInput,
  selectVisibleCourse,
  startOrContinuePractice,
  writeOriginReceipt,
} from "./real_stack_ui";

const maryEmail = "mary.okafor@live-demo.ple.example";
const actionTimeoutMs = 30_000;
const scenarioTimeoutMs = 300_000;

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
    const questionTitle = "Peptide Bond Resonance";
    const prompt = "Which statement about peptide bonds is correct?";
    const correctChoice = "Peptide bonds have partial double-bond character";
    const courseTitle = "Biochemistry: Protein Structure Workshop";
    const assignmentTitle = "Peptide Bonds and Planarity";
    const rosterId = "BIO-MARY-001";
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
      configureContextAndPage(initialContext, elena, actionTimeoutMs);

      await chooseSeededIdentity(elena, /Elena Rivera/u);
      await selectVisibleCourse(elena, BIOCHEMISTRY_COURSE_TITLE);
      const wasmRuntime = elena.locator('[data-runtime-mode="wasm"]');
      await expect(wasmRuntime).toBeAttached();
      await expect(wasmRuntime).toHaveAttribute("data-runtime-mode", "wasm");
      await elena.getByRole("link", { name: "Workspace" }).click();
      await expect(
        elena.getByRole("heading", { name: "Draft, preview, and publish a learning question" }),
      ).toBeVisible();
      await elena.getByRole("button", { name: "Create Question" }).click();
      await expect(elena.getByLabel("Question title")).toBeVisible();
      await elena.getByLabel("Question title").fill(questionTitle);
      await elena.getByLabel("Student-facing prompt").fill(prompt);
      await elena.getByLabel("Question format").selectOption("multipleAnswer");
      await elena.getByLabel("Choice text").nth(0).fill(correctChoice);
      await elena
        .getByLabel("Choice text")
        .nth(1)
        .fill("Peptide bonds freely rotate around the carbon-nitrogen bond");
      await elena.getByRole("checkbox", { name: "Correct answer", exact: true }).first().check();
      await elena.getByRole("button", { name: "Save private draft" }).click();
      await expect(elena.getByRole("button", { name: "Review publication changes" })).toBeEnabled();
      const studentPreview = elena
        .getByRole("heading", { name: "Student preview", exact: true })
        .locator("..")
        .locator("article");
      await expect(studentPreview).toContainText(prompt);
      await expect(
        studentPreview.getByRole("checkbox", { name: correctChoice, exact: true }),
      ).toBeVisible();
      await expect(studentPreview).not.toContainText("Correct answer");
      await elena.getByRole("button", { name: "Review publication changes" }).click();
      await elena.getByLabel("Question Authors").fill("Dr. Elena Rivera");
      await elena.getByRole("button", { name: "Confirm and publish" }).click();
      await expect(elena.getByRole("heading", { name: "Published" })).toBeVisible();
      const publicationResult = elena.getByRole("status").filter({ hasText: questionTitle });
      await expect(publicationResult).toContainText("Question ID:");
      await expect(publicationResult).toContainText("By: Dr. Elena Rivera");

      await elena.getByRole("link", { name: "Library", exact: true }).click();
      await expect(
        elena.getByRole("heading", { name: "Question library", exact: true }),
      ).toBeVisible();
      await elena.getByLabel("Search published questions").fill(questionTitle);
      const questionCard = elena
        .getByRole("region", { name: "Published questions" })
        .getByText(questionTitle, { exact: true })
        .locator("..");
      await expect(questionCard).toBeVisible();
      const questionId = await questionCard.locator("code").innerText();
      expect(questionId).toMatch(/^[A-Z0-9]{3}-[A-Z0-9]{4}$/u);
      await questionCard.getByRole("link", { name: "Open question", exact: true }).click();
      await expect(elena.getByRole("heading", { name: questionTitle, exact: true })).toBeVisible();
      await expect(elena.getByRole("region", { name: "Question prompt" })).toContainText(prompt);

      await elena.getByRole("link", { name: "Return to question library", exact: true }).click();
      await expect(
        elena.getByRole("heading", { name: "Question library", exact: true }),
      ).toBeVisible();
      await elena.getByRole("link", { name: "Courses" }).click();
      await elena.getByLabel("Course title").fill(courseTitle);
      await elena.getByLabel("Start date").fill(relativeIsoDate(-30));
      await elena.getByLabel("End date").fill(relativeIsoDate(365));
      await elena.getByLabel("Time zone (IANA)").fill("America/Chicago");
      await elena.getByRole("button", { name: "Create course" }).click();
      const courseCard = elena
        .getByRole("article")
        .filter({ has: elena.getByRole("heading", { name: courseTitle, exact: true }) });
      await expect(courseCard).toHaveCount(1);
      await courseCard.getByRole("link", { name: "Open course", exact: true }).click();
      await elena.getByRole("link", { name: "Assignments" }).click();
      await expect(elena.getByRole("heading", { name: "Assignments", exact: true })).toBeVisible();
      await expect(elena.getByRole("link", { name: "Create the first assignment" })).toBeVisible();
      await elena.getByRole("link", { name: "Create the first assignment" }).click();
      await expect(
        elena.getByRole("heading", { name: "Create an Assignment", exact: true }),
      ).toBeVisible();
      await elena.getByLabel("Assignment title").fill(assignmentTitle);
      await elena.getByRole("button", { name: "Create Assignment", exact: true }).click();
      await expect(elena.getByRole("heading", { name: "Questions", exact: true })).toBeVisible();
      await expect(elena.getByText("Add at least one question.", { exact: true })).toBeVisible();
      await elena.getByRole("link", { name: "Policies", exact: true }).click();
      await expect(elena.getByRole("heading", { name: "Policies", exact: true })).toBeVisible();
      await elena.getByLabel("Lifecycle").selectOption("released");
      await elena.getByRole("button", { name: "Save assignment policies", exact: true }).click();
      const addQuestionRecovery = elena.getByRole("link", {
        name: "Add at least one question",
        exact: true,
      });
      await expect(addQuestionRecovery).toBeFocused();
      await elena.keyboard.press("Enter");
      await expect(elena.getByRole("heading", { name: "Questions", exact: true })).toBeVisible();
      await elena.getByRole("button", { name: "Search question library", exact: true }).click();
      const picker = elena.getByRole("dialog", {
        name: "Choose assignment questions",
        exact: true,
      });
      await expect(picker).toBeVisible();
      // AssignmentEditorRepository orders its sources with the current Library first, so the
      // assignment journey starts in the intended source without an extra source mutation.
      await picker.getByLabel("Search questions", { exact: true }).fill(questionTitle);
      await picker.getByRole("button", { name: "Search questions", exact: true }).click();
      const libraryQuestion = picker.getByRole("checkbox", { name: new RegExp(questionTitle) });
      await expect(libraryQuestion).toBeVisible();
      await libraryQuestion.check();
      await picker.getByRole("button", { name: "Add selected questions", exact: true }).click();
      await expect(picker).toHaveCount(0);
      await expect(elena.locator(".assignment-editor-list")).toContainText(questionTitle);
      await elena.getByRole("button", { name: "Save questions and order", exact: true }).click();
      await expect(
        elena.getByRole("status").filter({ hasText: "Questions and order saved." }),
      ).toBeVisible();
      await elena.getByRole("link", { name: "Overview", exact: true }).click();
      await expect(
        elena.getByRole("heading", { name: assignmentTitle, exact: true }),
      ).toBeVisible();
      await elena.getByRole("link", { name: "Policies", exact: true }).click();
      await expect(elena.getByRole("heading", { name: "Policies", exact: true })).toBeVisible();
      await expect(elena.getByLabel("Lifecycle")).toHaveValue("unreleased");
      await elena.getByLabel("Lifecycle").selectOption("released");
      await elena.getByRole("button", { name: "Save assignment policies", exact: true }).click();
      const publishedResult = elena
        .getByRole("status")
        .filter({ hasText: "Assignment policies saved." });
      await expect(publishedResult).toBeVisible();
      await expect(elena.getByRole("heading", { name: "Policies", exact: true })).toBeVisible();
      await expect(elena.getByLabel("Lifecycle")).toHaveValue("released");

      await elena.getByRole("link", { name: "Student view", exact: true }).click();
      await expect(
        elena.getByRole("heading", { name: assignmentTitle, exact: true }),
      ).toBeVisible();
      await expect(
        elena.getByText(
          "Student view - current live assignment. Use Student entry to submit graded work.",
          { exact: true },
        ),
      ).toBeVisible();
      const courseNavigation = elena.getByRole("navigation", { name: "Course management" });
      await expect(courseNavigation).toBeInViewport();
      await expect(
        courseNavigation.getByRole("link", { name: "Assignments", exact: true }),
      ).toBeInViewport();
      const workspaceNavigation = elena.getByRole("navigation", {
        name: "Assignment workspace",
      });
      for (const name of ["Overview", "Questions", "Policies", "Student view"] as const) {
        await expect(workspaceNavigation.getByRole("link", { name, exact: true })).toBeInViewport();
      }

      await elena.getByRole("link", { name: "Return to assignment", exact: true }).click();
      await expect(
        elena.getByRole("heading", { name: assignmentTitle, exact: true }),
      ).toBeVisible();
      await elena.getByRole("link", { name: "Assignments", exact: true }).click();
      await elena.getByRole("link", { name: "Students" }).click();
      await elena.getByLabel("Course roster email").fill(maryEmail);
      await elena.getByLabel("Course roster ID").fill(rosterId);
      await elena.getByRole("button", { name: "Create invitation" }).click();
      const invitation = elena.getByLabel("Invitation link");
      await expect(invitation).toBeVisible();
      const invitationUrl = await invitation.inputValue();
      expect(new URL(invitationUrl).origin).toBe(new URL(elena.url()).origin);
      await expect(elena.getByRole("heading", { name: "Pending invitations" })).toBeVisible();
      await expect(elena.getByRole("row").filter({ hasText: rosterId })).toHaveCount(1);

      const maryContext = await browser.newContext({
        viewport: { width: 1280, height: 800 },
        ignoreHTTPSErrors: true,
      });
      contexts.push(maryContext);
      observeContextOrigins(maryContext, pageOrigins, requestOrigins);
      const mary = await maryContext.newPage();
      configureContextAndPage(maryContext, mary, actionTimeoutMs);
      await chooseSeededIdentity(mary, /Mary Okafor/u);
      await mary.goto(invitationUrl);
      await expect(mary.getByRole("heading", { name: "Join your PLE course" })).toBeVisible();
      await mary.getByRole("button", { name: "Claim this course" }).click();
      await expect(mary.getByRole("heading", { name: courseTitle, exact: true })).toBeVisible();
      const learnerAssignment = mary
        .getByRole("article")
        .filter({ has: mary.getByRole("heading", { name: assignmentTitle, exact: true }) });
      await learnerAssignment.getByRole("link", { name: "Start assignment", exact: true }).click();
      await expect(mary.locator("[data-route-surface=assignmentOverview]")).toBeVisible();
      await startOrContinuePractice(mary);
      await mary.getByRole("checkbox", { name: correctChoice, exact: true }).check();
      await mary.getByRole("button", { name: "Submit answer", exact: true }).click();
      const feedback = await waitForAutomatedStudentFeedback(mary);
      await expect(feedback.getByRole("heading", { name: "Correct", exact: true })).toBeVisible();
      await mary
        .getByRole("button", { name: "View completed Assignment Attempt", exact: true })
        .click();
      const completedAssignmentAttempt = mary.locator(".attempt-summary");
      await expect(
        completedAssignmentAttempt.getByText("Your completed Assignment Attempt is recorded."),
      ).toBeVisible();
      await expect(
        completedAssignmentAttempt.getByRole("region", { name: "Assignment score" }),
      ).toContainText("Best 100%");

      await elena.getByRole("link", { name: "Gradebook", exact: true }).click();
      await expect(elena.locator("[data-route-surface=gradebook]")).toBeVisible();
      const learnerScore = elena
        .locator("tr.gradebook-row")
        .filter({ has: elena.getByText("Mary Okafor", { exact: true }) });
      await expect(learnerScore).toHaveCount(1);
      await expect(learnerScore.locator(".gradebook-course-total")).toContainText("100%");
      await expect(learnerScore.locator(`[data-label="${assignmentTitle}"]`)).toContainText("100%");
      await elena.getByRole("link", { name: "Students", exact: true }).click();

      await elena.reload();
      await expect(elena.getByRole("heading", { name: "Students" })).toBeVisible();
      await expect(elena.getByRole("row").filter({ hasText: rosterId })).toHaveCount(1);
      await expect(elena.getByLabel("Invitation link")).toHaveCount(0);

      const freshElenaContext = await browser.newContext({
        viewport: { width: 1280, height: 800 },
        ignoreHTTPSErrors: true,
      });
      contexts.push(freshElenaContext);
      observeContextOrigins(freshElenaContext, pageOrigins, requestOrigins);
      const freshElena = await freshElenaContext.newPage();
      configureContextAndPage(freshElenaContext, freshElena, actionTimeoutMs);
      await chooseSeededIdentity(freshElena, /Elena Rivera/u);
      await selectVisibleCourse(freshElena, courseTitle);
      await freshElena.getByRole("link", { name: "Assignments" }).click();
      const assignmentCard = freshElena
        .getByRole("article")
        .filter({ has: freshElena.getByRole("heading", { name: assignmentTitle, exact: true }) });
      await expect(assignmentCard).toHaveCount(1);
      const assignmentTitleLink = assignmentCard.getByRole("link", {
        name: assignmentTitle,
        exact: true,
      });
      await assignmentTitleLink.focus();
      await freshElena.keyboard.press("Enter");
      await expect(
        freshElena.getByRole("heading", { name: assignmentTitle, exact: true }),
      ).toBeVisible();
      await freshElena.getByRole("link", { name: "Policies", exact: true }).click();
      await expect(freshElena.getByLabel("Lifecycle")).toHaveValue("released");
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
