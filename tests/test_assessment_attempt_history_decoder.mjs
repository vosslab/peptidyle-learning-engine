import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeStudentAssessmentAttemptHistory } from "../src/api/decoders/assessment_attempt_history.ts";
import { backendAnswerReviewDocumentUrl } from "../src/api/assessment_attempt_history.ts";

test("backend answer review accepts only availability and derives its authorized route", () => {
  const original = history();
  assert.equal(
    "backendAnswerReview" in decodeStudentAssessmentAttemptHistory(original).questions[0],
    false,
  );
  const permitted = {
    ...original,
    questions: [
      { ...original.questions[0], responseState: "closed", backendAnswerReview: "available" },
    ],
  };
  const decoded = decodeStudentAssessmentAttemptHistory(permitted);
  assert.equal(decoded.questions[0].backendAnswerReview, "available");
  assert.equal(decoded.questions[0].questionAnswer, undefined);
  assert.equal(decoded.score, undefined);
  assert.equal(
    backendAnswerReviewDocumentUrl(decoded.assessmentAttempt, 1),
    "/api/assessment-attempts/00000000-0000-0000-0000-00000000000c/questions/1/answer-review-document",
  );
  for (const marker of [
    null,
    false,
    true,
    "withheld",
    "https://renderer.example",
    { url: "/answer" },
  ]) {
    assert.throws(
      () =>
        decodeStudentAssessmentAttemptHistory({
          ...original,
          questions: [{ ...original.questions[0], backendAnswerReview: marker }],
        }),
      DecodeError,
    );
  }
  for (const position of [0, -1, 1.5, Number.NaN, Number.MAX_SAFE_INTEGER + 1]) {
    assert.throws(() => backendAnswerReviewDocumentUrl(decoded.assessmentAttempt, position));
  }
  assert.throws(() =>
    backendAnswerReviewDocumentUrl("00000000-0000-0000-0000-00000000000c?reveal=1", 1),
  );
});

function history() {
  return {
    assessmentAttempt: "00000000-0000-0000-0000-00000000000c",
    attemptNumber: 2,
    course: {
      id: "CI7K3M2QAZ",
      shortName: "Mol Bio",
      longName: "Molecular biology",
      theme: "forest",
    },
    assessment: { id: "A7K3M2QAS", title: "Protein folding practice" },
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
          id: "CI7K3M2QAZ",
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
