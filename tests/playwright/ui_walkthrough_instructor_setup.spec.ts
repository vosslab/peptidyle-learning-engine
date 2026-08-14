// Selector contract: visible labels in course_list_page.tsx, course_roster_page.tsx, and assignment_editor_page.tsx.

import { expect, test } from "./ui_walkthrough_fixture";

import {
  commitInstructorSetupState,
  type InstructorSetupFragment,
} from "./simulator/instructor_setup_state";
import { isAssignmentReference, isCourseReference } from "./simulator/public_references";
import { writeInstructorSetupCheckpoint } from "./simulator/instructor_setup_checkpoint";
import { tabTo } from "./simulator/keyboard_walkthrough";
import { credentialFromValidatedFile } from "./ui_walkthrough_config_factory";
import {
  captureDocumentationScreenshot,
  documentationScreenshotsEnabled,
} from "./docs_screenshot_capture";

test.describe.configure({ mode: "serial" });
test.setTimeout(120_000);

function elapsedSince(startedAt: number): number {
  return Math.round(performance.now() - startedAt);
}

test("J11/J12/J13 instructor visibly prepares a four-question Genetics assignment and hands off public IDs", async ({
  page,
  instructorSetupInputs,
}) => {
  test.skip(instructorSetupInputs === undefined, "requires the explicit instructor setup config");
  if (instructorSetupInputs === undefined) return;
  const inputs = instructorSetupInputs;
  writeInstructorSetupCheckpoint(inputs.instructorSetupCheckpointFile, "browser_ready");
  const startedAt = performance.now();
  const uniqueSuffix = `${inputs.masterSeedText}-${Date.now().toString(36)}`;
  const courseTitle = documentationScreenshotsEnabled(inputs.screenshotDirectory)
    ? `Fake Genetics Course ${Date.now().toString(36).slice(-6)}`
    : `Fall pilot instructor walkthrough ${uniqueSuffix}`;
  const assignmentTitle = documentationScreenshotsEnabled(inputs.screenshotDirectory)
    ? "Genetics Chapter 1 Practice"
    : `Mastery genetics practice ${uniqueSuffix}`;

  await page.goto("/");
  const credentialInput = page.getByLabel("Local development credential");
  await expect(credentialInput).toBeVisible();
  writeInstructorSetupCheckpoint(inputs.instructorSetupCheckpointFile, "login_visible");
  await tabTo(page, credentialInput);
  await expect(credentialInput).toBeFocused();
  await credentialInput.fill(credentialFromValidatedFile(inputs.credentialFile, "instructor"));
  const signIn = page.getByRole("button", { name: "Sign in locally", exact: true });
  await tabTo(page, signIn);
  await expect(signIn).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("heading", { name: "Courses you teach" })).toBeVisible();
  writeInstructorSetupCheckpoint(inputs.instructorSetupCheckpointFile, "signed_in");

  const courseTitleInput = page.getByLabel("Course title");
  await expect(courseTitleInput).toBeVisible();
  await tabTo(page, courseTitleInput);
  await expect(courseTitleInput).toBeFocused();
  await courseTitleInput.fill(courseTitle);
  const createCourse = page.getByRole("button", { name: "Create course", exact: true });
  await tabTo(page, createCourse);
  await expect(createCourse).toBeFocused();
  await page.keyboard.press("Enter");
  const courseCard = page.locator(".course-card", {
    has: page.getByRole("heading", { name: courseTitle, exact: true }),
  });
  await expect(courseCard).toHaveCount(1);
  await expect(courseCard).toBeVisible();
  writeInstructorSetupCheckpoint(inputs.instructorSetupCheckpointFile, "course_created");
  const openCourse = courseCard.getByRole("link", { name: "Open course", exact: true });
  await expect(openCourse).toBeVisible();
  await expect(openCourse).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.locator("[data-route-surface=courseAssignments]")).toBeVisible();
  await expect(page.getByRole("heading", { name: courseTitle, exact: true })).toBeVisible();
  const courseReference = new URL(page.url()).pathname.split("/")[2];
  if (!isCourseReference(courseReference)) throw new Error("visible course link is unavailable");
  writeInstructorSetupCheckpoint(inputs.instructorSetupCheckpointFile, "course_opened");
  await captureDocumentationScreenshot(
    page,
    "instructor_course_overview.png",
    undefined,
    undefined,
    inputs.screenshotDirectory,
  );
  const j11: InstructorSetupFragment = {
    schemaVersion: 2,
    journey: "J11",
    status: "PASS",
    elapsedMs: elapsedSince(startedAt),
    courseReference,
    visibleOutcomeCodes: ["visible_course_created", "visible_course_opened"],
    diagnostics: [],
  };
  const students = page.getByRole("link", { name: "Students", exact: true });
  await expect(students).toHaveCount(1);
  await expect(students).toBeVisible();
  await tabTo(page, students);
  await expect(students).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("heading", { name: "Students", exact: true })).toBeVisible();
  const emailEnrollmentRequests: string[] = [];
  page.on("request", (request) => {
    const pathname = new URL(request.url()).pathname;
    if (
      pathname.includes("/invitations") ||
      pathname.includes("/enrollment-policy") ||
      pathname.includes("/roster-imports")
    ) {
      emailEnrollmentRequests.push(pathname);
    }
  });
  await expect(page.getByRole("heading", { name: "Invite one student", exact: true })).toHaveCount(
    0,
  );
  await expect(page.getByRole("heading", { name: "Enrollment policy", exact: true })).toHaveCount(
    0,
  );
  await expect(page.getByRole("heading", { name: "Pending invitations", exact: true })).toHaveCount(
    0,
  );
  await expect(page.getByRole("heading", { name: "Import a CSV roster", exact: true })).toHaveCount(
    0,
  );
  await expect(page.getByRole("columnheader", { name: "Email", exact: true })).toHaveCount(0);
  await expect(page.getByRole("columnheader", { name: "Roster ID", exact: true })).toHaveCount(0);
  const addStudent = page.getByRole("button", { name: "Add Mary Fake Student", exact: true });
  await expect(addStudent).toBeVisible();
  await tabTo(page, addStudent);
  await expect(addStudent).toBeFocused();
  await page.keyboard.press("Enter");
  const activeRow = page.getByRole("row", { name: /Mary Fake Student.*active/u });
  await expect(activeRow).toHaveCount(1);
  await expect(activeRow).toHaveAttribute("tabindex", "-1");
  const activeStudentAnnouncement = page.getByRole("status").filter({
    hasText: "Mary Fake Student is now an active student in this course.",
  });
  await expect(activeStudentAnnouncement).toHaveCount(1);
  await expect(activeStudentAnnouncement).toHaveText(
    "Mary Fake Student is now an active student in this course.",
  );
  await expect(activeRow).toBeFocused();
  expect(emailEnrollmentRequests).toEqual([]);
  writeInstructorSetupCheckpoint(inputs.instructorSetupCheckpointFile, "student_active");
  if (documentationScreenshotsEnabled(inputs.screenshotDirectory)) {
    const addJack = page.getByRole("button", { name: "Add Jack Fake Student", exact: true });
    await tabTo(page, addJack);
    await expect(addJack).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(activeRow).toHaveCount(1);
    await expect(page.getByRole("row", { name: /Jack Fake Student.*active/u })).toBeFocused();
  }
  await captureDocumentationScreenshot(
    page,
    "instructor_roster_active_student.png",
    page.getByRole("heading", { name: "Course members", exact: true }),
    72,
    inputs.screenshotDirectory,
  );
  const j12: InstructorSetupFragment = {
    schemaVersion: 2,
    journey: "J12",
    status: "PASS",
    elapsedMs: elapsedSince(startedAt),
    courseReference,
    visibleOutcomeCodes: ["visible_local_student_active"],
    diagnostics: [],
  };
  const newAssignment = page.getByRole("link", { name: "New assignment", exact: true });
  await expect(newAssignment).toHaveCount(1);
  await expect(newAssignment).toBeVisible();
  await tabTo(page, newAssignment, "backward");
  await expect(newAssignment).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("heading", { name: "Create assignment", exact: true })).toBeVisible();
  writeInstructorSetupCheckpoint(inputs.instructorSetupCheckpointFile, "assignment_editor_opened");
  await expect(page.getByRole("radio", { name: "Timed", exact: true })).toBeChecked();
  await expect(page.getByLabel("Minutes per practice run")).toHaveValue("15");
  const assignmentTitleInput = page.getByLabel("Assignment title");
  await expect(assignmentTitleInput).toBeFocused();
  await assignmentTitleInput.fill(assignmentTitle);
  const addByQuestionId = page.getByText("Add by question ID", { exact: true });
  await expect(addByQuestionId).toBeVisible();
  await tabTo(page, addByQuestionId);
  await expect(addByQuestionId).toBeFocused();
  await page.keyboard.press("Enter");
  const directImport = page.getByLabel("Question IDs");
  await expect(directImport).toBeVisible();
  const search = page.getByLabel("Search published questions");
  await tabTo(page, search);
  await expect(search).toBeFocused();
  const searchCatalog = page.getByRole("button", { name: "Search catalog", exact: true });
  await page.context().grantPermissions(["clipboard-write"], {
    origin: new URL(inputs.baseUrl).origin,
  });
  const copiedDisplayIds = inputs.catalogDisplayIds;
  for (const [index, displayId] of copiedDisplayIds.entries()) {
    await search.fill(displayId);
    await tabTo(page, searchCatalog);
    await expect(searchCatalog).toBeFocused();
    await page.keyboard.press("Enter");
    const catalogRow = page.locator(".assignment-editor-catalog-results article", {
      has: page.locator("code", { hasText: new RegExp(`^${displayId}$`, "u") }),
    });
    await expect(catalogRow).toHaveCount(1);
    await expect(catalogRow).toBeVisible();
    if (index === 0) {
      await captureDocumentationScreenshot(
        page,
        "instructor_problem_catalog.png",
        page.getByRole("heading", { name: "Question catalog", exact: true }),
        72,
        inputs.screenshotDirectory,
      );
    }
    const humanReference = catalogRow.locator("code");
    await expect(humanReference).toHaveText(displayId);
    const copyId = catalogRow.getByRole("button", { name: `Copy question ID ${displayId}` });
    await tabTo(page, copyId);
    await expect(copyId).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(catalogRow.getByRole("status")).toHaveText(`Copied ${displayId}.`);
    await tabTo(page, directImport);
    await expect(directImport).toBeFocused();
    if (index > 0) {
      await page.keyboard.press("Enter");
      await expect(directImport).toHaveValue(copiedDisplayIds.slice(0, index).join("\n") + "\n");
    }
    await page.keyboard.press("ControlOrMeta+V");
    await expect(directImport).toHaveValue(copiedDisplayIds.slice(0, index + 1).join("\n"));
    if (index < copiedDisplayIds.length - 1) {
      await tabTo(page, search, "backward");
      await expect(search).toBeFocused();
    }
  }
  await expect(directImport).toHaveValue(copiedDisplayIds.join("\n"));
  const addById = page.getByRole("button", { name: "Add questions by ID", exact: true });
  await tabTo(page, addById);
  await expect(addById).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.locator(".assignment-editor-import-success")).toHaveText(
    `Added ${copiedDisplayIds.join(", ")} to the unsaved selection.`,
  );
  await expect(page.locator(".assignment-editor-list").getByRole("listitem")).toHaveCount(4);
  for (const displayId of copiedDisplayIds) {
    const selectedId = page
      .locator(".assignment-editor-list")
      .getByRole("listitem")
      .locator("code", { hasText: new RegExp(`^${displayId}$`, "u") });
    await expect(selectedId).toHaveCount(1);
    await expect(selectedId).toHaveText(displayId);
  }
  await expect(page.getByLabel("Completion requirement")).toHaveValue("allCorrect");
  await expect(page.getByLabel("Grade policy")).toHaveValue("highest");
  await expect(page.getByLabel("Continued practice")).toHaveValue("unlimited");
  writeInstructorSetupCheckpoint(inputs.instructorSetupCheckpointFile, "catalog_result_selected");
  await captureDocumentationScreenshot(
    page,
    "instructor_assignment_settings.png",
    page.locator(".assignment-editor-grid"),
    72,
    inputs.screenshotDirectory,
  );
  const createAssignment = page.getByRole("button", { name: "Create assignment", exact: true });
  await tabTo(page, createAssignment);
  await expect(createAssignment).toBeFocused();
  await page.keyboard.press("Enter");
  const assignmentLink = page.getByRole("link", { name: `Open ${assignmentTitle}`, exact: true });
  await expect(assignmentLink).toBeVisible();
  await captureDocumentationScreenshot(
    page,
    "instructor_assignment_created.png",
    page.locator(".success-state"),
    72,
    inputs.screenshotDirectory,
  );
  writeInstructorSetupCheckpoint(inputs.instructorSetupCheckpointFile, "assignment_created");
  const assignmentHref: unknown = await assignmentLink.getAttribute("href");
  if (typeof assignmentHref !== "string") {
    throw new Error("visible assignment link is unavailable");
  }
  const assignmentReference = assignmentHref.slice(assignmentHref.lastIndexOf("/") + 1);
  if (!isAssignmentReference(assignmentReference))
    throw new Error("visible assignment link is unavailable");
  const j13: Extract<InstructorSetupFragment, { readonly journey: "J13" }> = {
    schemaVersion: 2,
    journey: "J13",
    status: "PASS",
    elapsedMs: elapsedSince(startedAt),
    courseReference,
    assignmentReference,
    selectedDisplayIds: [...inputs.catalogDisplayIds] as [string, string, string, string],
    visibleOutcomeCodes: [
      "visible_assignment_created",
      "visible_catalog_problem_selected",
      "visible_four_question_chapter_one_selection",
      "visible_mastery_policy",
    ],
    diagnostics: [],
  };
  commitInstructorSetupState(inputs.journeyStateFile, [j11, j12, j13]);
});
