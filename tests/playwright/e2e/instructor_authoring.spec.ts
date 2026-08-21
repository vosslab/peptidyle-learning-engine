// Production-stack instructor authoring journey. Product state is created through visible PLE UI.
import { expect, test, type BrowserContext, type Page } from "@playwright/test";
import { writeFileSync } from "node:fs";

import { configuredLiveDemoInputs } from "../../../playwright.config";
import { liveDemoOriginReceiptPathFromEnvironment } from "../live_demo_live_config";
import {
  chooseSeededIdentity,
  observeContextOrigins,
  requireScenarioInput,
  selectVisibleCourse,
} from "./real_stack_ui";

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
      await elena.getByRole("link", { name: "Workspace" }).click();
      await elena.getByRole("button", { name: "Create flat question" }).click();
      await elena.getByLabel("Question title").fill(questionTitle);
      await elena.getByLabel("Learner-facing prompt").fill(`Which statement is correct? ${tag}`);
      await elena.getByLabel("Choice text").nth(0).fill(correctChoice);
      await elena.getByLabel("Choice text").nth(1).fill(`Incorrect choice ${tag}`);
      await elena
        .getByRole("radio", { name: new RegExp(`Mark choice 1 as correct: ${correctChoice}`) })
        .check();
      await elena.getByRole("button", { name: "Save private draft" }).click();
      await elena.getByRole("button", { name: "Review publication changes" }).click();
      await elena.getByLabel("Reviewed public byline").fill("Dr. Elena Rivera");
      await elena.getByRole("button", { name: "Confirm and publish" }).click();
      await expect(elena.getByRole("heading", { name: "Published" })).toBeVisible();

      await elena.getByRole("link", { name: "Library", exact: true }).click();
      await elena.getByLabel("Search published questions").fill(questionTitle);
      const questionCard = elena
        .getByRole("region", { name: "Published questions" })
        .getByText(questionTitle)
        .locator("..");
      await expect(questionCard).toBeVisible();
      const questionId = await questionCard.locator("code").innerText();
      expect(questionId).toMatch(/^[A-Z0-9]{3}-[A-Z0-9]{4}$/u);

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
      await courseCard.getByRole("link", { name: "Open course", exact: true }).click();
      await elena.getByRole("link", { name: "Assignments" }).click();
      await elena.getByRole("link", { name: "Create the first assignment" }).click();
      await elena.getByLabel("Assignment title").fill(assignmentTitle);
      await elena.getByText("Add several Question IDs", { exact: true }).click();
      await elena.getByLabel("Question IDs").fill(questionId);
      await elena.getByRole("button", { name: "Add questions by ID" }).click();
      await elena.getByRole("button", { name: "Create assignment" }).click();
      await expect(elena.getByText(`${assignmentTitle} now appears in this course.`)).toBeVisible();
      await elena.getByRole("link", { name: `Open ${assignmentTitle}` }).click();
      await elena.getByLabel("Lifecycle").selectOption("published");
      await elena.getByRole("button", { name: "Save teaching operations" }).click();
      await expect(elena.getByText("Published, open now.")).toBeVisible();

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
      const editAssignment = assignmentCard.getByRole("link", {
        name: "Edit assignment",
        exact: true,
      });
      await editAssignment.focus();
      await freshElena.keyboard.press("Enter");
      await expect(freshElena.getByLabel("Lifecycle")).toHaveValue("published");
      await expect(freshElena.getByText("Published, open now.")).toBeVisible();
      await freshElena.getByRole("link", { name: "Students" }).click();
      await expect(freshElena.getByRole("row").filter({ hasText: rosterId })).toHaveCount(1);
    } finally {
      try {
        await Promise.all(contexts.map((context) => context.close()));
      } finally {
        writeOriginReceipt(pageOrigins, requestOrigins);
      }
    }
  });
});
