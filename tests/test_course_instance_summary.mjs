// Course-list summary and dense Instructor row boundary checks.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import {
  decodeCreateBlueprintFromCourseInstanceInput,
  decodeCourseInstanceList,
  decodeCreateCourseInstanceInput,
} from "../src/api/decoders/course_instance.ts";
import { ApiProtocolError, createHttpApiClient } from "../src/api/http_client.ts";
import { courseThemeTokens } from "../src/features/course_appearance/course_theme_registry.ts";
import { decodeCourseClassification } from "../src/api/decoders/course_classification.ts";

const classification = {
  disciplineUuid: "018f5e7d-01b6-7c14-8a0b-4bfef6390d6d",
  subjectUuid: null,
  topicUuid: null,
  subtopicUuid: null,
  tags: [],
};

function courseSummary(lifecycleState = "active", theme = "forest") {
  return {
    id: "CI6F2R8TA0",
    classification,
    lifecycleState,
    courseEditNumber: "1",
    shortName: "Mol Bio",
    longName: "Molecular Biology",
    term: {
      startDate: "2026-09-01",
      endDate: "2026-12-18",
    },
    theme,
  };
}

test("the Instructor Course list accepts both closed activity states and one closed row-theme identity", () => {
  const [course] = decodeCourseInstanceList({ items: [courseSummary()], nextCursor: null });

  assert.equal(course.lifecycleState, "active");
  assert.equal(course.theme, "forest");
  assert.equal(courseThemeTokens(course.theme).name, "Forest");
  const [inactiveCourse] = decodeCourseInstanceList({
    items: [courseSummary("inactive")],
    nextCursor: null,
  });
  assert.equal(inactiveCourse.lifecycleState, "inactive");
});

test("the Course-list decoder rejects missing, surplus, and unknown closed data", () => {
  const missingLifecycleState = courseSummary();
  delete missingLifecycleState.lifecycleState;
  assert.throws(
    () => decodeCourseInstanceList({ items: [missingLifecycleState], nextCursor: null }),
    DecodeError,
  );
  const missingTheme = courseSummary();
  delete missingTheme.theme;
  assert.throws(
    () => decodeCourseInstanceList({ items: [missingTheme], nextCursor: null }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeCourseInstanceList({
        items: [{ ...courseSummary(), lifecycleState: "archived" }],
        nextCursor: null,
      }),
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
      source: { kind: "adopted", blueprintCourse: "BP6F2R8TA9", blueprintRevision: "2" },
    }).source,
    { kind: "adopted", blueprintCourse: "BP6F2R8TA9", blueprintRevision: "2" },
  );
  assert.throws(
    () =>
      decodeCreateCourseInstanceInput({
        ...common,
        source: { kind: "empty", blueprintCourse: "BP6F2R8TA9" },
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeCreateCourseInstanceInput({
        ...common,
        source: { kind: "adopted", blueprint_course: "BP6F2R8TA9", blueprint_revision: 2 },
      }),
    DecodeError,
  );
  assert.throws(
    () => decodeCreateCourseInstanceInput({ ...common, blueprintCourse: "BP6F2R8TA9" }),
    DecodeError,
  );
});

test("Course-derived Blueprint creation sends only metadata and requires a new private root", async () => {
  const requests = [];
  let blueprintRevisionTuple = "BP7K3M2QAF";
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(input.toString(), "https://ple.example"), init);
      requests.push(request.clone());
      return new Response(
        JSON.stringify({
          classification,
          id: "BP7K3M2QAF",
          short_name: "Mol Bio",
          long_name: "Molecular Biology",
          availability: "private",
          blueprint_edit_number: "1",
          current_revision_tuple: {
            blueprintCourseId: blueprintRevisionTuple,
            revisionNumber: "1",
          },
          read_access: "blueprint_course_owner",
          fork_source_tuple: null,
          modules: [],
        }),
        {
          status: 201,
          headers: {
            "cache-control": "no-store",
            "content-type": "application/json",
            etag: '"1"',
          },
        },
      );
    },
  });
  const input = { classification, shortName: "Mol Bio", longName: "Molecular Biology" };
  const created = await client.createBlueprintFromCourseInstance("CI6F2R8TA0", input, "create-7");
  assert.equal(created.blueprintCourse.id, "BP7K3M2QAF");
  assert.equal(created.revisionEtag, '"1"');
  assert.equal(requests.length, 1);
  const [request] = requests;
  assert.equal(new URL(request.url).pathname, "/api/course-instances/CI6F2R8TA0/course-blueprints");
  assert.equal(request.method, "POST");
  assert.equal(request.headers.get("idempotency-key"), "create-7");
  assert.deepEqual(await request.json(), input);
  assert.throws(
    () => decodeCreateBlueprintFromCourseInstanceInput({ ...input, modules: [] }),
    DecodeError,
  );
  blueprintRevisionTuple = "BP6F2R8TA9";
  await assert.rejects(
    client.createBlueprintFromCourseInstance("CI6F2R8TA0", input, "create-8"),
    ApiProtocolError,
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
  const styles =
    readFileSync("src/style.css", "utf8") + readFileSync("src/style_responsive.css", "utf8");

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
