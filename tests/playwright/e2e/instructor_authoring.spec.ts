// Production-stack journey: a private Draft Question becomes a Published Question.
// Selector contract: Questions Ribbon tab (src/ribbon/ribbon_catalog.ts:137-147), then Browse
// Question Library and My Draft Questions Ribbon tasks (src/ribbon/ribbon_catalog.ts:324-335,
// 372-383); Question Library heading (src/pages/library_page.tsx:163); Draft Question control and
// My Draft Questions heading
// (src/pages/question_drafts_page.tsx:101,106); JSON editor surface and private-draft status
// (src/features/ple_question_json_authoring/question_json_editor_page.tsx:420-422,439-442);
// Question License (src/features/ple_question_json_authoring/question_json_metadata_fields.tsx:92-105);
// Question Title and Save Question metadata (src/features/ple_question_json_authoring/question_json_metadata_fields.tsx);
// Save private draft (src/features/ple_question_json_authoring/question_json_editor_page.tsx:542-549);
// Review publication changes (src/features/ple_question_json_authoring/question_json_editor_page.tsx:581-587);
// Question Authors and Confirm and publish
// (src/features/ple_question_json_authoring/question_json_editor_page.tsx:604-630);
// published confirmation status, heading, and link (src/features/ple_question_json_authoring/question_json_editor_page.tsx:640-665);
// Question Library heading, article, title, and link (src/pages/library_page.tsx:163,313,314,321); detail title
// (src/pages/question_detail_page.tsx:50-52); owner metadata editor and nullable Bloom fields
// (src/features/question_metadata/question_metadata_editor.tsx:161-179,253-295); Bloom readback
// (src/pages/question_detail_page.tsx:744-746); Library search/results
// (src/pages/library_page.tsx:176,216-223,305-313); Archive confirmation/status/Restore and archived
// Fork (src/pages/question_detail_page.tsx:404-470,63-130). Seeded identity uses the imported helper and its visible contracts:
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
        page.getByRole("heading", { name: "Browse Question Library", exact: true }),
      ).toBeVisible();
      await ribbonTasks.getByRole("link", { name: "My Draft Questions", exact: true }).click();
      await expect(
        page.getByRole("heading", { name: "My Draft Questions", exact: true }),
      ).toBeVisible();
      await page.getByRole("button", { name: "Create Draft Question", exact: true }).click();
      await expect(page.locator('[data-route-surface="pleQuestionJsonEditor"]')).toBeVisible();
      await page.getByLabel("Question Title").fill(questionTitle);
      const draftDescription = "Browser authoring private Draft description.";
      await page.getByLabel("Question Description for Instructors").fill(draftDescription);
      const questionPrompt = `Browser fork copy prompt ${scenarioInput.namespace}.`;
      await page.getByLabel("Student-facing prompt").fill(questionPrompt);
      await page.getByLabel("Question License").selectOption("CC-BY-4.0");
      await page.getByRole("button", { name: "Save Question metadata", exact: true }).click();
      const saveMetadataButton = page.getByRole("button", {
        name: "Save Question metadata",
        exact: true,
      });
      await expect(saveMetadataButton).toHaveText("Save Question metadata");
      await expect(saveMetadataButton).toBeDisabled();
      await expect(page.getByRole("status", { name: "Private draft status" })).toContainText(
        "Private draft saved. It is not published.",
      );
      await page.reload();
      await expect(page.locator('[data-route-surface="pleQuestionJsonEditor"]')).toBeVisible();
      await expect(page.getByLabel("Question Title")).toHaveValue(questionTitle);
      await expect(page.getByLabel("Question Description for Instructors")).toHaveValue(
        draftDescription,
      );
      await expect(page.getByLabel("Question License")).toHaveValue("CC-BY-4.0");
      await expect(page.getByLabel("Student-facing prompt")).toHaveValue(questionPrompt);
      await page.getByRole("button", { name: "Review publication changes", exact: true }).click();
      await page.getByLabel("Question Authors").fill("Live Demo Instructor");
      await page.getByLabel("Discipline (required)").selectOption({ index: 1 });
      await page.getByLabel("Subject (required)").selectOption({ index: 1 });
      await page.getByRole("button", { name: "Confirm and publish", exact: true }).click();
      await expect(page.getByRole("heading", { name: "Published", exact: true })).toBeVisible();
      const publishedConfirmation = page
        .getByRole("status")
        .filter({ has: page.getByRole("heading", { name: "Published", exact: true }) })
        .filter({ hasText: questionTitle });
      await expect(publishedConfirmation).toContainText("Published Revision: 1");
      await expect(
        page.getByRole("heading", { name: "Question published", exact: true }),
      ).toBeFocused();
      const publishedId = (await publishedConfirmation.locator("code").innerText()).trim();
      const openPublished = page.getByRole("link", {
        name: "Open published Question",
        exact: true,
      });
      await expect(openPublished).toHaveAttribute("href", `/library/${publishedId}`);
      await openPublished.click();
      await expect(page).toHaveURL(new RegExp(`/library/${publishedId}$`, "u"));
      await expect(page.getByRole("heading", { name: questionTitle, exact: true })).toBeVisible();

      const metadataEditor = page.getByRole("region", { name: "Question metadata", exact: true });
      await metadataEditor
        .getByRole("button", { name: "Edit Question metadata", exact: true })
        .click();
      await metadataEditor
        .getByLabel("Description")
        .fill("Browser acceptance metadata description.");
      await metadataEditor
        .getByLabel("Tags, one per line")
        .fill("browser-acceptance\nquestion-lifecycle");
      await metadataEditor.getByLabel("Bloom Cognitive Process").selectOption("Analyze");
      await metadataEditor
        .getByLabel("Bloom Knowledge Dimension")
        .selectOption("Conceptual Knowledge");
      await metadataEditor.getByRole("button", { name: "Save metadata", exact: true }).click();
      await expect(metadataEditor.getByRole("status")).toContainText("Question metadata saved.");
      await expect(page.getByRole("region", { name: "Question Description" })).toContainText(
        "Browser acceptance metadata description.",
      );
      await expect(
        page.getByText(
          "Bloom Cognitive Process: Analyze; Bloom Knowledge Dimension: Conceptual Knowledge",
          { exact: true },
        ),
      ).toBeVisible();

      await metadataEditor
        .getByRole("button", { name: "Edit Question metadata", exact: true })
        .click();
      await expect(metadataEditor.getByLabel("Bloom Cognitive Process")).toHaveValue("Analyze");
      await expect(metadataEditor.getByLabel("Bloom Knowledge Dimension")).toHaveValue(
        "Conceptual Knowledge",
      );
      await metadataEditor.getByLabel("Bloom Cognitive Process").selectOption("");
      await metadataEditor.getByLabel("Bloom Knowledge Dimension").selectOption("");
      await metadataEditor.getByRole("button", { name: "Save metadata", exact: true }).click();
      await expect(metadataEditor.getByRole("status")).toContainText("Question metadata saved.");
      await expect(
        page.getByText(
          "Bloom Cognitive Process: Not assigned; Bloom Knowledge Dimension: Not assigned",
          { exact: true },
        ),
      ).toBeVisible();

      const searchQuestionInLibrary = async (): Promise<void> => {
        await page
          .getByRole("link", { name: "Return to Browse Question Library", exact: true })
          .click();
        await expect(
          page.getByRole("heading", { name: "Browse Question Library", exact: true }),
        ).toBeVisible();
        await page
          .getByRole("navigation", { name: "Ribbon tasks", exact: true })
          .getByRole("link", { name: "Search Question Library", exact: true })
          .click();
        await expect(
          page.getByRole("heading", { name: "Search Question Library", exact: true }),
        ).toBeVisible();
        await page.getByLabel("Search Question Library").fill(questionTitle);
        await page.getByRole("button", { name: "Search", exact: true }).click();
        await expect(page.getByRole("region", { name: "Question Library results" })).toBeVisible();
      };

      await page.getByRole("button", { name: "Archive Published Question", exact: true }).click();
      const archiveConfirmation = page.getByLabel(
        new RegExp(`Type ${questionTitle} to confirm`, "u"),
      );
      await expect(archiveConfirmation).toBeVisible();
      const archiveButton = page
        .getByRole("complementary", { name: "Danger Zone: Archive Published Question" })
        .getByRole("button", { name: "Archive Published Question", exact: true });
      await expect(archiveButton).toBeDisabled();
      await archiveConfirmation.fill(questionTitle);
      await archiveButton.click();
      const archivedStatus = page
        .getByRole("region", { name: "Archived Published Question" })
        .getByRole("status");
      await expect(archivedStatus).toContainText(
        "It no longer appears in normal Question Library discovery",
      );

      await searchQuestionInLibrary();
      const libraryResults = page.getByRole("region", { name: "Question Library results" });
      await expect(
        libraryResults.getByRole("heading", { name: questionTitle, exact: true }),
      ).toHaveCount(0);
      await expect(
        page.getByRole("heading", { name: "No Library objects match these filters", exact: true }),
      ).toBeVisible();

      await page.goto(`/library/${publishedId}`);
      await expect(page.getByRole("region", { name: "Archived Published Question" })).toContainText(
        "archived and read-only",
      );
      await page.getByRole("button", { name: "Fork Question", exact: true }).click();
      await expect(page.locator('[data-route-surface="pleQuestionJsonEditor"]')).toBeVisible();
      await expect(page).toHaveURL(/\/authoring\/drafts\/[0-9a-f-]+$/u);
      const forkDraftMatch = /\/authoring\/drafts\/([0-9a-f-]+)$/u.exec(page.url());
      if (forkDraftMatch === null) throw new Error("Fork did not open its private Draft route.");
      const forkDraftId = forkDraftMatch[1];
      expect(forkDraftId).not.toBe(publishedId);
      await expect(page.getByLabel("Question Title")).toHaveValue(questionTitle);
      await expect(page.getByLabel("Question Description for Instructors")).toHaveValue(
        "Browser acceptance metadata description.",
      );
      await expect(page.getByLabel("Tags (comma-separated)")).toHaveValue(
        "browser-acceptance, question-lifecycle",
      );
      await expect(page.getByLabel("Question License")).toHaveValue("CC-BY-4.0");
      await expect(page.getByLabel("Student-facing prompt")).toHaveValue(questionPrompt);
      const forkParentTuple = await page.evaluate(async (draftQuestionId) => {
        const response = await fetch("/api/authoring/drafts", {
          headers: { accept: "application/json" },
          credentials: "same-origin",
          cache: "no-store",
        });
        if (!response.ok) throw new Error("My Question Drafts is unavailable.");
        const payload = (await response.json()) as {
          readonly items?: ReadonlyArray<{
            readonly draftQuestionId?: string;
            readonly parentPublishedQuestionRevisionTuple?: {
              readonly publishedQuestionId: string;
              readonly revisionNumber: number;
            } | null;
          }>;
        };
        return payload.items?.find((item) => item.draftQuestionId === draftQuestionId)
          ?.parentPublishedQuestionRevisionTuple;
      }, forkDraftId);
      expect(forkParentTuple).toEqual({
        publishedQuestionId: publishedId,
        revisionNumber: 1,
      });

      await page.goto(`/library/${publishedId}`);
      await expect(
        page.getByRole("button", { name: "Restore Published Question", exact: true }),
      ).toBeVisible();
      await page.getByRole("button", { name: "Restore Published Question", exact: true }).click();
      await expect(
        page
          .getByRole("status")
          .filter({ hasText: /^Published Question restored to normal availability\.$/u }),
      ).toBeVisible();
      await searchQuestionInLibrary();
      await expect(
        page
          .getByRole("region", { name: "Question Library results" })
          .getByRole("heading", { name: questionTitle, exact: true }),
      ).toBeVisible();
    } finally {
      try {
        await context?.close();
      } finally {
        writeOriginReceipt(pageOrigins, requestOrigins);
      }
    }
  });
});
