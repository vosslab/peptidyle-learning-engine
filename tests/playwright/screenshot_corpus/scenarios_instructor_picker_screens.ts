// Instructor picker screenshots from normal Blueprint and Course Assessment workflows.
// Selector contract: Blueprint creation owns its Question picker in
// src/features/blueprint_course/blueprint_course_create_dialog.tsx and question_picker.tsx;
// Course Assessment creation owns parallel content regions and row actions in
// src/pages/assessment_workspace/assessment_workspace_questions_view.tsx and
// src/features/assessment_content_picker/assessment_editor.tsx.

import type { Page } from "playwright";

import { decodeLibraryObjectSearchPage } from "../../../src/api/decoders/question_library";

import type { ScenarioRuntime } from "./runtime";
import { catalogScreenshotFilename } from "./filenames";
import { viewportCoverage, type ScenarioDefinition } from "./scenario_types";
import { enterInstructor, openInstructorCourse } from "./visible_workflows";

const BLUEPRINT_TITLE = "Screenshot picker Blueprint preview";
const ASSESSMENT_TITLE = "Screenshot picker Assessment preview";

async function createAssessmentPickerPool(page: Page): Promise<string> {
  const poolTitle = `Assessment picker screenshot ${Date.now()}`;
  const response: unknown = await page.evaluate(async () => {
    const query = new URLSearchParams({
      kind: "questions",
      questions: "all",
      authorship: "any",
      backends: "ple",
      question_types: "multipleChoice",
      page_size: "50",
      sort: "titleAscending",
    });
    const searched = await fetch(`/api/library-objects/search?${query.toString()}`);
    if (!searched.ok) throw new Error(`Native MC Question search status ${searched.status}`);
    return (await searched.json()) as unknown;
  });
  const questionSummaries = decodeLibraryObjectSearchPage(response).items.flatMap((item) =>
    item.kind === "question" ? [item.question.summary] : [],
  );
  const eligibleQuestions = questionSummaries.filter(
    (summary) =>
      summary.backend === "ple" &&
      summary.questionType === "multipleChoice" &&
      summary.availability.availability === "available",
  );
  const source = eligibleQuestions[0];
  if (source === undefined) {
    throw new Error(
      `No available Native MC Question in the first 50 Library rows ` +
        `(questions=${questionSummaries.length}, eligible=${eligibleQuestions.length}).`,
    );
  }
  await page.evaluate(
    async ({ title, publishedQuestionRevisionTuple }) => {
      const created = await fetch("/api/question-pools", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          title,
          description: "One published Question selected from the Library for this picker capture.",
          members: [publishedQuestionRevisionTuple],
        }),
      });
      if (created.status !== 201) {
        const responseBody = await created.text();
        throw new Error(`Pool creation status ${created.status}: ${responseBody}`);
      }
    },
    {
      title: poolTitle,
      publishedQuestionRevisionTuple: source.publishedQuestionRevisionTuple,
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
  await picker.getByLabel("Search Questions", { exact: true }).press("Enter");
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
  await page.getByRole("main", { name: "Questions", exact: true }).waitFor();
  const available = page.getByRole("region", {
    name: "Available Assessment content",
    exact: true,
  });
  await available
    .getByLabel("Search published Assessment content", { exact: true })
    .fill(poolTitle);
  await available.getByRole("button", { name: "Search", exact: true }).click();
  const poolRow = available
    .locator("[data-record-id]")
    .filter({ has: page.getByText(poolTitle, { exact: true }) })
    .first();
  await poolRow.waitFor();
  await poolRow.getByRole("button", { name: "Add", exact: true }).waitFor();
}

async function captureBlueprintQuestionPicker(runtime: ScenarioRuntime): Promise<void> {
  const session = await runtime.open("blueprint_question_picker_laptop");
  try {
    await openBlueprintQuestionPicker(session.page);
    const windowScrollY = await session.page.evaluate(() => {
      window.scrollTo(0, 0);
      return window.scrollY;
    });
    if (windowScrollY !== 0) throw new Error("Blueprint Question picker page is not at the top.");
    const dialogScrollTop = await session.page
      .getByRole("dialog", { name: "Choose the first reusable Questions", exact: true })
      .evaluate((dialog) => {
        dialog.scrollTop = 0;
        return dialog.scrollTop;
      });
    if (dialogScrollTop !== 0)
      throw new Error("Blueprint Question picker dialog is not at the top.");
    await runtime.captureCheckpoint(session, "blueprint_question_picker_laptop");
  } finally {
    await runtime.close(session);
  }
}

async function captureAssessmentContentPicker(runtime: ScenarioRuntime): Promise<void> {
  const session = await runtime.open("assessment_content_picker_laptop");
  try {
    await openAssessmentContentPicker(session.page);
    const windowScrollY = await session.page.evaluate(() => {
      window.scrollTo(0, 0);
      return window.scrollY;
    });
    if (windowScrollY !== 0) throw new Error("Assessment content picker page is not at the top.");
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
