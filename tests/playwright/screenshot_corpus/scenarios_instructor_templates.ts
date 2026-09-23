// Instructor Assessment Template creation capture with a selected editable Template.
// Selector contract: Ribbon navigation is owned by visible_workflows.ts; the create form and
// focused editor headings are owned by src/pages/assessment_templates_page.tsx.

import type { ScenarioRuntime } from "./runtime";
import { viewportCoverage, type ScenarioDefinition } from "./scenario_types";
import { enterInstructor } from "./visible_workflows";

const TEMPLATE_NAME = "Weekly Quiz settings";

async function instructorTemplates(runtime: ScenarioRuntime): Promise<void> {
  const session = await runtime.open("template_editor");
  try {
    await enterInstructor(session.page);
    await session.page
      .getByRole("navigation", { name: "Ribbon tabs", exact: true })
      .getByRole("link", { name: "Assessments", exact: true })
      .click();
    await session.page
      .getByRole("navigation", { name: "Ribbon tasks", exact: true })
      .getByRole("link", { name: "My Assessment Templates", exact: true })
      .click();
    await session.page
      .getByRole("heading", { level: 1, name: "My Assessment Templates" })
      .waitFor();
    // Create a Template is a disclosure button. It auto-expands once a ready list has zero
    // Templates, so wait for the list, then expand only when it is still collapsed.
    const createDisclosure = session.page.getByRole("button", {
      name: "Create a Template",
      exact: true,
    });
    await session.page.getByText("Loading your Templates...").waitFor({ state: "hidden" });
    if ((await createDisclosure.getAttribute("aria-expanded")) !== "true") {
      await createDisclosure.click();
    }
    const createTemplateForm = session.page.locator("form#create-assessment-template");
    await createTemplateForm.waitFor();
    await createTemplateForm
      .getByRole("textbox", { name: "Template name", exact: true })
      .fill(TEMPLATE_NAME);
    await createTemplateForm
      .getByRole("combobox", { name: /^Assessment Type/ })
      .selectOption("quiz");
    await createTemplateForm.getByRole("button", { name: "Create Template", exact: true }).click();
    await session.page
      .getByText("Template created with the canonical settings for its Assessment Type.")
      .waitFor();
    const editorHeading = session.page.getByRole("heading", {
      level: 2,
      name: "Edit Template",
      exact: true,
    });
    await editorHeading.waitFor();
    await editorHeading.scrollIntoViewIfNeeded();
    await runtime.captureCheckpoint(session, "template_editor");
  } finally {
    await runtime.close(session);
  }
}

export const INSTRUCTOR_TEMPLATE_SCENARIOS: ReadonlyArray<ScenarioDefinition> = [
  {
    id: "instructor_templates",
    role: "instructor",
    captures: [
      {
        checkpoint: "template_editor",
        area: "assignments",
        workflow: "assessment template creation",
        state: "editable template",
        viewport: "laptop",
        privacyProfile: "instructor_answer_free",
        caption: "Editable Assessment Template",
      },
    ],
    viewportCoverage: viewportCoverage(["laptop"], {
      tablet: {
        target: "template_editor",
        reason:
          "template_editor captures the editable Assessment Template on the Instructor Assessment Templates route as a laptop representative substitution. The tablet layout remains unverified until a tablet replay is captured.",
      },
      phone: {
        target: "template_editor",
        reason:
          "template_editor captures the editable Assessment Template on the Instructor Assessment Templates route as a laptop representative substitution. The phone layout remains unverified until a phone replay is captured.",
      },
      square: {
        target: "template_editor",
        reason:
          "template_editor captures the editable Assessment Template on the Instructor Assessment Templates route as a laptop representative substitution. The square layout remains unverified until a square replay is captured.",
      },
    }),
    run: instructorTemplates,
  },
];
