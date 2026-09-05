// Ribbon icon tests preserve the deliberate semantic vocabulary, not a decorative quota.

import assert from "node:assert/strict";
import test from "node:test";

import { RIBBON_TASK_CATALOG, TAB_CATALOG } from "../src/ribbon/ribbon_catalog.ts";
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
  questionLibrary: "book-open",
  assignments: "clipboard-list",
  students: "users",
  gradebook: "table-list",
  courseSetup: "gear",
  attempt: "pen-to-square",
  myQuestionDrafts: "file-pen",
  starred: "star",
  watched: "eye",
  assignmentQuestions: "list-check",
  assignmentStudentView: "user-graduate",
  appearance: "palette",
  backToAssignments: "arrow-left",
};

const EXPECTED_TEXT_ONLY_DESTINATIONS = [
  "blueprintCourses",
  "teachingOperations",
  "blueprintUpdates",
  "instructorAccounts",
  "allQuestions",
  "myQuestions",
  "assignmentOverview",
  "assignmentPolicies",
  "assignmentGradingOperations",
  "gradeSettings",
];

test("the glyph vocabulary is a closed same-origin semantic contract", () => {
  assert.equal(RIBBON_ICON_ASSET_PATH, "/assets/ribbon-icons.svg");
  assert.equal(Object.isFrozen(RIBBON_DESTINATION_GLYPHS), true);
  assert.equal(Object.isFrozen(RIBBON_CONTEXT_GLYPHS), true);
  assert.deepEqual(
    Object.keys(RIBBON_DESTINATION_GLYPHS).sort(),
    Object.keys(EXPECTED_DESTINATION_GLYPHS).sort(),
  );
  assert.deepEqual(RIBBON_DESTINATION_GLYPHS, EXPECTED_DESTINATION_GLYPHS);
  assert.deepEqual(RIBBON_CONTEXT_GLYPH_KEYS, ["account", "signOut"]);
  assert.deepEqual(RIBBON_CONTEXT_GLYPHS, {
    account: "circle-user",
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

test("only conventional narrow-phone destinations may drop their labels", () => {
  assert.deepEqual(
    CATALOG.filter((control) => control.iconOnlySafe).map((control) => control.id),
    ["starred", "watched", "backToAssignments"],
  );
});

test("ambiguous teaching destinations remain deliberately text-only", () => {
  assert.deepEqual(
    CATALOG.filter((control) => !control.iconBearing).map((control) => control.id),
    EXPECTED_TEXT_ONLY_DESTINATIONS,
  );
  for (const id of EXPECTED_TEXT_ONLY_DESTINATIONS) {
    assert.equal(ribbonGlyphForDestination(id), undefined, `${id} must remain text-only`);
  }
});

test("context glyphs remain closed identities rather than invented navigation controls", () => {
  assert.equal(ribbonGlyphForContext("account"), "circle-user");
  assert.equal(ribbonGlyphForContext("signOut"), "right-from-bracket");
  assert.equal(
    CATALOG.some((control) => control.id === "account" || control.id === "signOut"),
    false,
  );
});
