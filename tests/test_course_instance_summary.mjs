// Course-list summary and dense Instructor row boundary checks.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeCourseInstanceList } from "../src/api/decoders/course_instance.ts";
import { courseThemeTokens } from "../src/features/course_appearance/course_theme_registry.ts";

function courseSummary(theme = "forest") {
  return {
    reference: "C-7",
    title: "Molecular Biology",
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

test("dense Instructor rows remain scoped away from Student cards and product theme scope", () => {
  const courseList = readFileSync("src/pages/course_list_page.tsx", "utf8");
  const courseWorkspace = readFileSync("src/pages/course_instance_page.tsx", "utf8");
  const styles = readFileSync("src/style.css", "utf8");

  assert.match(courseList, /class="instructor-list__row instructor-list__row--course"/u);
  assert.match(courseList, /Theme: \{theme\.name\}/u);
  assert.doesNotMatch(courseList, /CourseThemeVariables/u);
  assert.match(courseWorkspace, /class="instructor-list__row instructor-list__row--assignment"/u);
  assert.match(styles, /--ple-list-row-min-block-size/u);
  assert.match(styles, /@media \(forced-colors: active\)/u);
  const instructorListStyles = styles.slice(
    styles.indexOf(".instructor-list"),
    styles.indexOf(".course-card"),
  );
  assert.doesNotMatch(instructorListStyles, /!important/u);
});
