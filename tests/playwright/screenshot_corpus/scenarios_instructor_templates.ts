// Instructor Assessment Template creation capture with a selected editable Template.
// Selector contract: Ribbon navigation is owned by visible_workflows.ts; the create form and
// focused editor headings are owned by src/pages/assessment_templates_page.tsx.

import type { ScenarioRuntime } from "./runtime";
import type { ScenarioDefinition } from "./scenario_types";
import { enterInstructor } from "./visible_workflows";

const TEMPLATE_NAME = "Weekly Quiz settings";

async function instructorTemplates(runtime: ScenarioRuntime): Promise<void> {
  const scenario = "instructor_templates";
  const session = await runtime.open(runtime.record(scenario, "template_editor"));
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
    const createTemplateForm = session.page
      .getByRole("heading", { level: 3, name: "Create a Template", exact: true })
      .locator("xpath=ancestor::form");
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
    await runtime.capture(session, runtime.record(scenario, "template_editor"));
  } finally {
    await runtime.close(session);
  }
}

export const INSTRUCTOR_TEMPLATE_SCENARIOS: ReadonlyArray<ScenarioDefinition> = [
  {
    id: "instructor_templates",
    checkpoints: ["template_editor"],
    run: instructorTemplates,
  },
];
