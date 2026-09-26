// Instructor reusable Question Pool review capture.
// Selector contract: the Question Library Ribbon task is in ribbon_catalog.ts; the create action
// is owned by src/pages/library_page.tsx and picker headings, result checkboxes, and review action
// are owned by src/components/question_pool_create_dialog.tsx and question_picker.tsx.

import type { ScenarioRuntime } from "./runtime";
import { catalogScreenshotFilename } from "./filenames";
import { viewportCoverage, type ScenarioDefinition } from "./scenario_types";
import { enterInstructor } from "./visible_workflows";

async function instructorPools(runtime: ScenarioRuntime): Promise<void> {
  const session = await runtime.open("pool_creation_review");
  const page = session.page;
  try {
    await enterInstructor(page);
    await page
      .getByRole("navigation", { name: "Ribbon tabs", exact: true })
      .getByRole("link", { name: "Questions", exact: true })
      .click();
    await page
      .getByRole("heading", { level: 1, name: "Search Question Library", exact: true })
      .waitFor();
    await page.getByRole("button", { name: "Create Question Pool", exact: true }).click();
    await page
      .getByRole("heading", {
        level: 2,
        name: "Choose published Questions for this Pool",
        exact: true,
      })
      .waitFor();
    await page.getByRole("button", { name: "Search questions", exact: true }).click();
    const results = page.getByRole("region", { name: "Question results", exact: true });
    await results.getByRole("checkbox").nth(1).waitFor();
    await results.getByRole("checkbox").first().check();
    await results.getByRole("checkbox").nth(1).check();
    await page.getByRole("button", { name: "Review selected Questions", exact: true }).click();
    await page
      .getByRole("heading", { level: 1, name: "Create Question Pool", exact: true })
      .waitFor();
    await runtime.captureCheckpoint(session, "pool_creation_review");
  } finally {
    await runtime.close(session);
  }
}

export const INSTRUCTOR_POOL_SCENARIOS: ReadonlyArray<ScenarioDefinition> = [
  {
    id: "instructor_pools",
    role: "instructor",
    captures: [
      {
        checkpoint: "pool_creation_review",
        filenameStem: catalogScreenshotFilename(
          "questions",
          "searchQuestionLibrary",
          "pool_creation_review",
        ),
        area: "question library",
        workflow: "question pool creation",
        state: "selected Question review",
        viewport: "laptop",
        privacyProfile: "instructor_answer_free",
        caption: "Question Pool creation review",
      },
    ],
    viewportCoverage: viewportCoverage(["laptop"], {
      tablet: {
        target: "pool_creation_review",
        reason:
          "pool_creation_review captures the Instructor Question Library's selected Question Pool creation review state as a laptop representative substitution. The tablet layout remains unverified until a tablet replay is captured.",
      },
      phone: {
        target: "pool_creation_review",
        reason:
          "pool_creation_review captures the Instructor Question Library's selected Question Pool creation review state as a laptop representative substitution. The phone layout remains unverified until a phone replay is captured.",
      },
      square: {
        target: "pool_creation_review",
        reason:
          "pool_creation_review captures the Instructor Question Library's selected Question Pool creation review state as a laptop representative substitution. The square layout remains unverified until a square replay is captured.",
      },
    }),
    run: instructorPools,
  },
];
