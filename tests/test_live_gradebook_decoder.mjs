import assert from "node:assert/strict";
import test from "node:test";

import { decodeCourseGradebook } from "../src/api/decoders/live_gradebook.ts";

test("Gradebook distinguishes an expired submission still awaiting outcomes from zero credit", () => {
  const gradebook = {
    courseReference: "C-1",
    studentWork: [
      {
        rosterId: "student-1",
        assignmentReference: "A-1",
        assignmentAttemptCompletion: "inProgress",
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
