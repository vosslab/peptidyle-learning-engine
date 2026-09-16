// Course-list summary and dense Instructor row boundary checks.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import {
  decodeCourseInstanceList,
  decodeCreateCourseInstanceInput,
} from "../src/api/decoders/course_instance.ts";
import { courseThemeTokens } from "../src/features/course_appearance/course_theme_registry.ts";
import { decodeCourseClassification } from "../src/api/decoders/course_classification.ts";

const classification = {
  disciplineUuid: "018f5e7d-01b6-7c14-8a0b-4bfef6390d6d",
  subjectUuid: null,
  topicUuid: null,
  subtopicUuid: null,
  tags: [],
};

function courseSummary(theme = "forest") {
  return {
    reference: "CI6F2R8T",
    classification,
    metadataEtag: "018f5e7d-01b6-7c14-8a0b-4bfef6390d6e",
    shortName: "Mol Bio",
    longName: "Molecular Biology",
    term: {
      startDate: "2026-09-01",
      endDate: "2026-12-18",
    },
    theme,
  };
}

test("the Instructor Course list accepts one closed row-theme identity", () => {
  const [course] = decodeCourseInstanceList({ items: [courseSummary()], nextCursor: null });

  assert.equal(course.theme, "forest");
  assert.equal(courseThemeTokens(course.theme).name, "Forest");
});

test("the Course-list decoder rejects missing, surplus, and unknown theme data", () => {
  const missingTheme = courseSummary();
  delete missingTheme.theme;
  assert.throws(
    () => decodeCourseInstanceList({ items: [missingTheme], nextCursor: null }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeCourseInstanceList({
        items: [{ ...courseSummary(), theme: "unreviewed" }],
        nextCursor: null,
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeCourseInstanceList({
        items: [{ ...courseSummary(), privateAppearance: "must-not-cross" }],
        nextCursor: null,
      }),
    DecodeError,
  );
});

test("Course creation accepts only the two current source wires", () => {
  const common = {
    classification,
    shortName: "Genetics",
    longName: "Advanced Genetics",
    term: { startDate: "2026-09-01", endDate: "2026-12-18" },
  };
  assert.deepEqual(
    decodeCreateCourseInstanceInput({ ...common, source: { kind: "empty" } }).source,
    {
      kind: "empty",
    },
  );
  assert.deepEqual(
    decodeCreateCourseInstanceInput({
      ...common,
      source: { kind: "adopted", blueprintCourse: "BP6F2R8T", blueprintRevision: "2" },
    }).source,
    { kind: "adopted", blueprintCourse: "BP6F2R8T", blueprintRevision: "2" },
  );
  assert.throws(
    () =>
      decodeCreateCourseInstanceInput({
        ...common,
        source: { kind: "empty", blueprintCourse: "BP6F2R8T" },
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeCreateCourseInstanceInput({
        ...common,
        source: { kind: "adopted", blueprint_course: "BP6F2R8T", blueprint_revision: 2 },
      }),
    DecodeError,
  );
  assert.throws(
    () => decodeCreateCourseInstanceInput({ ...common, blueprintCourse: "BP6F2R8T" }),
    DecodeError,
  );
});

test("Course classification requires a Discipline, permits absent Subject, and has no Tag-count ceiling", () => {
  assert.deepEqual(decodeCourseClassification(classification), classification);
  assert.throws(
    () => decodeCourseClassification({ ...classification, disciplineUuid: null }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeCourseClassification({ ...classification, topicUuid: classification.disciplineUuid }),
    DecodeError,
  );
  assert.throws(
    () => decodeCourseClassification({ ...classification, tags: ["duplicate", "duplicate"] }),
    DecodeError,
  );
  const tags = Array.from({ length: 70 }, (_, index) => `tag-${index}`);
  assert.deepEqual(decodeCourseClassification({ ...classification, tags }).tags, tags);
});

test("dense Instructor rows remain scoped away from Student cards and product theme scope", () => {
  const courseList = readFileSync("src/pages/course_list_page.tsx", "utf8");
  const courseWorkspace = readFileSync("src/pages/course_instance_page.tsx", "utf8");
  const styles = readFileSync("src/style.css", "utf8");

  assert.match(courseList, /class="instructor-list__row instructor-list__row--course"/u);
  assert.match(courseList, /Theme: \{theme\.name\}/u);
  assert.doesNotMatch(courseList, /CourseThemeVariables/u);
  assert.match(courseWorkspace, /class="instructor-list__row instructor-list__row--assessment"/u);
  assert.match(styles, /--ple-list-row-min-block-size/u);
  assert.match(styles, /@media \(forced-colors: active\)/u);
  const instructorListStyles = styles.slice(
    styles.indexOf(".instructor-list"),
    styles.indexOf(".course-card"),
  );
  assert.doesNotMatch(instructorListStyles, /!important/u);
});
