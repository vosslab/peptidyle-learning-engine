// ribbon_icons.ts - closed semantic glyph vocabulary for the bundled Ribbon sprite.

import type { RibbonDestinationId } from "./ribbon_catalog";

/** The bundled, same-origin sprite is the only asset a Ribbon glyph may request. */
export const RIBBON_ICON_ASSET_PATH = "/assets/ribbon-icons.svg" as const;

/**
 * Stable Font Awesome Free solid icon names. These are semantic identifiers,
 * not a runtime dependency on the Font Awesome package.
 */
export const RIBBON_GLYPH_IDS = [
  "graduation-cap",
  "book-open",
  "clipboard-list",
  "users",
  "table-list",
  "gear",
  "pen-to-square",
  "file-pen",
  "star",
  "eye",
  "list-check",
  "user-graduate",
  "palette",
  "arrow-left",
  "arrows-spin",
  "circle-user",
  "file-signature",
  "right-from-bracket",
  "box-archive",
  "clock",
  "copy",
  "circle-question",
  "file-circle-question",
  "layer-group",
  "magnifying-glass",
] as const;

export type RibbonGlyphId = (typeof RIBBON_GLYPH_IDS)[number];

/** Assessment Type glyphs available in the bundled Free Solid sprite. */
export const RIBBON_ASSESSMENT_TYPE_GLYPHS = [
  "pen-to-square",
  "arrows-spin",
  "star",
  "circle-question",
  "file-signature",
] as const satisfies ReadonlyArray<RibbonGlyphId>;

/**
 * The sole destination-to-glyph authority. Every visible Ribbon destination
 * pairs one subsetted glyph with its text label.
 */
export const RIBBON_DESTINATION_GLYPHS = Object.freeze({
  courses: "graduation-cap",
  questions: "book-open",
  productAssessments: "clipboard-list",
  coursework: "clipboard-list",
  grades: "table-list",
  assessments: "clipboard-list",
  students: "users",
  gradebook: "table-list",
  teachingOperations: "gear",
  blueprintUpdates: "layer-group",
  courseSetup: "gear",
  instructorAccounts: "circle-user",
  disciplines: "book-open",
  myBlueprintCourses: "layer-group",
  myActiveCourses: "graduation-cap",
  myInactiveCourses: "box-archive",
  searchPublicBlueprintCourses: "magnifying-glass",
  myQuestions: "file-circle-question",
  myDraftQuestions: "file-pen",
  starred: "star",
  watched: "eye",
  searchQuestionLibrary: "magnifying-glass",
  browseQuestionLibrary: "book-open",
  assessmentsDueSoon: "clock",
  assessmentTemplates: "copy",
  assessmentOverview: "clipboard-list",
  assessmentQuestions: "list-check",
  assessmentPolicies: "gear",
  assessmentStudentView: "user-graduate",
  gradeSettings: "table-list",
  appearance: "palette",
  studentProgress: "table-list",
  studentPracticeStats: "book-open",
  allCoursework: "clipboard-list",
  dueSoon: "clock",
  completedCoursework: "list-check",
  studentScores: "table-list",
  studentAttemptHistory: "clock",
} as const satisfies Readonly<Partial<Record<RibbonDestinationId, RibbonGlyphId>>>);

export type RibbonDestinationGlyphId = keyof typeof RIBBON_DESTINATION_GLYPHS;

/** Context identities are not destinations and therefore remain a separate closed vocabulary. */
export const RIBBON_CONTEXT_GLYPH_KEYS = ["profile", "signOut"] as const;
export type RibbonContextGlyphKey = (typeof RIBBON_CONTEXT_GLYPH_KEYS)[number];

export const RIBBON_CONTEXT_GLYPHS = Object.freeze({
  profile: "circle-user",
  signOut: "right-from-bracket",
} as const satisfies Readonly<Record<RibbonContextGlyphKey, RibbonGlyphId>>);

/** Returns the paired destination glyph, if this destination has earned one. */
export function ribbonGlyphForDestination(id: RibbonDestinationId): RibbonGlyphId | undefined {
  return RIBBON_DESTINATION_GLYPHS[id];
}

/** Returns a conventional Context glyph without fabricating a navigation destination. */
export function ribbonGlyphForContext(key: RibbonContextGlyphKey): RibbonGlyphId {
  return RIBBON_CONTEXT_GLYPHS[key];
}
