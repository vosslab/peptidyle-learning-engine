// Focused real-stack Student answer preservation and answer-withheld review.
// Selector contracts (visible authoring, navigation, controls, and completed review):
// src/pages/question_drafts_page.tsx:252
// src/features/ple_question_json_authoring/question_json_metadata_fields.tsx:49
// src/features/ple_question_json_authoring/question_json_editor_workspace.tsx:104
// src/features/ple_question_json_authoring/question_json_fill_in_editor.tsx:62
// src/pages/assessment_workspace/assessment_workspace_create_page.tsx:187
// src/pages/assessment_workspace/assessment_workspace_questions_view.tsx:394
// src/pages/assessment_workspace/assessment_policy_feedback_controls.tsx:6
// src/components/student_assessment_attempt_navigation.tsx:135
// src/components/question_response_controls/backend_owned_document.tsx:311
// src/components/question_response_controls/short_text.tsx:39
// src/pages/assessment_attempt_page.tsx:734
// src/pages/assessment_attempt_finish.ts:9
// src/pages/assessment_attempt_summary_page.tsx:135
// src/pages/assessment_attempt_summary_page.tsx:164

import { expect, test, type FrameLocator, type Page, type Route } from "@playwright/test";

import { configuredLiveDemoInputs } from "../../../playwright.config";
import {
  chooseSeededIdentityAtSignIn,
  observeContextOrigins,
  relativeIsoDate,
  writeOriginReceipt,
} from "./real_stack_ui";
import { followCaptureLink, openStudentCourse } from "../screenshot_corpus/visible_workflows";

const COURSE = "Biochemistry 301: Proteins and Peptides";
// A failed --running rerun retains the prior disposable database.
const TITLE = `Student work preservation browser check ${Date.now().toString()}`;
const WEBWORK = "Genetic Disorders from Descriptions";
const FIB = `${TITLE} native FiB`;

async function publishFib(page: Page): Promise<void> {
  await page.getByRole("link", { name: "My Draft Questions", exact: true }).click();
  await page.getByRole("heading", { name: "My Draft Questions", exact: true }).waitFor();
  await page.getByLabel("Question source format").selectOption("pleQuestionJson");
  await page.getByRole("button", { name: "Create Draft Question", exact: true }).click();
  await page.getByLabel("Question Title", { exact: true }).fill(FIB);
  await page.getByLabel("Student-facing prompt", { exact: true }).fill("Type replication.");
  await page.getByLabel("Question License").selectOption("CC-BY-4.0");
  await page.getByLabel("Question format").selectOption("fillIn");
  await page.getByLabel("Accepted answer 1", { exact: true }).fill("replication");
  await page.getByLabel("Question Description for Instructors", { exact: true }).fill(FIB);
  await page.getByRole("button", { name: "Save Question metadata", exact: true }).click();
  await page.getByText("Private draft saved. It is not published.", { exact: true }).waitFor();
  await page.getByRole("button", { name: "Review publication changes", exact: true }).click();
  await page.getByLabel("Question Authors").fill("Live Demo Instructor");
  await page
    .getByRole("combobox", { name: "Discipline (required)", exact: true })
    .selectOption({ label: "Biology" });
  await page
    .getByRole("combobox", { name: "Subject (required)", exact: true })
    .selectOption({ label: "Biochemistry" });
  await page.getByRole("button", { name: "Confirm and publish", exact: true }).click();
  await page.getByRole("heading", { name: "Published", exact: true }).waitFor();
}

async function instructor(page: Page): Promise<void> {
  await page.goto("/sign-in");
  await chooseSeededIdentityAtSignIn(page, /Elena Rivera/u);
  await page.getByRole("heading", { name: "Search Question Library", exact: true }).waitFor();
  await publishFib(page);
  await page.getByRole("link", { name: "Courses", exact: true }).click();
  const card = page
    .getByRole("listitem")
    .filter({ has: page.getByRole("heading", { name: COURSE, exact: true }) });
  await followCaptureLink(page, card.getByRole("link", { name: "Open Course", exact: true }));
  await page.getByRole("link", { name: "Create Assessment", exact: true }).click();
  await page.getByLabel("Assessment title", { exact: true }).fill(TITLE);
  await page
    .getByRole("combobox", { name: /^Assessment Type/u })
    .selectOption("practice_question_assignment");
  await page.getByRole("button", { name: "Create Assessment", exact: true }).click();
  for (const title of [WEBWORK, FIB]) {
    await page
      .getByRole("button", { name: "Choose published Assessment content", exact: true })
      .click();
    const picker = page.getByRole("dialog", {
      name: "Choose published Assessment content",
      exact: true,
    });
    await picker.getByLabel("Search published Assessment content", { exact: true }).fill(title);
    await picker.getByRole("button", { name: "Search", exact: true }).click();
    await picker.getByRole("checkbox", { name: `Select ${title}`, exact: true }).check();
    await picker.getByRole("button", { name: "Add selected content", exact: true }).click();
  }
  await page.getByRole("button", { name: "Save Questions and order", exact: true }).click();
  await page
    .getByText("Questions and order saved. Review Assessment Properties when you are ready.", {
      exact: true,
    })
    .waitFor();
  await followCaptureLink(
    page,
    page.getByRole("link", { name: "Review Assessment Properties", exact: true }),
  );
  await page.getByRole("combobox", { name: "Correct answer", exact: true }).selectOption("never");
  const randomizedOrder = page.getByLabel("Randomize question order", { exact: true });
  if (await randomizedOrder.isChecked()) await randomizedOrder.uncheck();
  await page
    .getByRole("group", { name: "Due date and time", exact: true })
    .locator('input[type="date"]')
    .fill(relativeIsoDate(30));
  await page.getByLabel("Due time", { exact: true }).fill("12:00");
  await page.getByText("Saved", { exact: true }).waitFor();
  await page.getByRole("button", { name: "Check release readiness", exact: true }).click();
  await page.getByRole("button", { name: "Release assessment", exact: true }).click();
  await page.getByText(/^Assessment released\. Current edit number: [1-9][0-9]*\.$/u).waitFor();
}

async function studentAttempt(page: Page): Promise<void> {
  await page.goto("/sign-in");
  await chooseSeededIdentityAtSignIn(page, /Avery Thompson/u);
  await openStudentCourse(page, COURSE);
  await page.getByRole("link", { name: "Coursework", exact: true }).click();
  const row = page
    .getByRole("listitem")
    .filter({ has: page.getByRole("heading", { name: TITLE, exact: true }) });
  await followCaptureLink(page, row.getByRole("link").first());
  const attempt = page.locator('[data-route-surface="assessmentAttempt"]');
  const start = page.getByRole("button", { name: /^(Start|Resume) Unit Review Assignment$/u });
  await Promise.race([attempt.waitFor(), start.waitFor()]);
  if (!(await attempt.isVisible())) await start.click();
  await attempt.waitFor();
}

function frame(page: Page): FrameLocator {
  return page.frameLocator('iframe[title="Question document"]');
}

async function radioOrder(frameLocator: FrameLocator): Promise<string[]> {
  await frameLocator.locator("form").first().waitFor();
  return frameLocator.locator('input[type="radio"]').evaluateAll((controls) =>
    controls.map((control) => {
      const label =
        control.id === "" ? null : globalThis.document.querySelector(`label[for="${control.id}"]`);
      return (label?.textContent ?? control.getAttribute("value") ?? "").trim();
    }),
  );
}

test.describe("Student work on the production PLE stack", () => {
  test.skip(
    configuredLiveDemoInputs === undefined,
    "the disposable browser owner supplies this scenario input",
  );
  test("autosaves backend and native work, retries failed save without remount, and reviews without answers", async ({
    browser,
    page,
  }) => {
    test.setTimeout(180_000);
    page.setDefaultTimeout(30_000);
    const pageOrigins = new Set<string>();
    const requestOrigins = new Set<string>();
    observeContextOrigins(page.context(), pageOrigins, requestOrigins);
    try {
      await instructor(page);
      // Hold one frame resource so the form is visible before its load/READY event.
      let releaseFrameLoad = (): void => {};
      const frameLoad = new Promise<void>((resolve) => {
        releaseFrameLoad = resolve;
      });
      let markFrameResourceRequested = (): void => {};
      const frameResourceRequested = new Promise<void>((resolve) => {
        markFrameResourceRequested = resolve;
      });
      const imagePath = "/student-work-frame-readiness.svg";
      await page.route(`**${imagePath}`, async (route) => {
        markFrameResourceRequested();
        await frameLoad;
        await route.fulfill({
          contentType: "image/svg+xml",
          body: '<svg xmlns="http://www.w3.org/2000/svg"/>',
        });
      });
      await page.route(
        "**/questions/1/document",
        async (route) => {
          const result = await route.fetch();
          const body = (await result.text()).replace(
            "</body>",
            `<img src="${imagePath}" alt="" hidden></body>`,
          );
          await route.fulfill({ response: result, body });
        },
        { times: 1 },
      );
      await studentAttempt(page);
      const backend = frame(page);
      const radio = backend.locator('input[type="radio"]').first();
      await backend.locator("form").first().waitFor();
      await radio.waitFor();
      await frameResourceRequested;
      await page.getByRole("button", { name: /^Question 2:/u }).click();
      releaseFrameLoad();
      await page.getByText("Question 2 of 2", { exact: true }).waitFor();
      await expect(page.getByText("· 0 saved", { exact: true })).toBeVisible();
      await page.getByLabel("Short written response", { exact: true }).waitFor();
      await page.getByRole("button", { name: /^Question 1:/u }).click();
      await radio.waitFor();
      const initialOrder = await radioOrder(backend);
      await radio.check();
      await page.getByRole("button", { name: /^Question 2:/u }).click();
      await page.getByText("Question 2 of 2", { exact: true }).waitFor();
      await page.getByRole("button", { name: /^Question 1:/u }).click();
      await expect(radio).toBeChecked();
      expect(await radioOrder(backend)).toEqual(initialOrder);

      const embedded = page.locator('iframe[title="Question document"]');
      const handle = await embedded.elementHandle();
      const alternate = backend.locator('input[type="radio"]').nth(1);
      let failed = false;
      const failOnce = async (route: Route): Promise<void> => {
        if (!failed && /\/responses\/1$/u.test(new URL(route.request().url()).pathname)) {
          failed = true;
          await route.abort("failed");
        } else await route.continue();
      };
      await page.route("**/api/assessment-attempts/*/responses/*", failOnce);
      await alternate.check();
      await expect(page.getByText(/Save did not finish/u)).toBeVisible();
      expect(failed).toBe(true);
      expect(await handle?.evaluate((element) => element.isConnected)).toBe(true);
      await expect(alternate).toBeChecked();
      await page.unroute("**/api/assessment-attempts/*/responses/*", failOnce);
      await page.getByRole("button", { name: "Save response", exact: true }).click();
      await page.getByText("Response saved.", { exact: true }).waitFor();

      const freshContext = await browser.newContext();
      try {
        observeContextOrigins(freshContext, pageOrigins, requestOrigins);
        const fresh = await freshContext.newPage();
        await studentAttempt(fresh);
        await fresh.getByRole("button", { name: /^Question 1:/u }).click();
        await frame(fresh).locator("form").first().waitFor();
        await expect(frame(fresh).locator('input[type="radio"]').nth(1)).toBeChecked();
        expect(await radioOrder(frame(fresh))).toEqual(initialOrder);
      } finally {
        await freshContext.close();
      }

      await page.getByRole("button", { name: /^Question 2:/u }).click();
      const input = page
        .locator(
          'section.question-response-control input[type="text"], section.question-response-control textarea',
        )
        .first();
      await input.fill("replication");
      await page.getByRole("button", { name: /^Question 1:/u }).click();
      await page.getByText("Question 1 of 2", { exact: true }).waitFor();
      await expect(page.getByRole("button", { name: "Save response", exact: true })).toBeEnabled();
      await page.getByRole("button", { name: /^Question 2:/u }).click();
      await page.getByText("Question 2 of 2", { exact: true }).waitFor();
      await expect(input).toHaveValue("replication");
      await page.getByRole("button", { name: "Submit Attempt", exact: true }).click();
      await page.locator('[data-route-surface="assessmentAttemptSummary"]').waitFor();
      await expect(
        page.getByRole("heading", { name: "Question and your recorded response", exact: true }),
      ).toBeVisible();
      const firstQuestion = page.locator(".attempt-summary__question").first();
      await expect(firstQuestion.getByText(/Marked (correct|not correct)\./u)).toBeVisible();
      await expect(firstQuestion.getByText(/points/u)).toBeVisible();
      await expect(page.getByTitle("Correct answer", { exact: true })).toHaveCount(0);
      const review = page.frameLocator('iframe[title="Question and your recorded response"]');
      await review.locator("form").waitFor();
      await expect(review.locator('input[type="radio"]:checked')).toHaveCount(1);
      await expect(review.locator("input, select, textarea, button").first()).toBeDisabled();
    } finally {
      writeOriginReceipt(pageOrigins, requestOrigins);
    }
  });
});
