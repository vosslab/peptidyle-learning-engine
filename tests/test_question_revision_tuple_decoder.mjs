import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodePublishedQuestionRevisionTuple } from "../src/api/decoders/shared.ts";
import {
  blueprintCourseRevisionTuple,
  decodeBlueprintRevisionView,
} from "../src/api/decoders/blueprint_course.ts";

const TUPLE = { publishedQuestionId: "ABCD-XEFG", revisionNumber: 1 };
const BLUEPRINT_TUPLE = { blueprintCourseId: "BP7K3M2QXH", revisionNumber: "1" };

test("Published Question Revision Tuple decodes publishedQuestionId plus revisionNumber", () => {
  assert.deepEqual(
    decodePublishedQuestionRevisionTuple(TUPLE, "publishedQuestionRevisionTuple"),
    TUPLE,
  );
});

test("Question Revision Tuple rejects leftover reference JSON", () => {
  assert.throws(
    () =>
      decodePublishedQuestionRevisionTuple({ reference: TUPLE }, "publishedQuestionRevisionTuple"),
    DecodeError,
  );
  assert.throws(
    () =>
      decodePublishedQuestionRevisionTuple(
        { ...TUPLE, reference: "ABCD-XEFG" },
        "publishedQuestionRevisionTuple",
      ),
    DecodeError,
  );
  assert.throws(
    () => decodePublishedQuestionRevisionTuple(1, "publishedQuestionRevisionTuple"),
    DecodeError,
  );
});

test("Blueprint Course Revision Tuple decodes blueprintCourseId plus revisionNumber", () => {
  assert.deepEqual(
    blueprintCourseRevisionTuple(BLUEPRINT_TUPLE, "blueprintCourseRevisionTuple"),
    BLUEPRINT_TUPLE,
  );
  const view = { blueprintCourseRevisionTuple: BLUEPRINT_TUPLE, modules: [] };
  assert.deepEqual(decodeBlueprintRevisionView(view), view);
  assert.throws(
    () => decodeBlueprintRevisionView({ blueprintRevisionTuple: BLUEPRINT_TUPLE, modules: [] }),
    DecodeError,
  );
  assert.throws(
    () => decodeBlueprintRevisionView({ ...view, blueprintRevisionTuple: BLUEPRINT_TUPLE }),
    DecodeError,
  );
});

test("Blueprint Assessment fixed entry rejects leftover published_question Tuple JSON", async () => {
  const { decodeCreateBlueprintCourseInput } =
    await import("../src/api/decoders/blueprint_course.ts");
  const classification = {
    disciplineUuid: "018f5e7d-01b6-7c14-8a0b-4bfef6390d6d",
    subjectUuid: null,
    topicUuid: null,
    subtopicUuid: null,
    tags: [],
  };
  const content = {
    title: "Ready exam",
    instructions: "Answer.",
    entries: [
      {
        kind: "fixed",
        published_question: { publishedQuestionId: "AAAA-2BBB", revisionNumber: 1 },
        points_possible: "1",
        scoring_rule: "normal",
        question_attempt_limit: { maxAttempts: null },
        question_attempt_time_limit: { kind: "unlimited" },
      },
    ],
    defaults: {},
  };
  assert.throws(
    () =>
      decodeCreateBlueprintCourseInput({
        classification,
        short_name: "Local Blueprint",
        long_name: "Local Blueprint Course",
        modules: [{ label: "Module 1", assessments: [content] }],
      }),
    DecodeError,
  );
});

test("Blueprint Course Revision Tuple rejects leftover reference and snake_case members", () => {
  assert.throws(
    () =>
      blueprintCourseRevisionTuple({ reference: BLUEPRINT_TUPLE }, "blueprintCourseRevisionTuple"),
    DecodeError,
  );
  assert.throws(
    () =>
      blueprintCourseRevisionTuple(
        { blueprint_course_id: "BP7K3M2QXH", revision: "1" },
        "blueprintCourseRevisionTuple",
      ),
    DecodeError,
  );
  assert.throws(
    () => blueprintCourseRevisionTuple("1", "blueprintCourseRevisionTuple"),
    DecodeError,
  );
});
