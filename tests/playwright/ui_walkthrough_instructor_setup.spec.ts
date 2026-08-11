// Selector contract: visible labels in course_list_page.tsx, course_roster_page.tsx, and assignment_editor_page.tsx.

import { expect, test } from "@playwright/test";

import { configuredInstructorSetupInputs } from "../../playwright.config";

import {
  commitInstructorSetupState,
  type InstructorSetupFragment,
} from "./simulator/instructor_setup_state";
import { writeInstructorSetupCheckpoint } from "./simulator/instructor_setup_checkpoint";
import { tabTo } from "./simulator/keyboard_walkthrough";
import {
  instructorCredentialFromValidatedFile,
  learnerAliasFromValidatedFile,
} from "./ui_walkthrough_live_config";
import { exactCatalogResult } from "./simulator/instructor_catalog_binding";
import { documentationScreenshotsEnabled } from "./docs_screenshot_capture";

test.describe.configure({ mode: "serial" });
test.setTimeout(120_000);

test.skip(
  configuredInstructorSetupInputs === undefined,
  "requires the explicit instructor-only UI walkthrough invocation",
);

function elapsedSince(startedAt: number): number {
  return Math.round(performance.now() - startedAt);
}

function publicIdsFromAttributes(
  problemId: unknown,
  versionId: unknown,
): readonly [string, string] {
  if (
    typeof problemId !== "string" ||
    typeof versionId !== "string" ||
    !/^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/u.test(problemId) ||
    !/^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/u.test(versionId)
  ) {
    throw new Error("visible catalog reference is unavailable");
  }
  return [problemId, versionId];
}

test("J11/J12/J13 instructor visibly prepares a local Mastery assignment and hands off public IDs", async ({
  page,
}) => {
  const inputs = configuredInstructorSetupInputs;
  if (inputs === undefined)
    throw new Error("the declaration-time instructor setup skip did not apply");
  const startedAt = performance.now();
  const uniqueSuffix = `${inputs.masterSeedText}-${Date.now().toString(36)}`;
  const courseTitle = documentationScreenshotsEnabled()
    ? `Peptide bond mastery pilot ${uniqueSuffix}`
    : `Fall pilot instructor walkthrough ${uniqueSuffix}`;
  const assignmentTitle = documentationScreenshotsEnabled()
    ? "Peptide bond mastery"
    : `Mastery peptide practice ${uniqueSuffix}`;

  await page.goto("/");
  const credentialInput = page.getByLabel("Local development credential");
  await expect(credentialInput).toBeVisible();
  writeInstructorSetupCheckpoint(inputs.instructorSetupCheckpointFile, "login_visible");
  await tabTo(page, credentialInput);
  await expect(credentialInput).toBeFocused();
  await credentialInput.fill(instructorCredentialFromValidatedFile(inputs.credentialFile));
  const signIn = page.getByRole("button", { name: "Sign in locally", exact: true });
  await tabTo(page, signIn);
  await expect(signIn).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("heading", { name: "Pick up where you left off" })).toBeVisible();
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
  await expect(page.getByRole("heading", { name: courseTitle, exact: true })).toBeVisible();
  writeInstructorSetupCheckpoint(inputs.instructorSetupCheckpointFile, "course_opened");
  const courseId = new URL(page.url()).pathname.split("/")[2];
  if (courseId === undefined || !/^[0-9a-f-]{36}$/iu.test(courseId))
    throw new Error("visible course link is unavailable");
  const j11: InstructorSetupFragment = {
    schemaVersion: 2,
    journey: "J11",
    status: "PASS",
    elapsedMs: elapsedSince(startedAt),
    courseId,
    visibleOutcomeCodes: ["visible_course_created", "visible_course_opened"],
    diagnostics: [],
  };
  const students = page.getByRole("link", { name: "Students", exact: true });
  await expect(students).toHaveCount(1);
  await tabTo(page, students);
  await expect(students).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("heading", { name: "Students", exact: true })).toBeVisible();
  const learnerAlias = page.getByLabel("Configured learner alias");
  await tabTo(page, learnerAlias);
  await expect(learnerAlias).toBeFocused();
  await learnerAlias.fill(learnerAliasFromValidatedFile(inputs.learnerAliasFile));
  const addStudent = page.getByRole("button", { name: "Add active student", exact: true });
  await tabTo(page, addStudent);
  await expect(addStudent).toBeFocused();
  await page.keyboard.press("Enter");
  const activeRow = page.locator("tr", { hasText: "Local pilot" }).filter({ hasText: "active" });
  await expect(activeRow).toHaveCount(1);
  await expect(activeRow).toBeFocused();
  writeInstructorSetupCheckpoint(inputs.instructorSetupCheckpointFile, "student_active");
  const j12: InstructorSetupFragment = {
    schemaVersion: 2,
    journey: "J12",
    status: "PASS",
    elapsedMs: elapsedSince(startedAt),
    courseId,
    visibleOutcomeCodes: ["visible_local_student_active"],
    diagnostics: [],
  };
  const backToCourse = page.getByRole("link", { name: "Back to course", exact: true });
  await expect(backToCourse).toHaveCount(1);
  await tabTo(page, backToCourse);
  await expect(backToCourse).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("heading", { name: courseTitle, exact: true })).toBeVisible();
  const newAssignment = page.getByRole("link", { name: "New assignment", exact: true });
  await expect(newAssignment).toHaveCount(1);
  await tabTo(page, newAssignment);
  await expect(newAssignment).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("heading", { name: "Create assignment", exact: true })).toBeVisible();
  writeInstructorSetupCheckpoint(inputs.instructorSetupCheckpointFile, "assignment_editor_opened");
  const assignmentTitleInput = page.getByLabel("Assignment title");
  await expect(assignmentTitleInput).toBeFocused();
  await assignmentTitleInput.fill(assignmentTitle);
  const search = page.getByLabel("Search published problems");
  await tabTo(page, search);
  await expect(search).toBeFocused();
  await search.fill(inputs.catalogSearchTitle);
  const searchCatalog = page.getByRole("button", { name: "Search catalog", exact: true });
  await tabTo(page, searchCatalog);
  await expect(searchCatalog).toBeFocused();
  await page.keyboard.press("Enter");
  const catalogRow = await exactCatalogResult(page, inputs.catalogSearchTitle);
  const catalogReference = catalogRow.locator("p[data-problem-id][data-version-id]");
  await expect(catalogReference).toHaveCount(1);
  const [problemId, versionId] = publicIdsFromAttributes(
    await catalogReference.getAttribute("data-problem-id"),
    await catalogReference.getAttribute("data-version-id"),
  );
  const addVersion = catalogRow.getByRole("button", { name: "Add published version", exact: true });
  await tabTo(page, addVersion);
  await expect(addVersion).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.getByLabel("Completion requirement")).toHaveValue("allCorrect");
  await expect(page.getByLabel("Grade policy")).toHaveValue("highest");
  await expect(page.getByLabel("Continued practice")).toHaveValue("unlimited");
  writeInstructorSetupCheckpoint(inputs.instructorSetupCheckpointFile, "catalog_result_selected");
  const createAssignment = page.getByRole("button", { name: "Create assignment", exact: true });
  await tabTo(page, createAssignment);
  await expect(createAssignment).toBeFocused();
  await page.keyboard.press("Enter");
  const assignmentLink = page.getByRole("link", { name: `Open ${assignmentTitle}`, exact: true });
  await expect(assignmentLink).toBeVisible();
  writeInstructorSetupCheckpoint(inputs.instructorSetupCheckpointFile, "assignment_created");
  const assignmentHref: unknown = await assignmentLink.getAttribute("href");
  if (typeof assignmentHref !== "string") {
    throw new Error("visible assignment link is unavailable");
  }
  const assignmentId = assignmentHref.slice(assignmentHref.lastIndexOf("/") + 1);
  if (!/^[0-9a-f-]{36}$/iu.test(assignmentId))
    throw new Error("visible assignment link is unavailable");
  const j13: InstructorSetupFragment = {
    schemaVersion: 2,
    journey: "J13",
    status: "PASS",
    elapsedMs: elapsedSince(startedAt),
    courseId,
    assignmentId,
    problemId,
    versionId,
    visibleOutcomeCodes: [
      "visible_assignment_created",
      "visible_catalog_problem_selected",
      "visible_mastery_policy",
    ],
    diagnostics: [],
  };
  commitInstructorSetupState(inputs.journeyStateFile, [j11, j12, j13]);
});
