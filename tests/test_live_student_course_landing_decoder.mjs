import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import {
  decodeLiveStudentAssignmentLandings,
  decodeLiveStudentCourseLandings,
} from "../src/api/decoders/live_student_course_landing.ts";

test("Student Course landing carries both Course Instance names", () => {
  const value = {
    courses: [{ reference: "C-7", shortName: "Mol Bio", longName: "Molecular Biology" }],
  };
  assert.deepEqual(decodeLiveStudentCourseLandings(value), value.courses);
});

function assignment(overrides = {}) {
  return {
    reference: "A-7",
    title: "Peptide practice",
    assignmentAttemptNumber: 1,
    assignmentAttemptCompletion: "inProgress",
    gradedQuestionCount: 1,
    questionCount: 4,
    ...overrides,
  };
}

test("Student Course landing accepts an omitted score while disclosure withholds it", () => {
  const value = { assignments: [assignment()] };
  assert.deepEqual(decodeLiveStudentAssignmentLandings(value), value.assignments);
});

test("Student Course landing accepts one complete released score pair", () => {
  const value = {
    assignments: [
      assignment({ gradedQuestionCount: 4, score: { pointsEarned: 1, pointsPossible: 2 } }),
    ],
  };
  assert.deepEqual(decodeLiveStudentAssignmentLandings(value), value.assignments);
});

test("Student Course landing rejects a partial, nullable, or stale score projection", () => {
  for (const candidate of [
    assignment({ score: { pointsEarned: 1 } }),
    assignment({ score: null }),
    assignment({ score: { pointsEarned: 3, pointsPossible: 2 } }),
    assignment({ score: { pointsEarned: 1, pointsPossible: 2 } }),
    assignment({ pointsEarned: 1, pointsPossible: 2 }),
  ]) {
    assert.throws(
      () => decodeLiveStudentAssignmentLandings({ assignments: [candidate] }),
      DecodeError,
    );
  }
});
