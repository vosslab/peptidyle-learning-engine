import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeStudentAssessmentAttemptHistory } from "../src/api/decoders/assessment_attempt_history.ts";

function history() {
  return {
    assessmentAttempt: "R-12",
    attemptNumber: 2,
    course: {
      reference: "CI7K3M2QAZ",
      shortName: "Mol Bio",
      longName: "Molecular biology",
      theme: "forest",
    },
    assessment: { reference: "A7K3M2QAS", title: "Protein folding practice" },
    state: "submitted",
    questions: [
      {
        position: 1,
        questionRevision: { questionId: "7K3M-79QP", revisionNumber: 2 },
        responseState: "submitted",
      },
    ],
  };
}

test("selected history independently accepts disclosed aggregate and per-position grades", () => {
  const decoded = decodeStudentAssessmentAttemptHistory({
    ...history(),
    score: { pointsEarned: 0, pointsPossible: 2 },
    questions: [
      {
        position: 1,
        questionRevision: { questionId: "7K3M-79QP", revisionNumber: 2 },
        responseState: "submitted",
        correctness: false,
        pointsEarned: 0,
        pointsPossible: 2,
      },
    ],
  });

  assert.equal(decoded.score.pointsEarned, 0);
  assert.equal(decoded.questions[0].correctness, false);
});

test("selected history keeps protected grade fields absent and rejects partial disclosures", () => {
  assert.deepEqual(decodeStudentAssessmentAttemptHistory(history()), history());
  assert.throws(
    () =>
      decodeStudentAssessmentAttemptHistory({
        ...history(),
        questions: [
          {
            position: 1,
            questionRevision: { questionId: "7K3M-79QP", revisionNumber: 2 },
            responseState: "submitted",
            pointsEarned: 1,
          },
        ],
      }),
    DecodeError,
  );
});

test("selected history rejects a Course context outside the established route contract", () => {
  assert.throws(
    () =>
      decodeStudentAssessmentAttemptHistory({
        ...history(),
        course: {
          reference: "CI7K3M2QAZ",
          shortName: "Mol Bio",
          longName: "Molecular biology",
          theme: "unknown",
        },
      }),
    DecodeError,
  );
});

test("selected history accepts readable recorded response blocks without grading", () => {
  const decoded = decodeStudentAssessmentAttemptHistory({
    ...history(),
    questions: [
      {
        position: 1,
        questionRevision: { questionId: "7K3M-79QP", revisionNumber: 2 },
        responseState: "submitted",
        response: [{ kind: "text", markdown: "alpha helix" }],
      },
    ],
  });

  assert.deepEqual(decoded.questions[0].response, [{ kind: "text", markdown: "alpha helix" }]);
  assert.equal(decoded.questions[0].correctness, undefined);
});

test("selected history independently accepts a correct answer without a response or grade", () => {
  const decoded = decodeStudentAssessmentAttemptHistory({
    ...history(),
    questions: [
      {
        position: 1,
        questionRevision: { questionId: "7K3M-79QP", revisionNumber: 2 },
        responseState: "closed",
        questionAnswer: [{ kind: "text", markdown: "ATP" }],
      },
    ],
  });

  assert.deepEqual(decoded.questions[0].questionAnswer, [{ kind: "text", markdown: "ATP" }]);
  assert.equal(decoded.questions[0].response, undefined);
});

test("selected history accepts recorded outcome feedback and omits unavailable explanation", () => {
  const decoded = decodeStudentAssessmentAttemptHistory({
    ...history(),
    questions: [
      {
        position: 1,
        questionRevision: { questionId: "7K3M-79QP", revisionNumber: 2 },
        responseState: "submitted",
        choiceFeedback: [{ kind: "text", markdown: "You selected it." }],
        incorrectFeedback: [{ kind: "text", markdown: "Try again." }],
      },
    ],
  });

  assert.deepEqual(decoded.questions[0].incorrectFeedback, [
    { kind: "text", markdown: "Try again." },
  ]);
  assert.equal(decoded.questions[0].questionAnswerExplanation, undefined);
});
