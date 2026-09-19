import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeQuestionRevisionTuple } from "../src/api/decoders/shared.ts";
import { blueprintRevisionTuple } from "../src/api/decoders/blueprint_course.ts";

const TUPLE = { questionId: "ABCD-XEFG", revisionNumber: 1 };
const BLUEPRINT_TUPLE = { blueprintCourseId: "BP7K3M2QXH", revisionNumber: "1" };

test("Question Revision Tuple decodes questionId plus revisionNumber", () => {
  assert.deepEqual(decodeQuestionRevisionTuple(TUPLE, "questionRevisionTuple"), TUPLE);
});

test("Question Revision Tuple rejects leftover reference JSON", () => {
  assert.throws(
    () => decodeQuestionRevisionTuple({ reference: TUPLE }, "questionRevisionTuple"),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeQuestionRevisionTuple({ ...TUPLE, reference: "ABCD-XEFG" }, "questionRevisionTuple"),
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
        published_question: { questionId: "AAAA-2BBB", revisionNumber: 1 },
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
});
