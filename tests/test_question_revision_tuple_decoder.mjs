import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodePublishedQuestionRevisionTuple } from "../src/api/decoders/shared.ts";
import { blueprintRevisionTuple } from "../src/api/decoders/blueprint_course.ts";

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

test("Blueprint Revision Tuple decodes blueprintCourseId plus revisionNumber", () => {
  assert.deepEqual(
    blueprintRevisionTuple(BLUEPRINT_TUPLE, "blueprintRevisionTuple"),
    BLUEPRINT_TUPLE,
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

test("Blueprint Revision Tuple rejects leftover reference and snake_case members", () => {
  assert.throws(
    () => blueprintRevisionTuple({ reference: BLUEPRINT_TUPLE }, "blueprintRevisionTuple"),
    DecodeError,
  );
  assert.throws(
    () =>
      blueprintRevisionTuple(
        { blueprint_course_id: "BP7K3M2QXH", revision: "1" },
        "blueprintRevisionTuple",
      ),
    DecodeError,
  );
  assert.throws(() => blueprintRevisionTuple("1", "blueprintRevisionTuple"), DecodeError);
});
