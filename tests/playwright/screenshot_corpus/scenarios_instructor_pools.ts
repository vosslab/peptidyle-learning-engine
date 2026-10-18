// Instructor reusable Question Pool review capture.
// Selector contract: the Question Library Ribbon task is in ribbon_catalog.ts; the create action
// is owned by src/pages/library_page.tsx:204; the initial picker dialog and review action are
// owned by src/components/question_pool_create_dialog.tsx:386-390; review regions and search
// labels are owned there at lines 463-480, while available results and Add/Included actions are
// owned by src/components/question_pool_available_questions.tsx:65-113.

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
    await page.getByRole("main", { name: "Question Library", exact: true }).waitFor();
    await page.getByRole("button", { name: "Create Question Pool", exact: true }).click();
    await page
      .getByRole("heading", {
        level: 2,
        name: "Choose published Questions for this Pool",
        exact: true,
      })
      .waitFor();
    const picker = page.getByRole("dialog", {
      name: "Choose published Questions for this Pool",
      exact: true,
    });
    await picker.getByLabel("Search Questions", { exact: true }).fill("Genetic disorders");
    await picker.getByRole("button", { name: "Search", exact: true }).click();
    const results = picker.getByRole("region", { name: "Question results", exact: true });
    // The first visible selection establishes the Pool's eligibility, so this unbound picker
    // deliberately offers one radio choice before it can offer interchangeable checkboxes.
    await results.getByRole("radio").first().waitFor();
    await results.getByRole("radio").first().check();
    await page.getByRole("button", { name: "Review selected Questions", exact: true }).click();
    await page
      .getByRole("heading", { level: 1, name: "Create Question Pool", exact: true })
      .waitFor();
    const selectedQuestions = page.getByRole("region", {
      name: "Selected Pool Questions",
      exact: true,
    });
    const availableQuestions = page.getByRole("region", {
      name: "Available Library Questions",
      exact: true,
    });
    await selectedQuestions.locator("[data-record-id]").first().waitFor();
    await page.getByLabel("Search selected Pool Questions", { exact: true }).waitFor();
    const availableSearch = page.getByLabel("Search available Library Questions", {
      exact: true,
    });
    await availableSearch.waitFor();
    await availableQuestions
      .getByRole("button", { name: "Included", exact: true, disabled: true })
      .first()
      .waitFor();
    await availableSearch.fill("Genetic disorders");
    await availableSearch.press("Enter");
    await availableQuestions
      .getByRole("button", { name: "Included", exact: true, disabled: true })
      .first()
      .waitFor();
    await availableQuestions.getByRole("button", { name: "Add", exact: true }).first().click();
    await selectedQuestions.locator("[data-record-id]").nth(1).waitFor();
    await availableQuestions
      .getByRole("button", { name: "Included", exact: true, disabled: true })
      .first()
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
