// Production-stack journey: a private Draft Question becomes a Published Question.
// Selector contract: Questions Ribbon tab (src/ribbon/ribbon_catalog.ts:137-147), then Browse
// Question Library and My Draft Questions Ribbon tasks (src/ribbon/ribbon_catalog.ts:324-335,
// 372-383); Question Library heading (src/pages/library_page.tsx:163); Draft Question control and
// My Question Drafts heading
// (src/pages/question_drafts_page.tsx:101,106); JSON editor surface and private-draft status
// (src/features/ple_question_json_authoring/question_json_editor_page.tsx:420-422,439-442);
// Question License (src/features/ple_question_json_authoring/question_json_metadata_fields.tsx:92-105);
// Question Title (src/features/ple_question_json_authoring/question_json_editor_page.tsx:466-475);
// Save private draft (src/features/ple_question_json_authoring/question_json_editor_page.tsx:542-549);
// Review publication changes (src/features/ple_question_json_authoring/question_json_editor_page.tsx:581-587);
// Question Authors and Confirm and publish
// (src/features/ple_question_json_authoring/question_json_editor_page.tsx:604-630);
// published confirmation status, heading, and link (src/features/ple_question_json_authoring/question_json_editor_page.tsx:640-665);
// Question Library heading, article, title, and link (src/pages/library_page.tsx:163,313,314,321); detail title
// (src/pages/question_detail_page.tsx:50-52). Seeded identity uses the imported helper and its visible contracts:
// sign-in heading/persona action (src/pages/sign_in_page.tsx:89,120-128), Courses heading
// (src/pages/course_list_page.tsx:140-145), and helper selectors (tests/playwright/e2e/real_stack_ui.ts:89-100).
import { expect, test, type BrowserContext } from "@playwright/test";

import { configuredLiveDemoInputs } from "../../../playwright.config";
import {
  chooseSeededIdentity,
  configureContextAndPage,
  observeContextOrigins,
  requireScenarioInput,
  writeOriginReceipt,
} from "./real_stack_ui";

const actionTimeoutMs = 30_000;
const scenarioTimeoutMs = 120_000;

test.describe("instructor authoring on the production PLE stack", () => {
  test.skip(
    configuredLiveDemoInputs === undefined,
    "the disposable production browser-suite owner supplies this scenario input",
  );

  test("Instructor publishes a private Draft Question into the Question Library", async ({
    browser,
  }) => {
    test.setTimeout(scenarioTimeoutMs);
    const scenarioInput = requireScenarioInput(configuredLiveDemoInputs);
    expect(scenarioInput.scenarioId).toBe("instructor_authoring");
    const questionTitle = `Browser authoring publication ${scenarioInput.namespace}`;
    const pageOrigins = new Set<string>();
    const requestOrigins = new Set<string>();
    let context: BrowserContext | undefined;

    try {
      context = await browser.newContext({
        viewport: { width: 1280, height: 800 },
        ignoreHTTPSErrors: true,
      });
      observeContextOrigins(context, pageOrigins, requestOrigins);
      const page = await context.newPage();
      configureContextAndPage(context, page, actionTimeoutMs);

      await chooseSeededIdentity(page, /Elena Rivera/u);
      const ribbonTabs = page.getByRole("navigation", { name: "Ribbon tabs", exact: true });
      await ribbonTabs.getByRole("link", { name: "Questions", exact: true }).click();
      const ribbonTasks = page.getByRole("navigation", { name: "Ribbon tasks", exact: true });
      await ribbonTasks.getByRole("link", { name: "Browse Question Library", exact: true }).click();
      await expect(
        page.getByRole("heading", { name: "Question library", exact: true }),
      ).toBeVisible();
      await ribbonTasks.getByRole("link", { name: "My Draft Questions", exact: true }).click();
      await expect(
        page.getByRole("heading", { name: "My Question Drafts", exact: true }),
      ).toBeVisible();
      await page.getByRole("button", { name: "New Draft Question", exact: true }).click();
      await expect(page.locator('[data-route-surface="pleQuestionJsonEditor"]')).toBeVisible();
      await page.getByLabel("Question Title").fill(questionTitle);
      await page.getByLabel("Question License").selectOption("CC-BY-4.0");
      await page.getByRole("button", { name: "Save private draft", exact: true }).click();
      await expect(page.getByRole("status", { name: "Private draft status" })).toContainText(
        "Private draft saved. It is not published.",
      );
      await page.getByRole("button", { name: "Review publication changes", exact: true }).click();
      await page.getByLabel("Question Authors").fill("Live Demo Instructor");
      await page.getByRole("button", { name: "Confirm and publish", exact: true }).click();
      await expect(page.getByRole("heading", { name: "Published", exact: true })).toBeVisible();
      const publishedConfirmation = page
        .getByRole("status")
        .filter({ has: page.getByRole("heading", { name: "Published", exact: true }) })
        .filter({ hasText: questionTitle });
      await expect(publishedConfirmation).toContainText("Published to: Question Library");

      await page.getByRole("link", { name: "Open question library", exact: true }).click();
      await expect(
        page.getByRole("heading", { name: "Question library", exact: true }),
      ).toBeVisible();
      const questionCard = page
        .getByRole("article")
        .filter({ has: page.getByRole("heading", { name: questionTitle, exact: true }) });
      await expect(questionCard).toHaveCount(1);
      await questionCard.getByRole("link", { name: "Open question", exact: true }).click();
      await expect(page.getByRole("heading", { name: questionTitle, exact: true })).toBeVisible();
    } finally {
      try {
        await context?.close();
      } finally {
        writeOriginReceipt(pageOrigins, requestOrigins);
      }
    }
  });
});
