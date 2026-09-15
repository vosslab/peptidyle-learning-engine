import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import {
  decodeLiveStudentAssessmentLandings,
  decodeLiveStudentCourseLandings,
} from "../src/api/decoders/live_student_course_landing.ts";

test("Student Course landing carries both Course Instance names", () => {
  const value = {
    courses: [{ reference: "CI6F2R8T", shortName: "Mol Bio", longName: "Molecular Biology" }],
  };
  assert.deepEqual(decodeLiveStudentCourseLandings(value), value.courses);
});

function assessment(overrides = {}) {
  return {
    reference: "A5D9Q3X",
    title: "Peptide practice",
    decision: {
      availableAt: 1_000,
      dueAt: 2_000,
      closesAt: 3_000,
      timeLimitSeconds: 900,
      attemptLimit: 2,
      lateWorkRule: "reject",
      displayTimeZone: "America/Chicago",
      evaluatedAt: 1_500,
      startDecision: "may_start",
      publicReason: null,
    },
    assessmentAttemptNumber: 1,
    assessmentAttemptCompletion: "inProgress",
    gradedQuestionCount: 1,
    questionCount: 4,
    ...overrides,
  };
}

test("Student Course landing accepts an omitted score while disclosure withholds it", () => {
  const value = { assessments: [assessment()] };
  assert.deepEqual(decodeLiveStudentAssessmentLandings(value), value.assessments);
});

test("Student Course landing accepts one complete released score pair", () => {
  const value = {
    assessments: [
      assessment({ gradedQuestionCount: 4, score: { pointsEarned: 1, pointsPossible: 2 } }),
    ],
  };
  assert.deepEqual(decodeLiveStudentAssessmentLandings(value), value.assessments);
});

test("Student Course landing rejects a partial, nullable, or stale score projection", () => {
  for (const candidate of [
    assessment({ score: { pointsEarned: 1 } }),
    assessment({ score: null }),
    assessment({ score: { pointsEarned: 3, pointsPossible: 2 } }),
    assessment({ score: { pointsEarned: 1, pointsPossible: 2 } }),
    assessment({ pointsEarned: 1, pointsPossible: 2 }),
  ]) {
    assert.throws(
      () => decodeLiveStudentAssessmentLandings({ assessments: [candidate] }),
      DecodeError,
    );
  }
});

test("Student Course landing requires the closed server decision summary", () => {
  assert.throws(
    () =>
      decodeLiveStudentAssessmentLandings({
        assessments: [
          assessment({
            decision: {
              ...assessment().decision,
              startDecision: "closed",
              publicReason: "This Assignment is closed for new work.",
              accommodationId: "private",
            },
          }),
        ],
      }),
    DecodeError,
  );
});
