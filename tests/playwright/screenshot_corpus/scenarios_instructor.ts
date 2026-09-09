// scenarios_instructor.ts - Instructor survey and visible mutation checkpoints.
// Selector contract: shared Course actions live in visible_workflows.ts:32; primary headings are
// owned by src/pages/course_list_page.tsx:149, src/pages/library_page.tsx:163,
// src/pages/question_drafts_page.tsx:102, and src/pages/assignment_release_page.tsx:207.

import type { ScenarioRuntime } from "./runtime";
import type { ScenarioDefinition } from "./scenario_types";
import {
  COURSE_TITLE,
  courseCard,
  enterInstructor,
  openInstructorCourse,
  scrollTop,
} from "./visible_workflows";

const AUTHORING_TITLE = "Screenshot corpus peptide geometry";
const BLUEPRINT_PICKER_TITLE = "Screenshot corpus Blueprint preview";
const ASSIGNMENT_TITLE = "Screenshot corpus peptide practice";

async function captureCheckpoint(
  runtime: ScenarioRuntime,
  scenario: string,
  checkpoint: string,
  pageSession: Awaited<ReturnType<ScenarioRuntime["open"]>>,
): Promise<void> {
  await scrollTop(pageSession.page);
  await runtime.capture(pageSession, runtime.record(scenario, checkpoint));
}

async function seededInstructor(runtime: ScenarioRuntime): Promise<void> {
  const scenario = "instructor_seeded";
  const session = await runtime.open(runtime.record(scenario, "course_list"));
  try {
    await enterInstructor(session.page);
    await session.page
      .getByRole("navigation", { name: "Ribbon tabs", exact: true })
      .getByRole("link", { name: "Courses", exact: true })
      .click();
    await session.page
      .getByRole("heading", { name: "Course Instances you teach", exact: true })
      .waitFor();
    await captureCheckpoint(runtime, scenario, "course_list", session);
    const seededCourse = courseCard(session.page, COURSE_TITLE);
    await seededCourse.getByRole("link", { name: "Open Course Instance", exact: true }).click();
    await session.page.getByRole("heading", { name: COURSE_TITLE, exact: true }).waitFor();
    await captureCheckpoint(runtime, scenario, "course_assignment_workspace", session);
    await session.page.getByRole("link", { name: "Open Students", exact: true }).click();
    await session.page.getByRole("heading", { name: "Students", exact: true }).waitFor();
    await session.page.getByText("BIO301-AVERY", { exact: true }).waitFor();
    await captureCheckpoint(runtime, scenario, "course_roster_active", session);
    await session.page
      .getByLabel("Email, roster ID")
      .fill("screenshot.pending@live-demo.invalid,screenshot-pending");
    await session.page.getByRole("button", { name: "Import roster", exact: true }).click();
    await session.page.getByText("screenshot-pending", { exact: true }).waitFor();
    await captureCheckpoint(runtime, scenario, "course_roster_pending_invitation", session);
    await session.page
      .getByRole("navigation", { name: "Ribbon tabs", exact: true })
      .getByRole("link", { name: "Gradebook", exact: true })
      .click();
    await session.page.locator('[data-route-surface="gradebook"]').waitFor();
    await session.page.getByText("BIO301-JACK", { exact: true }).waitFor();
    await captureCheckpoint(runtime, scenario, "gradebook", session);
  } finally {
    await runtime.close(session);
  }
}

async function instructorLibrary(runtime: ScenarioRuntime): Promise<void> {
  const scenario = "instructor_library";
  const session = await runtime.open(runtime.record(scenario, "library_default"));
  try {
    await enterInstructor(session.page);
    await session.page
      .getByRole("heading", { name: "Peptide bond rotation", exact: true })
      .waitFor();
    await captureCheckpoint(runtime, scenario, "library_default", session);
    await session.page.getByLabel("Search published questions").fill("rotation");
    await session.page
      .getByRole("heading", { name: "Peptide bond rotation", exact: true })
      .waitFor();
    await captureCheckpoint(runtime, scenario, "library_filtered", session);
    const result = session.page.locator("article.question-library-row").filter({
      has: session.page.getByRole("heading", { name: "Peptide bond rotation", exact: true }),
    });
    await result.getByRole("link", { name: "Open question", exact: true }).click();
    await session.page.getByRole("region", { name: "Question prompt", exact: true }).waitFor();
    await captureCheckpoint(runtime, scenario, "published_question_detail", session);
  } finally {
    await runtime.close(session);
  }
}

async function instructorAuthoring(runtime: ScenarioRuntime): Promise<void> {
  const scenario = "instructor_authoring";
  const session = await runtime.open(runtime.record(scenario, "draft_list"));
  try {
    await enterInstructor(session.page);
    await session.page.getByRole("link", { name: "My Question Drafts", exact: true }).click();
    await session.page.getByRole("heading", { name: "My Question Drafts", exact: true }).waitFor();
    await session.page
      .getByText("Loading your private Draft Questions...", { exact: true })
      .waitFor({ state: "hidden" });
    await captureCheckpoint(runtime, scenario, "draft_list", session);
    await session.page.getByRole("button", { name: "New Draft Question", exact: true }).click();
    await session.page.getByLabel("Question Title").fill(AUTHORING_TITLE);
    await session.page.getByLabel("Question License").selectOption("CC-BY-4.0");
    await session.page.getByRole("button", { name: "Save private draft", exact: true }).click();
    await session.page
      .getByText("Private draft saved. It is not published.", { exact: true })
      .waitFor();
    await session.page.reload({ waitUntil: "commit" });
    await session.page.locator('[data-route-surface="pleQuestionJsonEditor"]').waitFor();
    if ((await session.page.getByLabel("Question Title").inputValue()) !== AUTHORING_TITLE) {
      throw new Error("saved Draft Question did not persist after reload");
    }
    await captureCheckpoint(runtime, scenario, "saved_editor", session);
    await session.page
      .getByRole("button", { name: "Review publication changes", exact: true })
      .click();
    await session.page.getByLabel("Question Authors").waitFor();
    await captureCheckpoint(runtime, scenario, "publication_review", session);
    await session.page.getByLabel("Question Authors").fill("Live Demo Instructor");
    await session.page.getByRole("button", { name: "Confirm and publish", exact: true }).click();
    await session.page.getByRole("heading", { name: "Published", exact: true }).waitFor();
    await captureCheckpoint(runtime, scenario, "published_result", session);
    await session.page.getByRole("link", { name: "Open question library", exact: true }).click();
    await session.page.getByRole("heading", { name: AUTHORING_TITLE, exact: true }).waitFor();
  } finally {
    await runtime.close(session);
  }
}

async function instructorBlueprint(runtime: ScenarioRuntime): Promise<void> {
  const scenario = "instructor_blueprint";
  const session = await runtime.open(runtime.record(scenario, "blueprint_list"));
  try {
    await enterInstructor(session.page);
    await session.page.getByRole("link", { name: "Blueprint Courses", exact: true }).click();
    await session.page
      .getByRole("heading", { name: "Build reusable course structure", exact: true })
      .waitFor();
    await session.page.getByText(COURSE_TITLE, { exact: true }).waitFor();
    await captureCheckpoint(runtime, scenario, "blueprint_list", session);
    await session.page.getByText(COURSE_TITLE, { exact: true }).click();
    await session.page.getByRole("heading", { name: COURSE_TITLE, exact: true }).waitFor();
    await captureCheckpoint(runtime, scenario, "blueprint_detail", session);
    await session.page
      .getByRole("link", { name: "Return to Blueprint Courses", exact: true })
      .click();
    await session.page
      .getByRole("button", { name: "Create Blueprint Course", exact: true })
      .click();
    await session.page.getByLabel("Blueprint Course title").fill(BLUEPRINT_PICKER_TITLE);
    await session.page
      .getByRole("button", { name: "Choose published Questions", exact: true })
      .click();
    await session.page.getByRole("button", { name: "Search questions", exact: true }).click();
    await session.page.locator(".question-picker-result input").first().waitFor();
    await captureCheckpoint(runtime, scenario, "blueprint_question_picker", session);
  } finally {
    await runtime.close(session);
  }
}

async function instructorAssignment(runtime: ScenarioRuntime): Promise<void> {
  const scenario = "instructor_assignment";
  const session = await runtime.open(runtime.record(scenario, "assignment_creation"));
  const page = session.page;
  try {
    await enterInstructor(page);
    await openInstructorCourse(page);
    await page.getByRole("link", { name: "Create Assignment", exact: true }).click();
    await page.getByRole("heading", { name: "Create Assignment", exact: true }).waitFor();
    await captureCheckpoint(runtime, scenario, "assignment_creation", session);
    await page.getByLabel("Assignment title").fill(ASSIGNMENT_TITLE);
    await page.getByLabel("Instructions").fill("Complete the selected peptide Question.");
    await page.getByRole("button", { name: "Create Assignment", exact: true }).click();
    await page.getByRole("heading", { name: "Assignment Workspace", exact: true }).waitFor();
    await page.getByRole("group", { name: "Available Published Questions", exact: true }).waitFor();
    await captureCheckpoint(runtime, scenario, "assignment_release_draft", session);
    await page
      .getByRole("group", { name: "Available Published Questions", exact: true })
      .getByRole("checkbox")
      .first()
      .check();
    await page.getByLabel("Due date").fill("2026-12-01T12:00");
    await page.getByLabel("Late-work rule").selectOption("mark_late");
    await page.getByRole("button", { name: "Save Assignment", exact: true }).click();
    await page
      .getByText("Assignment saved. Validate it before release.", { exact: true })
      .waitFor();
    await page.getByRole("button", { name: "Validate Assignment", exact: true }).click();
    await page.getByText(/Assignment validation passed/u).waitFor();
    await page.getByRole("button", { name: "Open Assignment Preview", exact: true }).click();
    await page.getByRole("heading", { name: "Assignment Preview", exact: true }).waitFor();
    await runtime.capture(session, runtime.record(scenario, "assignment_release_preview"));
    await page.getByRole("button", { name: "Return to Assignment Workspace", exact: true }).click();
    await page.getByRole("button", { name: "Release Assignment", exact: true }).click();
    await page.getByText(/Released state · current edit [1-9][0-9]*/u).waitFor();
    await captureCheckpoint(runtime, scenario, "assignment_release_released", session);
  } finally {
    await runtime.close(session);
  }
}

export const INSTRUCTOR_SCENARIOS: ReadonlyArray<ScenarioDefinition> = [
  {
    id: "instructor_seeded",
    checkpoints: [
      "course_list",
      "course_assignment_workspace",
      "course_roster_active",
      "course_roster_pending_invitation",
      "gradebook",
    ],
    run: seededInstructor,
  },
  {
    id: "instructor_library",
    checkpoints: ["library_default", "library_filtered", "published_question_detail"],
    run: instructorLibrary,
  },
  {
    id: "instructor_authoring",
    checkpoints: ["draft_list", "saved_editor", "publication_review", "published_result"],
    run: instructorAuthoring,
  },
  {
    id: "instructor_blueprint",
    checkpoints: ["blueprint_list", "blueprint_detail", "blueprint_question_picker"],
    run: instructorBlueprint,
  },
  {
    id: "instructor_assignment",
    checkpoints: [
      "assignment_creation",
      "assignment_release_draft",
      "assignment_release_preview",
      "assignment_release_released",
    ],
    run: instructorAssignment,
  },
];
