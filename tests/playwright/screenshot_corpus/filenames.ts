// filenames.ts - Screenshot filenames tied to canonical Ribbon catalog identities.

import {
  RIBBON_TASK_CATALOG,
  TAB_CATALOG,
  type RibbonTabId,
  type RibbonTaskId,
} from "../../../src/ribbon/ribbon_catalog";

function catalogSlug(identifier: string): string {
  return identifier.replace(/[A-Z]/gu, (letter) => `-${letter.toLowerCase()}`);
}

/** Short, stable filename aliases for catalog-backed Tier 2 task identities. */
export const TIER_TWO_FILENAME_ALIASES: Readonly<Record<RibbonTaskId, string>> = {
  myBlueprintCourses: "blueprint",
  myActiveCourses: "active",
  myInactiveCourses: "inactive",
  searchPublicBlueprintCourses: "search",
  myQuestions: "questions",
  myDraftQuestions: "drafts",
  starred: "starred",
  watched: "watched",
  searchQuestionLibrary: "search",
  browseQuestionLibrary: "browse",
  assessmentsDueSoon: "due",
  assessmentTemplates: "templates",
  assessments: "assessments",
  students: "students",
  gradebook: "gradebook",
  teachingOperations: "teaching",
  blueprintUpdates: "updates",
  courseSetup: "setup",
  allCoursework: "coursework",
  dueSoon: "due",
  completedCoursework: "completed",
  activeAttempt: "active",
  studentScores: "scores",
  studentResponseStats: "stats",
  studentAttemptHistory: "history",
  studentLatestFeedback: "feedback",
  assessmentOverview: "overview",
  assessmentQuestions: "questions",
  assessmentPolicies: "policies",
  assessmentStudentView: "student-view",
  gradeSettings: "grade-settings",
  appearance: "appearance",
};

/** Build a filename prefix from actual Tier 1 and Tier 2 catalog identifiers. */
export function catalogScreenshotFilename(
  tierOneId: RibbonTabId,
  tierTwoId: RibbonTaskId,
  details: string,
): string {
  const tierOne = TAB_CATALOG.find((entry) => entry.id === tierOneId);
  const tierTwo = RIBBON_TASK_CATALOG.find((entry) => entry.id === tierTwoId);
  if (tierOne === undefined || tierTwo === undefined) {
    throw new Error(
      `Screenshot filename needs catalog-backed Ribbon IDs: ${tierOneId}/${tierTwoId}`,
    );
  }
  if (!/^[a-z0-9][a-z0-9_-]*$/u.test(details)) {
    throw new Error("Screenshot filename details must be a lowercase ASCII slug.");
  }
  const tierTwoAlias = TIER_TWO_FILENAME_ALIASES[tierTwoId];
  if (!/^[a-z0-9]+(?:-[a-z0-9]+)*$/u.test(tierTwoAlias)) {
    throw new Error(`Tier 2 filename alias is not a lowercase slug: ${tierTwoAlias}`);
  }
  return `${catalogSlug(tierOne.id)}-${tierTwoAlias}-${details}`;
}
