// scenarios_instructor.ts - Instructor survey and visible mutation checkpoints.
// Selector contract: shared Course actions live in visible_workflows.ts:32; primary headings are
// owned by src/pages/course_list_page.tsx:149, src/pages/library_page.tsx:163,
// src/pages/question_drafts_page.tsx:102, assessment_workspace_create_page.tsx, and the focused
// Assessment workspace pages.
// Saved Draft and publication-review states are owned by
// src/features/ple_question_json_authoring/question_json_editor_page.tsx:908.
// Publication completion is intentionally outside this corpus until the configured
// Bloom-classification provider makes the ordinary publication journey available.

import type { Page } from "playwright";

import type { ScenarioRuntime } from "./runtime";
import { viewportCoverage, type ScenarioDefinition } from "./scenario_types";
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
  checkpoint: string,
  pageSession: Awaited<ReturnType<ScenarioRuntime["open"]>>,
): Promise<void> {
  await scrollTop(pageSession.page);
  await runtime.captureCheckpoint(pageSession, checkpoint);
}

async function openInstructorProfile(page: Page): Promise<void> {
  await enterInstructor(page);
  await page.getByRole("button", { name: "Profile", exact: true }).click();
  await page.getByRole("menuitem", { name: "Profile", exact: true }).click();
  await page.getByRole("heading", { level: 1, name: "Your profile", exact: true }).waitFor();
  await page.getByRole("heading", { level: 2, name: "Profile image", exact: true }).waitFor();
  await page.locator(".profile-thumbnail-placeholder").waitFor();
}

async function navigateInstructorLibraryBrowse(page: Page): Promise<void> {
  await page
    .getByRole("navigation", { name: "Ribbon tabs", exact: true })
    .getByRole("link", { name: "Questions", exact: true })
    .click();
  await page
    .getByRole("navigation", { name: "Ribbon tasks", exact: true })
    .getByRole("link", { name: "Browse Question Library", exact: true })
    .click();
  await page.getByRole("heading", { name: "Browse Question Library", exact: true }).waitFor();
  await page.getByRole("heading", { name: "Subjects", exact: true }).waitFor();
}

async function openInstructorDraftList(page: Page): Promise<void> {
  await enterInstructor(page);
  await page
    .getByRole("navigation", { name: "Ribbon tabs", exact: true })
    .getByRole("link", { name: "Questions", exact: true })
    .click();
  await page.getByRole("heading", { name: "Search Question Library", exact: true }).waitFor();
  await page.getByRole("link", { name: "My Draft Questions", exact: true }).click();
  await page.getByRole("heading", { name: "My Draft Questions", exact: true }).waitFor();
  await page.getByText("Loading your private Draft Questions...", { exact: true }).waitFor({
    state: "hidden",
  });
}

async function seededInstructor(runtime: ScenarioRuntime): Promise<void> {
  const session = await runtime.open("course_list");
  try {
    await enterInstructor(session.page);
    await session.page
      .getByRole("navigation", { name: "Ribbon tabs", exact: true })
      .getByRole("link", { name: "Courses", exact: true })
      .click();
    await session.page.getByRole("heading", { name: "My Active Courses", exact: true }).waitFor();
    await captureCheckpoint(runtime, "course_list", session);
    await session.page
      .getByRole("navigation", { name: "Ribbon tasks", exact: true })
      .getByRole("link", { name: "My Inactive Courses", exact: true })
      .click();
    await session.page.getByRole("heading", { name: "My Inactive Courses", exact: true }).waitFor();
    const inactiveCourseList = session.page.getByLabel("Inactive Course Instances", {
      exact: true,
    });
    // A zero-row list has no height, so Playwright reports it hidden; attachment is the state.
    await inactiveCourseList.waitFor({ state: "attached" });
    if ((await inactiveCourseList.locator(".instructor-list__row--course").count()) !== 0) {
      throw new Error("The inactive Course list must have zero Course rows for this scenario.");
    }
    await captureCheckpoint(runtime, "inactive_courses_list", session);
    await session.page.getByRole("link", { name: "My Active Courses", exact: true }).click();
    await session.page.getByRole("heading", { name: "My Active Courses", exact: true }).waitFor();
    const seededCourse = courseCard(session.page, COURSE_TITLE);
    await seededCourse.getByRole("link", { name: "Open Course Instance", exact: true }).click();
    await session.page.getByRole("heading", { name: COURSE_TITLE, exact: true }).waitFor();
    await captureCheckpoint(runtime, "course_assignment_workspace", session);
    await session.page.getByRole("link", { name: "Open Students", exact: true }).click();
    await session.page.getByRole("heading", { name: "Students", exact: true }).waitFor();
    await session.page.getByText("BIO301-AVERY", { exact: true }).waitFor();
    await captureCheckpoint(runtime, "course_roster_active", session);
    await session.page.locator("summary").filter({ hasText: "Roster tools" }).click();
    await session.page
      .getByLabel("Email, roster ID, Course roster name")
      .fill(
        "screenshot.pending@biology.roosevelt.edu,screenshot-pending,Synthetic Pending Student",
      );
    await session.page.getByRole("button", { name: "Import roster", exact: true }).click();
    await session.page.getByText("screenshot-pending", { exact: true }).waitFor();
    await captureCheckpoint(runtime, "course_roster_pending_invitation", session);
    await session.page
      .getByRole("navigation", { name: "Ribbon tasks", exact: true })
      .getByRole("link", { name: "Gradebook", exact: true })
      .click();
    await session.page.locator('[data-route-surface="gradebook"]').waitFor();
    await session.page.getByText("BIO301-JACK", { exact: true }).first().waitFor();
    await captureCheckpoint(runtime, "gradebook", session);
    await session.page.locator(".ple-app-ribbon__brand").click();
    await session.page.waitForURL((url) => url.pathname === "/instructor");
    await session.page.locator('[data-route-surface="courses"]').waitFor();
    await session.page.getByRole("heading", { name: "My Active Courses", exact: true }).waitFor();
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
    await captureCheckpoint(runtime, "assignments_due_soon_empty", session);
  } finally {
    await runtime.close(session);
  }
}

async function instructorProfile(runtime: ScenarioRuntime): Promise<void> {
  const session = await runtime.open("default");
  try {
    await openInstructorProfile(session.page);
    await captureCheckpoint(runtime, "default", session);
  } finally {
    await runtime.close(session);
  }
}

async function instructorLibrary(runtime: ScenarioRuntime): Promise<void> {
  const session = await runtime.open("library_default");
  try {
    await enterInstructor(session.page);
    await captureCheckpoint(runtime, "library_default", session);
    await session.page.getByLabel("Search published questions").fill("charged functional");
    await session.page
      .getByRole("heading", { name: PUBLISHED_NATIVE_TITLE, exact: true })
      .waitFor();
    await captureCheckpoint(runtime, "library_filtered", session);
    const result = session.page.locator(".record-list__row").filter({
      has: session.page.getByRole("heading", { name: PUBLISHED_NATIVE_TITLE, exact: true }),
    });
    await result.getByRole("link", { name: "Open question", exact: true }).click();
    await session.page.getByRole("region", { name: "Question prompt", exact: true }).waitFor();
    await captureCheckpoint(runtime, "published_question_detail", session);
    await navigateInstructorLibraryBrowse(session.page);
    await captureCheckpoint(runtime, "library_browse", session);
  } finally {
    await runtime.close(session);
  }
}

async function instructorAuthoring(runtime: ScenarioRuntime): Promise<void> {
  const session = await runtime.open("draft_list");
  try {
    await openInstructorDraftList(session.page);
    await captureCheckpoint(runtime, "draft_list", session);
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
    await captureCheckpoint(runtime, "saved_editor", session);
    await session.page
      .getByRole("button", { name: "Review publication changes", exact: true })
      .click();
    const questionAuthors = session.page.getByLabel("Question Authors");
    await questionAuthors.waitFor();
    // The review section renders below the editor form, so capture it in view.
    await questionAuthors.scrollIntoViewIfNeeded();
    await runtime.captureCheckpoint(session, "publication_review");
  } finally {
    await runtime.close(session);
  }
}

async function instructorBlueprint(runtime: ScenarioRuntime): Promise<void> {
  const session = await runtime.open("blueprint_list");
  try {
    await enterInstructor(session.page);
    await session.page
      .getByRole("navigation", { name: "Ribbon tabs", exact: true })
      .getByRole("link", { name: "Courses", exact: true })
      .click();
    await session.page.getByRole("heading", { name: "My Active Courses", exact: true }).waitFor();
    await session.page.getByRole("link", { name: "My Blueprint Courses", exact: true }).click();
    await session.page
      .getByRole("heading", { name: "Build reusable course structure", exact: true })
      .waitFor();
    await session.page.getByText(COURSE_TITLE, { exact: true }).waitFor();
    await captureCheckpoint(runtime, "blueprint_list", session);
    await session.page.getByText(COURSE_TITLE, { exact: true }).click();
    await session.page.getByRole("heading", { name: COURSE_TITLE, exact: true }).waitFor();
    await captureCheckpoint(runtime, "blueprint_detail", session);
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
    await captureCheckpoint(runtime, "blueprint_question_picker", session);
  } finally {
    await runtime.close(session);
  }
}

async function instructorPublicBlueprintSearch(runtime: ScenarioRuntime): Promise<void> {
  const session = await runtime.open("filtered_results");
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
      .filter({ hasText: /^[0-9,]+ Public Blueprint Courses? shown\.$/u })
      .waitFor();
    await session.page
      .getByText('Applied search: "Biochemistry"; all promotions; all classifications.', {
        exact: true,
      })
      .waitFor();
    await session.page
      .getByRole("heading", { level: 3, name: COURSE_TITLE, exact: true })
      .waitFor();
    await captureCheckpoint(runtime, "filtered_results", session);
  } finally {
    await runtime.close(session);
  }
}

async function instructorAssignment(runtime: ScenarioRuntime): Promise<void> {
  const session = await runtime.open("assignment_creation");
  const page = session.page;
  try {
    await enterInstructor(page);
    await openInstructorCourse(page);
    await page.getByRole("link", { name: "Create Assessment", exact: true }).click();
    await page.getByRole("heading", { name: "Create an Assessment", exact: true }).waitFor();
    await captureCheckpoint(runtime, "assignment_creation", session);
    await page.getByLabel("Assessment title").fill(ASSIGNMENT_TITLE);
    await page
      .getByRole("combobox", { name: /^Assessment Type/u })
      .selectOption("practice_question_assignment");
    await page.getByRole("button", { name: "Create Assessment", exact: true }).click();
    await page.getByRole("heading", { name: "Assessment Question Editor", exact: true }).waitFor();
    await page.getByRole("button", { name: "Add Question", exact: true }).first().click();
    await captureCheckpoint(runtime, "assignment_questions_draft", session);
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
    await page
      .getByLabel(
        /^Assessment duration override in minutes \(optional, maximum 720 minutes \/ 12 hours\)/u,
      )
      .fill("30");
    await page.getByRole("combobox", { name: /^Late-work rule/u }).selectOption("mark_late");
    await page.getByText("Saved", { exact: true }).waitFor();
    await page.getByRole("link", { name: "Open Student View", exact: true }).click();
    await page.getByRole("note", { name: "Student View preview", exact: true }).waitFor();
    await page.getByRole("heading", { name: ASSIGNMENT_TITLE, exact: true }).waitFor();
    await runtime.captureCheckpoint(session, "assignment_delivery_check");
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
    await captureCheckpoint(runtime, "assignment_policies_released", session);
  } finally {
    await runtime.close(session);
  }
}

export const INSTRUCTOR_SCENARIOS: ReadonlyArray<ScenarioDefinition> = [
  {
    id: "instructor_seeded",
    role: "instructor",
    captures: [
      {
        checkpoint: "course_list",
        area: "courses",
        workflow: "seeded teaching course",
        state: "course list",
        viewport: "laptop",
        privacyProfile: "instructor_answer_free",
        caption: "Instructor Course Instances",
        featured: true,
      },
      {
        checkpoint: "inactive_courses_list",
        area: "courses",
        workflow: "past teaching courses",
        state: "zero past Course Instances",
        viewport: "laptop",
        privacyProfile: "instructor_answer_free",
        caption: "Inactive Courses list",
      },
      {
        checkpoint: "course_assignment_workspace",
        area: "courses",
        workflow: "seeded teaching course",
        state: "assignment workspace",
        viewport: "laptop",
        privacyProfile: "instructor_answer_free",
        caption: "Course Assignment workspace",
      },
      {
        checkpoint: "course_roster_active",
        area: "courses",
        workflow: "roster",
        state: "active students",
        viewport: "laptop",
        privacyProfile: "instructor_answer_free",
        caption: "Active Course roster",
      },
      {
        checkpoint: "course_roster_pending_invitation",
        area: "courses",
        workflow: "roster",
        state: "pending invitation",
        viewport: "laptop",
        privacyProfile: "instructor_answer_free",
        caption: "Roster with a pending invitation",
      },
      {
        checkpoint: "gradebook",
        area: "grading",
        workflow: "seeded teaching course",
        state: "mixed progress",
        viewport: "laptop",
        privacyProfile: "instructor_answer_free",
        caption: "Answer-free Gradebook",
        featured: true,
      },
      {
        checkpoint: "assignments_due_soon_empty",
        area: "assignments",
        workflow: "upcoming deadlines",
        state: "no current deadlines",
        viewport: "laptop",
        privacyProfile: "instructor_answer_free",
        caption: "Assignments Due Soon without current deadlines",
      },
    ],
    viewportCoverage: viewportCoverage(["laptop"], {
      tablet: { target: "course_list", reason: "Instructor laptop capture is the representative." },
      phone: { target: "course_list", reason: "Instructor laptop capture is the representative." },
      square: { target: "course_list", reason: "Instructor laptop capture is the representative." },
    }),
    run: seededInstructor,
  },
  {
    id: "instructor_library",
    role: "instructor",
    captures: [
      {
        checkpoint: "library_default",
        area: "question library",
        workflow: "question discovery",
        state: "default",
        viewport: "laptop",
        privacyProfile: "instructor_answer_free",
        caption: "Question Library",
        featured: true,
      },
      {
        checkpoint: "library_filtered",
        area: "question library",
        workflow: "question discovery",
        state: "filtered",
        viewport: "laptop",
        privacyProfile: "instructor_answer_free",
        caption: "Filtered Question Library",
      },
      {
        checkpoint: "published_question_detail",
        area: "question library",
        workflow: "question discovery",
        state: "published detail",
        viewport: "laptop",
        privacyProfile: "instructor_answer_free",
        caption: "Published Question detail",
      },
      {
        checkpoint: "library_browse",
        area: "question library",
        workflow: "question discovery",
        state: "browse overview",
        viewport: "laptop",
        privacyProfile: "instructor_answer_free",
        caption: "Browse Question Library",
      },
    ],
    viewportCoverage: viewportCoverage(["laptop"], {
      tablet: {
        target: "library_default",
        reason: "Instructor laptop capture is the representative.",
      },
      phone: {
        target: "library_default",
        reason: "Instructor laptop capture is the representative.",
      },
      square: {
        target: "library_default",
        reason: "Instructor laptop capture is the representative.",
      },
    }),
    run: instructorLibrary,
  },
  {
    id: "instructor_profile",
    role: "instructor",
    captures: [
      {
        checkpoint: "default",
        area: "account",
        workflow: "profile preferences",
        state: "default profile",
        viewport: "laptop",
        privacyProfile: "instructor_answer_free",
        caption: "Instructor Profile",
      },
    ],
    viewportCoverage: viewportCoverage(["laptop"], {
      tablet: { target: "default", reason: "Instructor laptop capture is the representative." },
      phone: { target: "default", reason: "Instructor laptop capture is the representative." },
      square: { target: "default", reason: "Instructor laptop capture is the representative." },
    }),
    run: instructorProfile,
  },
  {
    id: "instructor_authoring",
    role: "instructor",
    captures: [
      {
        checkpoint: "draft_list",
        area: "question authoring",
        workflow: "publication",
        state: "draft list",
        viewport: "laptop",
        privacyProfile: "instructor_answer_free",
        caption: "Private Question drafts",
      },
      {
        checkpoint: "saved_editor",
        area: "question authoring",
        workflow: "publication",
        state: "saved draft",
        viewport: "laptop",
        privacyProfile: "instructor_answer_free",
        caption: "Saved private Question editor",
      },
      {
        checkpoint: "publication_review",
        area: "question authoring",
        workflow: "publication",
        state: "review",
        viewport: "laptop",
        privacyProfile: "instructor_answer_free",
        caption: "Question publication review",
      },
    ],
    viewportCoverage: viewportCoverage(["laptop"], {
      tablet: { target: "draft_list", reason: "Instructor laptop capture is the representative." },
      phone: { target: "draft_list", reason: "Instructor laptop capture is the representative." },
      square: { target: "draft_list", reason: "Instructor laptop capture is the representative." },
    }),
    run: instructorAuthoring,
  },
  {
    id: "instructor_blueprint",
    role: "instructor",
    captures: [
      {
        checkpoint: "blueprint_list",
        area: "blueprint courses",
        workflow: "reusable course design",
        state: "list",
        viewport: "laptop",
        privacyProfile: "instructor_answer_free",
        caption: "Blueprint Courses",
      },
      {
        checkpoint: "blueprint_detail",
        area: "blueprint courses",
        workflow: "reusable course design",
        state: "detail",
        viewport: "laptop",
        privacyProfile: "instructor_answer_free",
        caption: "Blueprint Course detail",
      },
      {
        checkpoint: "blueprint_question_picker",
        area: "blueprint courses",
        workflow: "reusable course design",
        state: "question picker",
        viewport: "laptop",
        privacyProfile: "instructor_answer_free",
        caption: "Blueprint Question picker",
      },
    ],
    viewportCoverage: viewportCoverage(["laptop"], {
      tablet: {
        target: "blueprint_list",
        reason: "Instructor laptop capture is the representative.",
      },
      phone: {
        target: "blueprint_list",
        reason: "Instructor laptop capture is the representative.",
      },
      square: {
        target: "blueprint_list",
        reason: "Instructor laptop capture is the representative.",
      },
    }),
    run: instructorBlueprint,
  },
  {
    id: "instructor_public_blueprint_search",
    role: "instructor",
    captures: [
      {
        checkpoint: "filtered_results",
        area: "blueprint courses",
        workflow: "public blueprint discovery",
        state: "filtered results",
        viewport: "laptop",
        privacyProfile: "instructor_answer_free",
        caption: "Search Public Blueprint Courses",
      },
    ],
    viewportCoverage: viewportCoverage(["laptop"], {
      tablet: {
        target: "filtered_results",
        reason: "Instructor laptop capture is the representative.",
      },
      phone: {
        target: "filtered_results",
        reason: "Instructor laptop capture is the representative.",
      },
      square: {
        target: "filtered_results",
        reason: "Instructor laptop capture is the representative.",
      },
    }),
    run: instructorPublicBlueprintSearch,
  },
  {
    id: "instructor_assignment",
    role: "instructor",
    captures: [
      {
        checkpoint: "assignment_creation",
        area: "assignments",
        workflow: "assignment release",
        state: "creation",
        viewport: "laptop",
        privacyProfile: "instructor_answer_free",
        caption: "Assignment creation",
      },
      {
        checkpoint: "assignment_questions_draft",
        area: "assignments",
        workflow: "assignment release",
        state: "draft",
        viewport: "laptop",
        privacyProfile: "instructor_answer_free",
        caption: "Draft Assignment Questions",
      },
      {
        checkpoint: "assignment_delivery_check",
        area: "assignments",
        workflow: "assignment release",
        state: "answer-free preview",
        viewport: "laptop",
        privacyProfile: "instructor_answer_free",
        caption: "Answer-free Assignment Preview",
      },
      {
        checkpoint: "assignment_policies_released",
        area: "assignments",
        workflow: "assignment release",
        state: "released",
        viewport: "laptop",
        privacyProfile: "instructor_answer_free",
        caption: "Released Assignment Policies",
        featured: true,
      },
    ],
    viewportCoverage: viewportCoverage(["laptop"], {
      tablet: {
        target: "assignment_creation",
        reason: "Instructor laptop capture is the representative.",
      },
      phone: {
        target: "assignment_creation",
        reason: "Instructor laptop capture is the representative.",
      },
      square: {
        target: "assignment_creation",
        reason: "Instructor laptop capture is the representative.",
      },
    }),
    run: instructorAssignment,
  },
];
