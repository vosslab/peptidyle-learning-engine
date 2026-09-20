import assert from "node:assert/strict";
import test from "node:test";

import { decodeCourseGradebook } from "../src/api/decoders/live_gradebook.ts";

test("Gradebook distinguishes an expired submission still awaiting outcomes from zero credit", () => {
  const gradebook = {
    courseInstanceId: "CI7K3M2QAZ",
    studentWork: [
      {
        rosterId: "student-1",
        rosterName: "Synthetic Student",
        assessmentId: "A7K3M2QAS",
        assessmentTitle: "Protein structure practice",
        assessmentAttemptCompletion: "inProgress",
        expiredSubmitting: true,
        score: null,
      },
    ],
  };
  assert.deepEqual(decodeCourseGradebook(gradebook), gradebook);
  for (const rosterName of [undefined, null, "", "   ", "x".repeat(201), "Invalid\nName"]) {
    assert.throws(() =>
      decodeCourseGradebook({
        ...gradebook,
        studentWork: [{ ...gradebook.studentWork[0], rosterName }],
      }),
    );
  }
  const { assessmentTitle: _title, ...missingTitle } = gradebook.studentWork[0];
  assert.throws(() => decodeCourseGradebook({ ...gradebook, studentWork: [missingTitle] }));
  // The authorized title is required; malformed or expanded projections fail closed.
  for (const assessmentTitle of [undefined, "", "   ", "x".repeat(201), 1]) {
    assert.throws(() =>
      decodeCourseGradebook({
        ...gradebook,
        studentWork: [{ ...gradebook.studentWork[0], assessmentTitle }],
      }),
    );
  }
  assert.throws(() =>
    decodeCourseGradebook({
      ...gradebook,
      studentWork: [{ ...gradebook.studentWork[0], answerKey: "not authorized" }],
    }),
  );
  assert.throws(() => decodeCourseGradebook({ ...gradebook, courseId: "CI7K3M2QAZ" }));
  assert.throws(() =>
    decodeCourseGradebook({
      ...gradebook,
      studentWork: [{ ...gradebook.studentWork[0], score: { pointsEarned: 0, pointsPossible: 2 } }],
    }),
  );
});

test("Gradebook accepts finite non-negative grade contribution pairs", () => {
  const gradebook = {
    courseInstanceId: "CI7K3M2QAZ",
    studentWork: [
      {
        rosterId: "bonus-student",
        rosterName: "Synthetic Student",
        assessmentId: "A7K3M2QAS",
        assessmentTitle: "Protein structure bonus",
        assessmentAttemptCompletion: "completed",
        expiredSubmitting: false,
        score: { pointsEarned: 3, pointsPossible: 0 },
      },
      {
        rosterId: "extra-credit-student",
        rosterName: "Synthetic Student",
        assessmentId: "A7K3M2RAC",
        assessmentTitle: "Protein structure assignment",
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
