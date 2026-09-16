// scenarios_instructor.ts - Instructor survey and visible mutation checkpoints.
// Selector contract: shared Course actions live in visible_workflows.ts:32; primary headings are
// owned by src/pages/course_list_page.tsx:149, src/pages/library_page.tsx:163,
// src/pages/question_drafts_page.tsx:102, assessment_workspace_create_page.tsx, and the focused
// Assessment workspace pages.

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
const PUBLISHED_NATIVE_TITLE = "Biochemistry Chapter 1: Charged functional groups";

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
    await session.page.locator("summary").filter({ hasText: "Roster tools" }).click();
    await session.page
      .getByLabel("Email, roster ID")
      .fill("screenshot.pending@biology.roosevelt.edu,screenshot-pending");
    await session.page.getByRole("button", { name: "Import roster", exact: true }).click();
    await session.page.getByText("screenshot-pending", { exact: true }).waitFor();
    await captureCheckpoint(runtime, scenario, "course_roster_pending_invitation", session);
    await session.page
      .getByRole("navigation", { name: "Ribbon tabs", exact: true })
      .getByRole("link", { name: "Gradebook", exact: true })
      .click();
    await session.page.locator('[data-route-surface="gradebook"]').waitFor();
    await session.page.getByRole("cell", { name: "BIO301-JACK", exact: true }).first().waitFor();
    await captureCheckpoint(runtime, scenario, "gradebook", session);
    await session.page.locator(".ple-app-ribbon__brand").click();
    await session.page.waitForURL((url) => url.pathname === "/instructor");
    await session.page.locator('[data-route-surface="courses"]').waitFor();
    await session.page
      .getByRole("heading", { name: "Course Instances you teach", exact: true })
      .waitFor();
    await session.page
      .getByRole("navigation", { name: "Ribbon tabs", exact: true })
      .getByRole("link", { name: "Assessments", exact: true })
      .click();
    await session.page
      .getByRole("heading", { level: 1, name: "Assessments Due Soon", exact: true })
      .waitFor();
    await Promise.race([
      session.page
        .getByRole("heading", { level: 2, name: "No Assessments are due in the next 7 days." })
        .waitFor(),
      session.page.getByRole("list", { name: "Assessments due in the next 7 days" }).waitFor(),
    ]);
    await captureCheckpoint(runtime, scenario, "assignments_due_soon_empty", session);
  } finally {
    await runtime.close(session);
  }
}

async function instructorProfile(runtime: ScenarioRuntime): Promise<void> {
  const scenario = "instructor_profile";
  const session = await runtime.open(runtime.record(scenario, "default"));
  try {
    await enterInstructor(session.page);
    await session.page.getByRole("button", { name: "Profile", exact: true }).click();
    await session.page.getByRole("menuitem", { name: "Profile", exact: true }).click();
    await session.page
      .getByRole("heading", { level: 1, name: "Your profile", exact: true })
      .waitFor();
    await session.page
      .getByRole("heading", { level: 2, name: "Profile image", exact: true })
      .waitFor();
    await session.page.locator(".profile-thumbnail-placeholder").waitFor();
    await captureCheckpoint(runtime, scenario, "default", session);
  } finally {
    await runtime.close(session);
  }
}

async function instructorLibrary(runtime: ScenarioRuntime): Promise<void> {
  const scenario = "instructor_library";
  const session = await runtime.open(runtime.record(scenario, "library_default"));
  try {
    await enterInstructor(session.page);
    await captureCheckpoint(runtime, scenario, "library_default", session);
    await session.page.getByLabel("Search published questions").fill("charged functional");
    await session.page
      .getByRole("heading", { name: PUBLISHED_NATIVE_TITLE, exact: true })
      .waitFor();
    await captureCheckpoint(runtime, scenario, "library_filtered", session);
    const result = session.page.locator("article.question-library-row").filter({
      has: session.page.getByRole("heading", { name: PUBLISHED_NATIVE_TITLE, exact: true }),
    });
    await result.getByRole("link", { name: "Open question", exact: true }).click();
    await session.page.getByRole("region", { name: "Question prompt", exact: true }).waitFor();
    await captureCheckpoint(runtime, scenario, "published_question_detail", session);
    await session.page
      .getByRole("navigation", { name: "Ribbon tabs", exact: true })
      .getByRole("link", { name: "Questions", exact: true })
      .click();
    await session.page
      .getByRole("navigation", { name: "Ribbon tasks", exact: true })
      .getByRole("link", { name: "Browse Question Library", exact: true })
      .click();
    await session.page
      .getByRole("heading", { name: "Browse Question Library", exact: true })
      .waitFor();
    await session.page.getByRole("heading", { name: "Subjects", exact: true }).waitFor();
    await captureCheckpoint(runtime, scenario, "library_browse", session);
  } finally {
    await runtime.close(session);
  }
}

async function instructorAuthoring(runtime: ScenarioRuntime): Promise<void> {
  const scenario = "instructor_authoring";
  const session = await runtime.open(runtime.record(scenario, "draft_list"));
  try {
    await enterInstructor(session.page);
    await session.page
      .getByRole("navigation", { name: "Ribbon tabs", exact: true })
      .getByRole("link", { name: "Questions", exact: true })
      .click();
    await session.page
      .getByRole("heading", { name: "Search Question Library", exact: true })
      .waitFor();
    await session.page.getByRole("link", { name: "My Draft Questions", exact: true }).click();
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
    await session.page
      .getByRole("combobox", { name: "Discipline (required)", exact: true })
      .selectOption({ label: "Biology" });
    await session.page
      .getByRole("combobox", { name: "Subject (required)", exact: true })
      .selectOption({ label: "Biochemistry" });
    await session.page.getByRole("button", { name: "Confirm and publish", exact: true }).click();
    await session.page.getByRole("heading", { name: "Published", exact: true }).waitFor();
    await captureCheckpoint(runtime, scenario, "published_result", session);
    await session.page.getByRole("link", { name: "Open question library", exact: true }).click();
    await session.page.getByLabel("Search published questions").fill(AUTHORING_TITLE);
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
    await session.page
      .getByRole("navigation", { name: "Ribbon tabs", exact: true })
      .getByRole("link", { name: "Courses", exact: true })
      .click();
    await session.page
      .getByRole("heading", { name: "Course Instances you teach", exact: true })
      .waitFor();
    await session.page.getByRole("link", { name: "My Blueprint Courses", exact: true }).click();
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
    await session.page
      .getByLabel("Blueprint Course long name", { exact: true })
      .fill(BLUEPRINT_PICKER_TITLE);
    await session.page
      .getByRole("combobox", { name: "Discipline (required)", exact: true })
      .selectOption({ label: "Biology" });
    await session.page
      .getByRole("combobox", { name: /^First Assessment Type/u })
      .selectOption("practice_question_assignment");
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

async function instructorPublicBlueprintSearch(runtime: ScenarioRuntime): Promise<void> {
  const scenario = "instructor_public_blueprint_search";
  const session = await runtime.open(runtime.record(scenario, "filtered_results"));
  try {
    await enterInstructor(session.page);
    await session.page
      .getByRole("navigation", { name: "Ribbon tabs", exact: true })
      .getByRole("link", { name: "Courses", exact: true })
      .click();
    await session.page
      .getByRole("navigation", { name: "Ribbon tasks", exact: true })
      .getByRole("link", { name: "Search Public Blueprint Courses", exact: true })
      .click();
    await session.page
      .getByRole("heading", { level: 1, name: "Search Public Blueprint Courses", exact: true })
      .waitFor();
    await session.page.getByLabel("Blueprint Course name", { exact: true }).fill("Biochemistry");
    await session.page.getByRole("button", { name: "Search", exact: true }).click();
    await session.page
      .getByRole("status")
      .filter({ hasText: /Public Blueprint Courses? shown for "Biochemistry"\./u })
      .waitFor();
    await session.page
      .getByRole("heading", { level: 3, name: COURSE_TITLE, exact: true })
      .waitFor();
    await captureCheckpoint(runtime, scenario, "filtered_results", session);
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
    await page.getByRole("link", { name: "Create Assessment", exact: true }).click();
    await page.getByRole("heading", { name: "Create an Assessment", exact: true }).waitFor();
    await captureCheckpoint(runtime, scenario, "assignment_creation", session);
    await page.getByLabel("Assessment title").fill(ASSIGNMENT_TITLE);
    await page
      .getByRole("combobox", { name: /^Assessment Type/u })
      .selectOption("practice_question_assignment");
    await page.getByRole("button", { name: "Create Assessment", exact: true }).click();
    await page.getByRole("heading", { name: "Assessment Question Editor", exact: true }).waitFor();
    await page.getByRole("button", { name: "Add Question", exact: true }).first().click();
    await captureCheckpoint(runtime, scenario, "assignment_questions_draft", session);
    await page.getByRole("button", { name: "Save Questions and order", exact: true }).click();
    await page
      .getByText("Questions and order saved. Review Assessment Properties when you are ready.")
      .waitFor();
    await page.getByRole("link", { name: "Review Assessment Properties", exact: true }).click();
    await page
      .getByRole("heading", { name: "Assessment Properties Editor", exact: true })
      .waitFor();
    await page
      .getByRole("group", { name: "Due date and time", exact: true })
      .locator('input[type="date"]')
      .fill("2026-12-01");
    await page.getByLabel("Due time").fill("12:00");
    await page.getByLabel("Time limit in seconds").fill("1800");
    await page.getByRole("combobox", { name: /^Late-work rule/u }).selectOption("mark_late");
    await page.getByText("Saved", { exact: true }).waitFor();
    await page.getByRole("link", { name: "Open Student View", exact: true }).click();
    await page.locator('section[aria-label="Student View"]').waitFor();
    await page.getByRole("note", { name: "Student View preview", exact: true }).waitFor();
    await page.getByRole("heading", { name: ASSIGNMENT_TITLE, exact: true }).waitFor();
    await runtime.capture(session, runtime.record(scenario, "assignment_delivery_check"));
    await page.getByRole("link", { name: "Return to assessment", exact: true }).click();
    await page.getByRole("link", { name: "Assessment Properties Editor", exact: true }).click();
    await page
      .getByRole("heading", { name: "Assessment Properties Editor", exact: true })
      .waitFor();
    const readiness = page.getByRole("button", { name: "Check release readiness", exact: true });
    await readiness.focus();
    await page.keyboard.press("Enter");
    await page.getByRole("heading", { name: "Ready to release", exact: true }).waitFor();
    await page
      .getByText("Release readiness checked. This saved assessment is ready to release.")
      .waitFor();
    await page.getByRole("button", { name: "Release assessment", exact: true }).click();
    await page.getByText(/^Assessment released\. Current edit number: [1-9][0-9]*\.$/u).waitFor();
    await captureCheckpoint(runtime, scenario, "assignment_policies_released", session);
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
      "assignments_due_soon_empty",
    ],
    run: seededInstructor,
  },
  {
    id: "instructor_library",
    checkpoints: [
      "library_default",
      "library_filtered",
      "published_question_detail",
      "library_browse",
    ],
    run: instructorLibrary,
  },
  {
    id: "instructor_profile",
    checkpoints: ["default"],
    run: instructorProfile,
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
    id: "instructor_public_blueprint_search",
    checkpoints: ["filtered_results"],
    run: instructorPublicBlueprintSearch,
  },
  {
    id: "instructor_assignment",
    checkpoints: [
      "assignment_creation",
      "assignment_questions_draft",
      "assignment_delivery_check",
      "assignment_policies_released",
    ],
    run: instructorAssignment,
  },
];
