// Instructor picker screenshots from normal Blueprint and Course Assessment workflows.
// Selector contract: Blueprint creation owns its Question picker in
// src/features/blueprint_course/blueprint_course_create_dialog.tsx and question_picker.tsx;
// Course Assessment creation owns the shared content picker in
// src/pages/assessment_workspace/assessment_workspace_questions_view.tsx and
// src/features/assessment_content_picker/assessment_content_picker.tsx.

import type { Locator, Page } from "playwright";

import { decodeQuestionSearchPage } from "../../../src/api/decoders/question_library";

import type { ScenarioRuntime } from "./runtime";
import { catalogScreenshotFilename } from "./filenames";
import { viewportCoverage, type ScenarioDefinition } from "./scenario_types";
import { enterInstructor, openInstructorCourse } from "./visible_workflows";

const BLUEPRINT_TITLE = "Screenshot picker Blueprint preview";
const ASSESSMENT_TITLE = "Screenshot picker Assessment preview";
const SOURCE_QUESTION_TITLE = "Genetic disorders: Which one?";

async function requireOpaqueDialogSurface(dialog: Locator): Promise<void> {
  const background = await dialog.evaluate((element) => getComputedStyle(element).backgroundColor);
  if (background === "transparent" || background === "rgba(0, 0, 0, 0)") {
    throw new Error("the Assessment content picker dialog must resolve an opaque themed surface");
  }
}

async function createAssessmentPickerPool(page: Page): Promise<string> {
  const poolTitle = `Assessment picker screenshot ${Date.now()}`;
  const response: unknown = await page.evaluate(async (sourceQuestionTitle) => {
    const query = new URLSearchParams({
      kind: "questions",
      membership: "all",
      authorship: "any",
      sort: "titleAscending",
      page_size: "50",
      text: sourceQuestionTitle,
    });
    const searched = await fetch(`/api/questions/search?${query.toString()}`);
    if (!searched.ok) throw new Error(`Pilot Question search status ${searched.status}`);
    return (await searched.json()) as unknown;
  }, SOURCE_QUESTION_TITLE);
  const matches = decodeQuestionSearchPage(response).items.filter(
    (item) =>
      item.kind === "question" &&
      item.question.summary.metadata.questionTitle === SOURCE_QUESTION_TITLE,
  );
  const match = matches[0];
  if (matches.length !== 1 || match?.kind !== "question")
    throw new Error("Expected one exact Pilot Question for this capture.");
  const source = match.question.summary;
  await page.evaluate(
    async ({ title, questionId, revisionNumber }) => {
      const created = await fetch("/api/question-pools", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          title,
          description: "One exact Pilot Question for this Assessment picker capture.",
          members: [{ questionId, revisionNumber }],
          interchangeabilityAttested: true,
        }),
      });
      if (created.status !== 201) throw new Error(`Pool creation status ${created.status}`);
    },
    {
      title: poolTitle,
      questionId: source.questionId,
      revisionNumber: source.publishedQuestionRevisionTuple.revisionNumber,
    },
  );
  return poolTitle;
}

async function openBlueprintQuestionPicker(page: Page): Promise<void> {
  await enterInstructor(page);
  await page
    .getByRole("navigation", { name: "Ribbon tabs", exact: true })
    .getByRole("link", { name: "Courses", exact: true })
    .click();
  await page.getByRole("link", { name: "My Blueprint Courses", exact: true }).click();
  await page.getByRole("button", { name: "Create Blueprint Course", exact: true }).click();
  await page.getByLabel("Blueprint Course long name", { exact: true }).fill(BLUEPRINT_TITLE);
  await page
    .getByRole("combobox", { name: "Discipline (required)", exact: true })
    .selectOption({ label: "Biology" });
  await page
    .getByRole("combobox", { name: /^First Assessment Type/u })
    .selectOption("practice_question_assignment");
  await page.getByRole("button", { name: "Choose published Questions", exact: true }).click();
  const picker = page.getByRole("dialog", {
    name: "Choose the first reusable Questions",
    exact: true,
  });
  await picker.getByRole("button", { name: "Search", exact: true }).click();
  await picker.getByRole("heading", { name: "Current results", exact: true }).waitFor();
  await picker.getByRole("checkbox").first().waitFor();
}

async function openAssessmentContentPicker(page: Page): Promise<void> {
  await enterInstructor(page);
  const poolTitle = await createAssessmentPickerPool(page);
  await openInstructorCourse(page);
  await page.getByRole("link", { name: "Create Assessment", exact: true }).click();
  await page.getByLabel("Assessment title", { exact: true }).fill(ASSESSMENT_TITLE);
  await page
    .getByRole("combobox", { name: /^Assessment Type/u })
    .selectOption("practice_question_assignment");
  await page.getByRole("button", { name: "Create Assessment", exact: true }).click();
  await page.getByRole("heading", { name: "Assessment Question Editor", exact: true }).waitFor();
  await page
    .getByRole("button", { name: "Choose published Assessment content", exact: true })
    .click();
  const picker = page.getByRole("dialog", {
    name: "Choose published Assessment content",
    exact: true,
  });
  const rows = picker.getByRole("list", {
    name: "Published Assessment content",
    exact: true,
  });
  await rows.getByRole("listitem").nth(1).waitFor();
  await rows.getByRole("listitem").filter({ hasText: poolTitle }).first().waitFor();
  await requireOpaqueDialogSurface(picker);
}

async function captureBlueprintQuestionPicker(runtime: ScenarioRuntime): Promise<void> {
  const session = await runtime.open("blueprint_question_picker_laptop");
  try {
    await openBlueprintQuestionPicker(session.page);
    await runtime.captureCheckpoint(session, "blueprint_question_picker_laptop");
  } finally {
    await runtime.close(session);
  }
}

async function captureAssessmentContentPicker(runtime: ScenarioRuntime): Promise<void> {
  const session = await runtime.open("assessment_content_picker_laptop");
  try {
    await openAssessmentContentPicker(session.page);
    await runtime.captureCheckpoint(session, "assessment_content_picker_laptop");
  } finally {
    await runtime.close(session);
  }
}

const captures = [
  {
    checkpoint: "blueprint_question_picker_laptop",
    filenameStem: catalogScreenshotFilename("courses", "myBlueprintCourses", "question-picker"),
    area: "blueprint courses",
    workflow: "Blueprint Course creation",
    state: "published Question picker with results",
    viewport: "laptop",
    privacyProfile: "instructor_answer_free",
    caption: "Blueprint Question picker",
  },
  {
    checkpoint: "assessment_content_picker_laptop",
    filenameStem: catalogScreenshotFilename("courses", "assessments", "content-picker"),
    area: "assessments",
    workflow: "Course Assessment editing",
    state: "Question and Pool picker with default results",
    viewport: "laptop",
    privacyProfile: "instructor_answer_free",
    caption: "Assessment Question and Pool picker",
  },
] as const;

export const INSTRUCTOR_PICKER_SCREEN_SCENARIOS: ReadonlyArray<ScenarioDefinition> = [
  {
    id: "instructor_picker_screens",
    role: "instructor",
    captures,
    viewportCoverage: viewportCoverage(["laptop"], {
      square: {
        target: "assessment_content_picker_laptop",
        reason: "Instructor laptop capture is the representative.",
      },
      tablet: {
        target: "assessment_content_picker_laptop",
        reason: "Instructor laptop capture is the representative.",
      },
      phone: {
        target: "assessment_content_picker_laptop",
        reason: "Instructor laptop capture is the representative.",
      },
    }),
    run: async (runtime): Promise<void> => {
      await captureBlueprintQuestionPicker(runtime);
      await captureAssessmentContentPicker(runtime);
    },
  },
];
