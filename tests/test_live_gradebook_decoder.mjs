import assert from "node:assert/strict";
import test from "node:test";

import { decodeCourseGradebook } from "../src/api/decoders/live_gradebook.ts";

test("Gradebook distinguishes an expired submission still awaiting outcomes from zero credit", () => {
  const gradebook = {
    courseReference: "CI7K3M2Q",
    studentWork: [
      {
        rosterId: "student-1",
        assessmentReference: "A7K3M2Q",
        assessmentAttemptCompletion: "inProgress",
        expiredSubmitting: true,
        score: null,
      },
    ],
  };
  assert.deepEqual(decodeCourseGradebook(gradebook), gradebook);
  assert.throws(() =>
    decodeCourseGradebook({
      ...gradebook,
      studentWork: [{ ...gradebook.studentWork[0], score: { pointsEarned: 0, pointsPossible: 2 } }],
    }),
  );
});

test("Gradebook accepts finite non-negative grade contribution pairs", () => {
  const gradebook = {
    courseReference: "CI7K3M2Q",
    studentWork: [
      {
        rosterId: "bonus-student",
        assessmentReference: "A7K3M2Q",
        assessmentAttemptCompletion: "completed",
        expiredSubmitting: false,
        score: { pointsEarned: 3, pointsPossible: 0 },
      },
      {
        rosterId: "extra-credit-student",
        assessmentReference: "A7K3M2R",
        assessmentAttemptCompletion: "completed",
        expiredSubmitting: false,
        score: { pointsEarned: 4, pointsPossible: 2 },
      },
    ],
  };

  assert.deepEqual(decodeCourseGradebook(gradebook), gradebook);
  for (const score of [
    { pointsEarned: -1, pointsPossible: 0 },
    { pointsEarned: 1, pointsPossible: Number.POSITIVE_INFINITY },
    { pointsEarned: 1 },
  ]) {
    assert.throws(() =>
      decodeCourseGradebook({
        ...gradebook,
        studentWork: [{ ...gradebook.studentWork[0], score }],
      }),
    );
  }
});
