// WP-PROF-S6 strict browser course-grade contract tests.

import assert from "node:assert/strict";
import test from "node:test";

import {
  decodeCourseGradeSchemeUpdateView,
  decodeCourseGradeSchemeView,
  decodeCourseGradebookTotalsView,
} from "../src/api/decoders/course_grade.ts";
import {
  ApiProtocolError,
  CourseGradeSchemeConflictError,
  createHttpApiClient,
} from "../src/api/http_client.ts";
import {
  canonicalizeAssignments,
  gradeSettingsErrors,
  percentToBasisPoints,
} from "../src/pages/course_grade_settings_model.ts";

const COURSE_ID = "0198e000-0000-7000-8000-000000000014";
const ASSIGNMENT_ONE = "0198e000-0000-7000-8000-000000000031";
const ASSIGNMENT_TWO = "0198e000-0000-7000-8000-000000000032";
const CATEGORY_ONE = "0198e000-0000-7000-8000-000000000041";
const CATEGORY_TWO = "0198e000-0000-7000-8000-000000000042";

const weightedView = {
  scheme: {
    mode: "weightedCategories",
    rounding: "fourDecimalPlacesHalfAwayFromZero",
    categories: [
      {
        id: CATEGORY_ONE,
        title: "Laboratory work",
        position: 0,
        weightBasisPoints: 4_000,
        dropLowest: 0,
      },
      {
        id: CATEGORY_TWO,
        title: "Examinations",
        position: 1,
        weightBasisPoints: 6_000,
        dropLowest: 0,
      },
    ],
    letterBands: [
      { label: "A", minimumBasisPoints: 9_000 },
      { label: "B", minimumBasisPoints: 8_000 },
    ],
  },
  assignments: [
    {
      assignment: ASSIGNMENT_ONE,
      title: "Enzyme kinetics lab",
      included: true,
      category: CATEGORY_ONE,
      position: 0,
    },
    {
      assignment: ASSIGNMENT_TWO,
      title: "Midterm exam",
      included: true,
      category: CATEGORY_TWO,
      position: 0,
    },
  ],
};

function noStoreJson(value, etag = undefined) {
  const headers = {
    "cache-control": "no-store",
    "content-type": "application/json; charset=utf-8",
  };
  if (etag !== undefined) headers.etag = etag;
  return new Response(JSON.stringify(value), { headers });
}

test("course-grade read and write decoders keep server titles out of strict writes", () => {
  assert.deepEqual(decodeCourseGradeSchemeView(weightedView), weightedView);
  const update = {
    scheme: weightedView.scheme,
    assignments: weightedView.assignments.map(({ title: _title, ...assignment }) => assignment),
  };
  assert.deepEqual(decodeCourseGradeSchemeUpdateView(update), update);
  assert.throws(
    () => decodeCourseGradeSchemeUpdateView(weightedView),
    /assignments\[0\]\.title.*known field/u,
  );

  const unsafeRead = structuredClone(weightedView);
  unsafeRead.assignments[0].tenant = "tenant leak";
  assert.throws(() => decodeCourseGradeSchemeView(unsafeRead), /tenant.*known field/u);
});

test("course-grade decoder rejects noncanonical mappings, weights, and private total fields", () => {
  const badWeight = structuredClone(weightedView);
  badWeight.scheme.categories[1].weightBasisPoints = 5_999;
  assert.throws(() => decodeCourseGradeSchemeView(badWeight), /totaling 10000/u);

  const badPosition = structuredClone(weightedView);
  badPosition.assignments[1].position = 1;
  assert.throws(() => decodeCourseGradeSchemeView(badPosition), /canonical positions/u);

  const safeTotals = {
    mode: "totalPoints",
    rounding: "fourDecimalPlacesHalfAwayFromZero",
    rows: [
      {
        rosterId: ".student-01",
        displayName: "Student One",
        outcome: {
          status: "available",
          score: 0.875,
          letter: "B",
          droppedAssignmentIds: [],
          totalEarned: 35,
          totalPossible: 40,
        },
      },
    ],
  };
  assert.deepEqual(decodeCourseGradebookTotalsView(safeTotals), safeTotals);
  const privateTotals = structuredClone(safeTotals);
  privateTotals.rows[0].email = "student@example.edu";
  assert.throws(() => decodeCourseGradebookTotalsView(privateTotals), /email.*known field/u);
});

test("course-grade model canonicalizes order and explains invalid weighted drafts", () => {
  const update = {
    scheme: structuredClone(weightedView.scheme),
    assignments: [
      {
        assignment: ASSIGNMENT_ONE,
        included: true,
        category: CATEGORY_ONE,
        position: 9,
      },
      {
        assignment: ASSIGNMENT_TWO,
        included: true,
        category: CATEGORY_ONE,
        position: 3,
      },
    ],
  };
  assert.deepEqual(
    canonicalizeAssignments(update).assignments.map((assignment) => assignment.position),
    [1, 0],
  );
  update.scheme.categories[0].dropLowest = 2;
  update.scheme.categories[1].weightBasisPoints = 5_000;
  update.assignments[1].category = null;
  update.assignments[1].position = null;
  const errors = gradeSettingsErrors(update).join("\n");
  assert.match(errors, /total exactly 100\.00%/u);
  assert.match(errors, /Every included assignment needs a category/u);
  assert.match(errors, /retain at least one included assignment/u);
  assert.equal(percentToBasisPoints("33.33"), 3_333);
  assert.equal(percentToBasisPoints("100.01"), undefined);
});

test("HTTP course-grade client enforces no-store, exact CAS, and audited CSV metadata", async () => {
  const requests = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(input.toString(), "https://ple.example"), init);
      requests.push(request.clone());
      if (request.method === "GET" && request.url.endsWith("/grade-scheme")) {
        return noStoreJson(weightedView, '"7"');
      }
      if (request.method === "PUT") return noStoreJson(weightedView, '"8"');
      if (request.method === "POST") {
        return new Response("record_type,aggregation_mode\r\nmetadata,weightedCategories\r\n", {
          headers: {
            "cache-control": "no-store",
            "content-type": "text/csv; charset=utf-8",
            "content-disposition": "attachment; filename=ple-course-grades.csv",
            "x-ple-course-grade-export-id": "0198e000-0000-7000-8000-000000000099",
          },
        });
      }
      return noStoreJson({
        mode: "weightedCategories",
        rounding: weightedView.scheme.rounding,
        rows: [],
      });
    },
  });

  const current = await client.getCourseGradeScheme(COURSE_ID);
  assert.equal(current.revision, '"7"');
  const update = {
    scheme: weightedView.scheme,
    assignments: weightedView.assignments.map(({ title: _title, ...assignment }) => assignment),
  };
  const saved = await client.saveCourseGradeScheme(COURSE_ID, update, current.revision);
  assert.equal(saved.revision, '"8"');
  const exported = await client.createCourseGradeExport(COURSE_ID);
  assert.deepEqual(
    { exportId: exported.exportId, filename: exported.filename, text: await exported.csv.text() },
    {
      exportId: "0198e000-0000-7000-8000-000000000099",
      filename: "ple-course-grades.csv",
      text: "record_type,aggregation_mode\r\nmetadata,weightedCategories\r\n",
    },
  );
  const save = requests.find((request) => request.method === "PUT");
  assert.equal(save.headers.get("if-match"), '"7"');
  const saveBody = JSON.parse(await save.text());
  assert.equal(Object.hasOwn(saveBody.assignments[0], "title"), false);

  const conflict = createHttpApiClient({
    fetch: () =>
      Promise.resolve(
        new Response(null, { status: 412, headers: { "cache-control": "no-store" } }),
      ),
  });
  await assert.rejects(
    conflict.saveCourseGradeScheme(COURSE_ID, update, '"7"'),
    CourseGradeSchemeConflictError,
  );
  const missingRevision = createHttpApiClient({
    fetch: () => Promise.resolve(noStoreJson(weightedView)),
  });
  await assert.rejects(missingRevision.getCourseGradeScheme(COURSE_ID), ApiProtocolError);
});
