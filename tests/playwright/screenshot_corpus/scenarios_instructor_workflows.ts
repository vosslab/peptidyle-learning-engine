// Instructor workflow implementations for the screenshot corpus.
// Selector contract: shared Course actions live in visible_workflows.ts:32; primary headings are
// owned by src/pages/course_list_page.tsx:149, src/pages/library_page.tsx:163,
// src/pages/question_drafts_page.tsx:102, assessment_workspace_create_page.tsx, and the focused
// Assessment workspace pages.
// Saved Draft and publication-review states are owned by
// src/features/ple_question_json_authoring/question_json_editor_page.tsx:908.

import type { Page } from "playwright";

import type { ScenarioRuntime } from "./runtime";
import {
  COURSE_TITLE,
  searchNativeMcLibraryQuestion,
  courseCard,
  enterInstructor,
  filterBrowseToMultipleChoice,
  followCaptureLink,
  openInstructorCourse,
  openInstructorLibraryBrowse,
  scrollTop,
} from "./visible_workflows";

const AUTHORING_TITLE = "Screenshot corpus peptide geometry";
const BLUEPRINT_PICKER_TITLE = "Screenshot corpus Blueprint preview";
const ASSIGNMENT_TITLE = "Screenshot corpus peptide practice";

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
  await page.getByRole("menuitem", { name: "Profile settings", exact: true }).click();
  await page.getByRole("main", { name: "Profile settings", exact: true }).waitFor();
  await page.getByRole("heading", { level: 2, name: "Profile image", exact: true }).waitFor();
  await page.locator(".profile-thumbnail-placeholder").waitFor();
}

async function openInstructorDraftList(page: Page): Promise<void> {
  await enterInstructor(page);
  await page
    .getByRole("navigation", { name: "Ribbon tabs", exact: true })
    .getByRole("link", { name: "Questions", exact: true })
    .click();
  await page.getByRole("main", { name: "Question Library", exact: true }).waitFor();
  await page.getByRole("link", { name: "My Drafts", exact: true }).click();
  await page.getByRole("main", { name: "My Drafts", exact: true }).waitFor();
  await page
    .locator('nav[aria-label="Private Draft Questions pages"][aria-busy="false"]')
    .waitFor();
}

export async function seededInstructor(runtime: ScenarioRuntime): Promise<void> {
  const session = await runtime.open("course_list");
  try {
    await enterInstructor(session.page);
    await session.page
      .getByRole("navigation", { name: "Ribbon tabs", exact: true })
      .getByRole("link", { name: "Courses", exact: true })
      .click();
    await session.page.getByRole("main", { name: "My Active Courses", exact: true }).waitFor();
    await captureCheckpoint(runtime, "course_list", session);
    await session.page
      .getByRole("navigation", { name: "Ribbon tasks", exact: true })
      .getByRole("link", { name: "My Inactive Courses", exact: true })
      .click();
    await session.page.getByRole("main", { name: "My Inactive Courses", exact: true }).waitFor();
    const inactiveCourses = session.page.locator('[data-route-surface="inactiveCourses"]');
    await inactiveCourses
      .getByRole("heading", { name: "No inactive Courses", exact: true })
      .waitFor({ state: "visible" });
    await captureCheckpoint(runtime, "inactive_courses_list", session);
    await session.page.getByRole("link", { name: "My Active Courses", exact: true }).click();
    await session.page.getByRole("main", { name: "My Active Courses", exact: true }).waitFor();
    const seededCourse = courseCard(session.page, COURSE_TITLE);
    await followCaptureLink(
      session.page,
      seededCourse.getByRole("link", { name: "Open Course", exact: true }),
    );
    await session.page.getByRole("main", { name: COURSE_TITLE, exact: true }).waitFor();
    await captureCheckpoint(runtime, "course_assignment_workspace", session);
    await session.page.getByRole("link", { name: "Open Students", exact: true }).click();
    await session.page.getByRole("main", { name: "Students", exact: true }).waitFor();
    await session.page.getByText("BCHM355-AVERY", { exact: true }).waitFor();
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
      .getByRole("navigation", { name: "Breadcrumb", exact: true })
      .getByRole("link", { name: COURSE_TITLE, exact: true })
      .click();
    await session.page.getByRole("main", { name: COURSE_TITLE, exact: true }).waitFor();
    await session.page
      .getByRole("navigation", { name: "Course actions", exact: true })
      .getByRole("link", { name: "Gradebook", exact: true })
      .click();
    await session.page.locator('[data-route-surface="gradebook"]').waitFor();
    await session.page.getByText("BCHM355-JACK", { exact: true }).first().waitFor();
    await captureCheckpoint(runtime, "gradebook", session);
    await session.page.locator(".ple-app-ribbon__brand").click();
    await session.page.waitForURL((url) => url.pathname === "/instructor");
    await session.page.locator('[data-route-surface="courses"]').waitFor();
    await session.page.getByRole("main", { name: "My Active Courses", exact: true }).waitFor();
    await session.page
      .getByRole("navigation", { name: "Ribbon tabs", exact: true })
      .getByRole("link", { name: "Assessments", exact: true })
      .click();
    await session.page.getByRole("main", { name: "Assessments Due Soon", exact: true }).waitFor();
    await session.page.getByText("Loading Assessments due soon...", { exact: true }).waitFor({
      state: "hidden",
    });
    await Promise.race([
      session.page
        .getByRole("heading", { level: 2, name: "No Assessments are due in the next 7 days." })
        .waitFor(),
      session.page
        .getByRole("table", { name: "Assessments due in the next 7 days" })
        .locator("[data-record-id]")
        .first()
        .waitFor(),
    ]);
    await captureCheckpoint(runtime, "assignments_due_soon_empty", session);
  } finally {
    await runtime.close(session);
  }
}

export async function instructorProfile(runtime: ScenarioRuntime): Promise<void> {
  const session = await runtime.open("profile_preferences");
  try {
    await openInstructorProfile(session.page);
    await captureCheckpoint(runtime, "profile_preferences", session);
  } finally {
    await runtime.close(session);
  }
}

/** Captures the signed-in, cross-Account Instructor Profile surface from a real Profile link ID. */
export async function instructorPublicProfile(runtime: ScenarioRuntime): Promise<void> {
  const session = await runtime.open("public_profile");
  try {
    await openInstructorProfile(session.page);
    const accountId = await session.page.locator('code[aria-label^="Account ID "]').textContent();
    if (accountId === null || !/^U[0-9A-HJKMNP-TV-Z]{8}$/u.test(accountId)) {
      throw new Error("The signed-in Instructor Profile must expose one canonical Account ID.");
    }
    const profileResponses: string[] = [];
    session.page.on("response", (response) => {
      if (response.url().includes("/api/instructor-profiles/")) {
        profileResponses.push(`${String(response.status())} ${response.url()}`);
      }
    });
    await session.page.goto(new URL(`/instructors/${accountId}`, session.page.url()).href);
    await session.page.locator('[data-route-surface="instructor-profile"]').waitFor();
    await session.page.getByRole("main", { name: "Instructor Profile", exact: true }).waitFor();
    const profileLink = session.page.getByRole("link", { name: /Elena Martinez/u });
    const unavailable = session.page.getByRole("alert");
    await Promise.race([profileLink.waitFor(), unavailable.waitFor()]);
    if (await unavailable.isVisible()) {
      throw new Error(
        `Instructor Profile was unavailable: ${(await unavailable.textContent()) ?? "no alert text"}; ` +
          `profile responses: ${profileResponses.join(", ") || "none"}`,
      );
    }
    await captureCheckpoint(runtime, "public_profile", session);
  } finally {
    await runtime.close(session);
  }
}

export async function instructorLibrary(runtime: ScenarioRuntime): Promise<void> {
  const session = await runtime.open("library_default");
  try {
    await enterInstructor(session.page);
    await captureCheckpoint(runtime, "library_default", session);
    await session.page.getByRole("link", { name: "My Pools", exact: true }).click();
    await session.page.getByRole("main", { name: "My Pools", exact: true }).waitFor();
    await session.page.getByRole("searchbox", { name: "Search My Pools" }).waitFor();
    await session.page
      .locator('nav[aria-label="My Question Pools pages"][aria-busy="false"]')
      .waitFor();
    await captureCheckpoint(runtime, "my_pools_default", session);
    await session.page.getByRole("link", { name: "Search Question Library", exact: true }).click();
    await session.page.getByRole("main", { name: "Question Library", exact: true }).waitFor();
    const question = await searchNativeMcLibraryQuestion(session.page);
    await captureCheckpoint(runtime, "library_filtered", session);
    await openInstructorLibraryBrowse(session.page);
    await captureCheckpoint(runtime, "library_browse", session);
    await filterBrowseToMultipleChoice(session.page);
    await session.page.getByRole("link", { name: "Search within results", exact: true }).click();
    await session.page.getByRole("main", { name: "Question Library", exact: true }).waitFor();
    await searchNativeMcLibraryQuestion(session.page);
    const result = session.page
      .locator("[data-record-id]")
      .filter({
        has: session.page.getByText(question.metadata.questionTitle, { exact: true }),
      })
      .filter({ has: session.page.getByRole("link", { name: "Open", exact: true }) })
      .first();
    await followCaptureLink(session.page, result.getByRole("link", { name: "Open", exact: true }));
    await session.page.getByRole("region", { name: "Question prompt", exact: true }).waitFor();
    await captureCheckpoint(runtime, "published_question_detail", session);
  } finally {
    await runtime.close(session);
  }
}

export async function instructorAuthoring(runtime: ScenarioRuntime): Promise<void> {
  const session = await runtime.open("draft_list");
  try {
    await openInstructorDraftList(session.page);
    await captureCheckpoint(runtime, "draft_list", session);
    await session.page.getByLabel("Question source format").selectOption("pleQuestionJson");
    await session.page.getByRole("button", { name: "Create Draft Question", exact: true }).click();
    await session.page.getByLabel("Question Title").fill(AUTHORING_TITLE);
    await session.page.getByLabel("Question License").selectOption("CC-BY-4.0");
    const saveMetadataButton = session.page.getByRole("button", {
      name: "Save Question metadata",
      exact: true,
    });
    await saveMetadataButton.click();
    await session.page
      .getByRole("button", { name: "Save Question metadata", exact: true, disabled: true })
      .waitFor();
    await session.page
      .getByRole("status", { name: "Private draft status" })
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

export async function instructorBlueprint(runtime: ScenarioRuntime): Promise<void> {
  const session = await runtime.open("blueprint_list");
  try {
    await enterInstructor(session.page);
    await session.page
      .getByRole("navigation", { name: "Ribbon tabs", exact: true })
      .getByRole("link", { name: "Courses", exact: true })
      .click();
    await session.page.getByRole("main", { name: "My Active Courses", exact: true }).waitFor();
    const blueprintResponses: string[] = [];
    const blueprintResponseBodies: Promise<void>[] = [];
    session.page.on("response", (response) => {
      if (response.url().includes("/api/course-blueprints")) {
        blueprintResponseBodies.push(
          response.text().then((body) => {
            blueprintResponses.push(`${String(response.status())} ${response.url()}: ${body}`);
          }),
        );
      }
    });
    await session.page.getByRole("link", { name: "My Blueprint Courses", exact: true }).click();
    await session.page
      .getByRole("main", { name: "My Blueprint Courses", exact: true })
      .getByRole("region", { name: "Blueprint Courses", exact: true })
      .waitFor();
    const seededCourseTitle = session.page.getByText(COURSE_TITLE, { exact: true });
    const unavailable = session.page.getByRole("alert");
    await Promise.race([seededCourseTitle.waitFor(), unavailable.waitFor()]);
    if (await unavailable.isVisible()) {
      await Promise.all(blueprintResponseBodies);
      throw new Error(
        `Blueprint Courses were unavailable: ${(await unavailable.textContent()) ?? "no alert text"}; ` +
          `Blueprint responses: ${blueprintResponses.join(", ") || "none"}`,
      );
    }
    await captureCheckpoint(runtime, "blueprint_list", session);
    await followCaptureLink(
      session.page,
      session.page
        .locator("[data-record-id]")
        .filter({ has: session.page.getByText(COURSE_TITLE, { exact: true }) })
        .getByRole("link", { name: "Open Blueprint Course", exact: true }),
    );
    await session.page.waitForURL(/\/blueprint-courses\/[^/]+$/u);
    await session.page.getByRole("main", { name: COURSE_TITLE, exact: true }).waitFor();
    await captureCheckpoint(runtime, "blueprint_detail", session);
    await session.page
      .getByRole("link", { name: "My Blueprint Courses", exact: true })
      .first()
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
    const picker = session.page.getByRole("dialog", {
      name: "Choose the first reusable Questions",
      exact: true,
    });
    await picker.getByRole("button", { name: "Search", exact: true }).click();
    await picker.getByRole("checkbox").first().waitFor({ state: "visible" });
    await captureCheckpoint(runtime, "blueprint_question_picker", session);
  } finally {
    await runtime.close(session);
  }
}

export async function instructorPublicBlueprintSearch(runtime: ScenarioRuntime): Promise<void> {
  const session = await runtime.open("filtered_results");
  try {
    await enterInstructor(session.page);
    await session.page
      .getByRole("navigation", { name: "Ribbon tabs", exact: true })
      .getByRole("link", { name: "Courses", exact: true })
      .click();
    await session.page
      .getByRole("navigation", { name: "Ribbon tasks", exact: true })
      .getByRole("link", { name: "Search Blueprint Courses", exact: true })
      .click();
    await session.page
      .getByRole("main", { name: "Search Blueprint Courses", exact: true })
      .waitFor();
    await session.page.getByLabel("Blueprint Course name", { exact: true }).fill("Biochemistry");
    await session.page.getByRole("button", { name: "Search", exact: true }).click();
    await session.page.getByText(/^[0-9,]+ shown on this page$/u).waitFor();
    await session.page.getByText(COURSE_TITLE, { exact: true }).waitFor();
    await captureCheckpoint(runtime, "filtered_results", session);
  } finally {
    await runtime.close(session);
  }
}

export async function instructorAssignment(runtime: ScenarioRuntime): Promise<void> {
  const session = await runtime.open("assignment_creation");
  const page = session.page;
  try {
    await enterInstructor(page);
    const question = await searchNativeMcLibraryQuestion(page);
    await openInstructorCourse(page);
    await page.getByRole("link", { name: "Create Assessment", exact: true }).click();
    await page.getByRole("main", { name: "New Assessment", exact: true }).waitFor();
    await captureCheckpoint(runtime, "assignment_creation", session);
    await page.getByLabel("Assessment title").fill(ASSIGNMENT_TITLE);
    await page
      .getByRole("combobox", { name: /^Assessment Type/u })
      .selectOption("practice_question_assignment");
    await page.getByRole("button", { name: "Create Assessment", exact: true }).click();
    await page.getByRole("main", { name: "Questions", exact: true }).waitFor();
    const available = page.getByRole("region", {
      name: "Available Assessment content",
      exact: true,
    });
    await available.getByRole("combobox", { name: "Show", exact: true }).selectOption("questions");
    await available
      .getByRole("combobox", { name: "Question membership", exact: true })
      .selectOption("all");
    // Search the actual published Question chosen from the current Library, including Pool members.
    await available
      .getByLabel("Search published Assessment content", { exact: true })
      .fill(question.metadata.questionTitle);
    await available.getByRole("button", { name: "Search", exact: true }).click();
    await available
      .locator(`[data-record-id="${question.questionId}"]`)
      .getByRole("button", { name: "Add", exact: true })
      .click();
    await page
      .getByRole("region", { name: "Assessment contents", exact: true })
      .getByText(question.metadata.questionTitle, { exact: true })
      .waitFor();
    await captureCheckpoint(runtime, "assignment_questions_draft", session);
    await page.getByRole("button", { name: "Save Questions and order", exact: true }).click();
    await page
      .getByText("Questions and order saved. Review Assessment Properties when you are ready.")
      .waitFor();
    await page.getByRole("link", { name: "Review Assessment Properties", exact: true }).click();
    await page.getByRole("main", { name: "Properties", exact: true }).waitFor();
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
    await page.getByRole("main", { name: "Student View", exact: true }).waitFor();
    await runtime.captureCheckpoint(session, "assignment_delivery_check");
    await page.getByRole("link", { name: "Return to assessment", exact: true }).click();
    await page.getByRole("link", { name: "Assessment Properties Editor", exact: true }).click();
    await page.getByRole("main", { name: "Properties", exact: true }).waitFor();
    const readiness = page.getByRole("button", { name: "Check release readiness", exact: true });
    await readiness.focus();
    await page.keyboard.press("Enter");
    await page.getByRole("heading", { name: "Ready to release", exact: true }).waitFor();
    await page.getByRole("button", { name: "Release assessment", exact: true }).click();
    await page.getByText(/^Assessment released\. Current edit number: [1-9][0-9]*\.$/u).waitFor();
    await captureCheckpoint(runtime, "assignment_policies_released", session);
  } finally {
    await runtime.close(session);
  }
}
