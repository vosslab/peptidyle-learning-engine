// scenarios_instructor.ts - Instructor survey and visible mutation checkpoints.
// Selector contract: shared Course actions live in visible_workflows.ts:32; primary headings are
// owned by src/pages/course_list_page.tsx:149, src/pages/library_page.tsx:163,
// src/pages/question_drafts_page.tsx:102, assignment_workspace_create_page.tsx, and the focused
// Assignment workspace pages.

import type { ScenarioRuntime } from "./runtime";
import { monitorCapturePrivacy } from "./privacy_profiles";
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
    await session.page.locator(".ple-app-ribbon__brand").click();
    await session.page.waitForURL((url) => url.pathname === "/");
    await session.page.locator('[data-route-surface="courses"]').waitFor();
    await session.page
      .getByRole("heading", { name: "Course Instances you teach", exact: true })
      .waitFor();
    await session.page
      .getByRole("navigation", { name: "Ribbon tabs", exact: true })
      .getByRole("link", { name: "Assignments", exact: true })
      .click();
    await session.page
      .getByRole("heading", { level: 1, name: "Assignments Due Soon", exact: true })
      .waitFor();
    await Promise.race([
      session.page
        .getByRole("heading", { level: 2, name: "No Assignments are due in the next 7 days." })
        .waitFor(),
      session.page.getByRole("list", { name: "Assignments due in the next 7 days" }).waitFor(),
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
    await session.page
      .locator('[data-ribbon-context-control="profile"]')
      .getByText("Profile", { exact: true })
      .click();
    await session.page.getByRole("heading", { level: 1, name: "Profile", exact: true }).waitFor();
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
    await session.page
      .getByRole("navigation", { name: "Ribbon tabs", exact: true })
      .getByRole("link", { name: "Questions", exact: true })
      .click();
    await session.page.getByRole("heading", { name: "Question library", exact: true }).waitFor();
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
    await page.getByRole("heading", { name: "Create an Assignment", exact: true }).waitFor();
    await captureCheckpoint(runtime, scenario, "assignment_creation", session);
    await page.getByLabel("Assignment title").fill(ASSIGNMENT_TITLE);
    await page.getByRole("button", { name: "Create Assignment", exact: true }).click();
    await page.getByRole("heading", { name: "Questions", exact: true }).waitFor();
    await page.getByRole("button", { name: "Search question library", exact: true }).click();
    const picker = page.getByRole("dialog", { name: "Choose assignment questions", exact: true });
    await picker.getByRole("button", { name: "Search questions", exact: true }).click();
    await picker.locator(".question-picker-result input").first().check();
    await picker.getByRole("button", { name: "Add selected questions", exact: true }).click();
    await captureCheckpoint(runtime, scenario, "assignment_questions_draft", session);
    await page.getByRole("button", { name: "Save Questions and order", exact: true }).click();
    await page
      .getByText("Questions and order saved. Review assignment policies when you are ready.")
      .waitFor();
    await page.getByRole("link", { name: "Review assignment policies", exact: true }).click();
    await page.getByRole("heading", { name: "Policies", exact: true }).waitFor();
    await page
      .getByRole("group", { name: "Due date and time", exact: true })
      .locator('input[type="date"]')
      .fill("2026-12-01");
    await page.getByLabel("Due time").fill("12:00");
    await page.getByLabel("Time limit in seconds").fill("1800");
    await page.getByLabel("Late-work rule").selectOption("mark_late");
    await page.getByRole("button", { name: "Save assignment policies", exact: true }).click();
    await page
      .getByText("Assignment policies saved. The current assignment now uses the new revision.")
      .waitFor();
    const deliveryCheckPage = session.context.waitForEvent("page");
    await page.getByRole("link", { name: "Check assignment delivery", exact: true }).click();
    const deliveryCheck = await deliveryCheckPage;
    await deliveryCheck
      .getByRole("heading", { name: "Assignment delivery check", exact: true })
      .waitFor();
    await deliveryCheck
      .getByText("Preview only - no Student work or grades are created.", { exact: true })
      .waitFor();
    const deliveryCheckSession = {
      ...session,
      page: deliveryCheck,
      pageErrors: [],
      privacy: monitorCapturePrivacy(deliveryCheck, runtime.entryUrl.origin),
    };
    await runtime.capture(
      deliveryCheckSession,
      runtime.record(scenario, "assignment_delivery_check"),
    );
    await deliveryCheck
      .getByRole("link", { name: "Return to assignment policies", exact: true })
      .waitFor();
    await deliveryCheck.close();
    const readiness = page.getByRole("button", { name: "Check release readiness", exact: true });
    await readiness.focus();
    await page.keyboard.press("Enter");
    await page.getByRole("heading", { name: "Ready to release", exact: true }).waitFor();
    await page
      .getByText("Release readiness checked. This saved assignment is ready to release.")
      .waitFor();
    await page.getByRole("button", { name: "Release assignment", exact: true }).click();
    await page.getByText("Assignment released as revision 1.").waitFor();
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
    checkpoints: ["library_default", "library_filtered", "published_question_detail"],
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
