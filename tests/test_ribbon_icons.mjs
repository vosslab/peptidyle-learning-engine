// Ribbon icon tests preserve the deliberate semantic vocabulary, not a decorative quota.

import assert from "node:assert/strict";
import test from "node:test";

import {
  RIBBON_CONTEXT_CONTROL_CATALOG,
  RIBBON_TASK_CATALOG,
  TAB_CATALOG,
} from "../src/ribbon/ribbon_catalog.ts";
import {
  RIBBON_CONTEXT_GLYPH_KEYS,
  RIBBON_CONTEXT_GLYPHS,
  RIBBON_DESTINATION_GLYPHS,
  RIBBON_GLYPH_IDS,
  RIBBON_ICON_ASSET_PATH,
  ribbonGlyphForContext,
  ribbonGlyphForDestination,
} from "../src/ribbon/ribbon_icons.ts";

const CATALOG = [...TAB_CATALOG, ...RIBBON_TASK_CATALOG];

const EXPECTED_DESTINATION_GLYPHS = {
  courses: "graduation-cap",
  questions: "book-open",
  productAssignments: "clipboard-list",
  assignments: "clipboard-list",
  studentAssignments: "clipboard-list",
  students: "users",
  gradebook: "table-list",
  teachingOperations: "gear",
  blueprintUpdates: "layer-group",
  courseSetup: "gear",
  attempt: "pen-to-square",
  instructorAccounts: "circle-user",
  supportRoster: "users",
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
  assignmentsDueSoon: "clock",
  assignmentTemplates: "copy",
  assignmentOverview: "clipboard-list",
  assignmentQuestions: "list-check",
  assignmentPolicies: "gear",
  assignmentGradingOperations: "table-list",
  assignmentStudentView: "user-graduate",
  gradeSettings: "table-list",
  appearance: "palette",
  backToAssignments: "arrow-left",
};

test("the glyph vocabulary is a closed same-origin semantic contract", () => {
  assert.equal(RIBBON_ICON_ASSET_PATH, "/assets/ribbon-icons.svg");
  assert.equal(Object.isFrozen(RIBBON_DESTINATION_GLYPHS), true);
  assert.equal(Object.isFrozen(RIBBON_CONTEXT_GLYPHS), true);
  assert.deepEqual(
    Object.keys(RIBBON_DESTINATION_GLYPHS).sort(),
    Object.keys(EXPECTED_DESTINATION_GLYPHS).sort(),
  );
  assert.deepEqual(RIBBON_DESTINATION_GLYPHS, EXPECTED_DESTINATION_GLYPHS);
  assert.deepEqual(RIBBON_CONTEXT_GLYPH_KEYS, ["profile", "signOut"]);
  assert.deepEqual(RIBBON_CONTEXT_GLYPHS, {
    profile: "circle-user",
    signOut: "right-from-bracket",
  });

  const declaredGlyphs = new Set(RIBBON_GLYPH_IDS);
  for (const glyph of [
    ...Object.values(RIBBON_DESTINATION_GLYPHS),
    ...Object.values(RIBBON_CONTEXT_GLYPHS),
  ]) {
    assert.equal(
      declaredGlyphs.has(glyph),
      true,
      `${glyph} must be a declared Font Awesome glyph id`,
    );
  }
});

test("catalog icon intent and the glyph map are exhaustive in both directions", () => {
  const mappedIds = new Set(Object.keys(RIBBON_DESTINATION_GLYPHS));
  for (const control of CATALOG) {
    const mappedGlyph = ribbonGlyphForDestination(control.id);
    assert.equal(
      control.iconBearing,
      mappedGlyph !== undefined,
      `${control.id} icon-bearing intent must exactly match the glyph map`,
    );
    assert.equal(
      mappedIds.has(control.id),
      control.iconBearing,
      `${control.id} must not have an accidental or missing glyph map entry`,
    );
    assert.equal(
      control.iconOnlySafe && !control.iconBearing,
      false,
      `${control.id} cannot be icon-only without a glyph`,
    );
  }

  for (const id of mappedIds) {
    assert.ok(
      CATALOG.some((control) => control.id === id),
      `${id} must be a declared destination`,
    );
  }
});

test("every navigation destination retains its text label", () => {
  assert.deepEqual(
    CATALOG.filter((control) => control.iconOnlySafe),
    [],
  );
});

test("context glyphs remain closed identities rather than invented navigation controls", () => {
  assert.equal(ribbonGlyphForContext("signOut"), "right-from-bracket");
  assert.equal(ribbonGlyphForContext("profile"), "circle-user");
  assert.deepEqual(RIBBON_CONTEXT_CONTROL_CATALOG, [
    {
      id: "profile",
      label: "Profile",
      productRole: "instructor",
      availability: "Available",
      glyph: "profile",
    },
  ]);
  assert.equal(
    CATALOG.some((control) => control.id === "profile" || control.id === "signOut"),
    false,
  );
});
