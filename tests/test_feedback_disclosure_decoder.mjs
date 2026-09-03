// Public feedback must stay an exact policy-redacted contract at the browser boundary.

import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import {
  decodeStudentFeedback,
  decodeStudentQuestionAttempt,
  decodeStudentIssuedQuestion,
  decodeAssignmentAttemptSummaryResponse,
  decodeGradedQuestionSubmissionReceipt,
} from "../src/api/decoders.ts";
import { publishedQuestionFixture } from "./fixtures/published_question.ts";

const studentProgress = {
  assignment_progress: {
    completed_assignment_attempt_count: 2,
    total_question_attempts: 4,
    last_activity_at: 1786000000000,
  },
  student_assignment_grade: {
    score_state: "available",
    assignment_scoring_state: "current",
    current_score: 0,
    best_score: 1,
    latest_score: 0,
  },
};

test("disclosed feedback preserves allowed accessible blocks and optional omission", () => {
  const feedback = {
    correctness: false,
    choiceFeedback: [{ kind: "text", markdown: "This choice has no chromosome pair." }],
    incorrectFeedback: [{ kind: "text", markdown: "Review mitosis vocabulary." }],
  };
  assert.deepEqual(decodeStudentFeedback(feedback), feedback);
  assert.deepEqual(decodeStudentFeedback({}), {});
});

test("Student attempts require score freshness and redact stale numeric results", () => {
  const attempt = structuredClone(publishedQuestionFixture.attempts[0]);
  const current = { ...attempt, assignmentScoringState: "current" };
  assert.deepEqual(decodeStudentQuestionAttempt(current), current);

  for (const assignmentScoringState of ["recalculating", "failed"]) {
    const redacted = {
      ...attempt,
      submission: { ...attempt.submission, gradingResult: null },
      assignmentScoringState,
    };
    assert.deepEqual(decodeStudentQuestionAttempt(redacted), redacted);
    assert.throws(
      () =>
        decodeStudentQuestionAttempt({
          ...redacted,
          submission: { ...redacted.submission, gradingResult: attempt.submission.gradingResult },
        }),
      DecodeError,
      `${assignmentScoringState} must reject a numeric result`,
    );
  }

  const { assignmentScoringState: _assignmentScoringState, ...missingState } = current;
  assert.throws(() => decodeStudentQuestionAttempt(missingState), DecodeError);
  assert.throws(
    () => decodeStudentQuestionAttempt({ ...attempt, assignmentScoringState: "stale" }),
    DecodeError,
  );
});

test("attempt decoder accepts only the closed Question Attempt state vocabulary", () => {
  const attempt = structuredClone(publishedQuestionFixture.attempts[0]);
  const deadlineClosed = {
    ...structuredClone(publishedQuestionFixture.attempts.at(-1)),
    state: "closed_at_deadline",
    assignmentScoringState: "current",
    questionPoolSelectionPosition: null,
  };
  assert.equal(decodeStudentQuestionAttempt(deadlineClosed).state, "closed_at_deadline");
  for (const nonCanonicalState of [
    "unexpected_question_attempt_state",
    "deadline_submission_state",
  ]) {
    assert.throws(
      () =>
        decodeStudentQuestionAttempt({
          ...attempt,
          state: nonCanonicalState,
          assignmentScoringState: "current",
          questionPoolSelectionPosition: null,
        }),
      DecodeError,
      `${nonCanonicalState} must be rejected`,
    );
  }
});

test("Student Question Pool Selection Position exposes only a valid server-selected ordinal", () => {
  const attempt = structuredClone(publishedQuestionFixture.attempts[0]);
  const pooled = {
    ...attempt,
    assignmentScoringState: "current",
    questionPoolSelectionPosition: { selectedQuestionNumber: 1, selectedQuestionCount: 2 },
  };
  assert.deepEqual(decodeStudentQuestionAttempt(pooled), pooled);
  assert.throws(
    () =>
      decodeStudentQuestionAttempt({
        ...pooled,
        questionPoolSelectionPosition: { selectedQuestionNumber: 3, selectedQuestionCount: 2 },
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeStudentQuestionAttempt({
        ...pooled,
        questionPoolSelectionPosition: {
          selectedQuestionNumber: 1,
          selectedQuestionCount: 2,
          seed: 7,
        },
      }),
    DecodeError,
  );
});

test("Student Issued Question excludes durable Question Pool Selection evidence", () => {
  const issuedQuestion = structuredClone(publishedQuestionFixture.issuedQuestions[0]);
  assert.deepEqual(decodeStudentIssuedQuestion(issuedQuestion), issuedQuestion);
  assert.throws(
    () =>
      decodeStudentIssuedQuestion({
        ...issuedQuestion,
        questionPoolSelection: "0198e000-0000-7000-8000-000000000060",
      }),
    DecodeError,
  );
});

test("Assignment Attempt summary decoder accepts only its compact redacted wire shape", () => {
  const assignmentAttempt = publishedQuestionFixture.assignment_attempts[0];
  const summary = {
    course: {
      summary: publishedQuestionFixture.course,
      appearance: { theme: "grass", revision: "1", banner: null },
    },
    assignmentAttempt,
    summary: studentProgress,
    outcomes: {
      items: [
        {
          attempt: publishedQuestionFixture.attempts[0].id,
          issuedQuestion: publishedQuestionFixture.issuedQuestions[0],
          submittedAt: 1,
          response: null,
          feedback: null,
          assignmentScoringState: "current",
        },
      ],
      nextCursor: null,
    },
  };
  assert.deepEqual(decodeAssignmentAttemptSummaryResponse(summary), summary);
  assert.throws(
    () => decodeAssignmentAttemptSummaryResponse({ ...summary, policy: "onRelease" }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeAssignmentAttemptSummaryResponse({
        ...summary,
        summary: { ...summary.summary, privateScope: "0198e000-0000-7000-8000-000000000099" },
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeAssignmentAttemptSummaryResponse({
        ...summary,
        outcomes: {
          ...summary.outcomes,
          items: [{ ...summary.outcomes.items[0], result: { correct: true } }],
        },
      }),
    DecodeError,
  );
});

test("submission receipts reject hostile private grading data", () => {
  const { questionPoolSelectionPosition: _questionPoolSelectionPosition, ...attempt } =
    publishedQuestionFixture.attempts[0];
  const receipt = {
    accepted: true,
    attempt,
    feedback: { correctness: true },
    assignmentScoringState: "current",
    assignmentAttemptCompletion: "inProgress",
    nextIssued: null,
    nextPending: false,
  };
  assert.deepEqual(decodeGradedQuestionSubmissionReceipt(receipt), {
    ...receipt,
    attemptId: receipt.attempt.id,
  });
  assert.throws(
    () =>
      decodeGradedQuestionSubmissionReceipt({
        ...receipt,
        feedback: { correctness: true, token: "no" },
      }),
    DecodeError,
  );

  for (const assignmentScoringState of ["recalculating", "failed"]) {
    const redacted = structuredClone(receipt);
    redacted.assignmentScoringState = assignmentScoringState;
    redacted.attempt.submission.gradingResult = null;
    redacted.feedback = { correctness: true };
    assert.deepEqual(decodeGradedQuestionSubmissionReceipt(redacted), {
      ...redacted,
      attemptId: redacted.attempt.id,
    });

    const resultLeak = structuredClone(redacted);
    resultLeak.attempt.submission.gradingResult = receipt.attempt.submission.gradingResult;
    assert.throws(() => decodeGradedQuestionSubmissionReceipt(resultLeak), DecodeError);

    const pointLeak = structuredClone(redacted);
    pointLeak.feedback = { correctness: true, pointsEarned: 1, pointsPossible: 1 };
    assert.throws(() => decodeGradedQuestionSubmissionReceipt(pointLeak), DecodeError);
  }
  const { assignmentScoringState: _assignmentScoringState, ...withoutAssignmentScoringState } =
    receipt;
  assert.throws(
    () => decodeGradedQuestionSubmissionReceipt(withoutAssignmentScoringState),
    DecodeError,
  );
  const {
    assignmentAttemptCompletion: _assignmentAttemptCompletion,
    ...withoutAssignmentAttemptCompletion
  } = receipt;
  assert.throws(
    () => decodeGradedQuestionSubmissionReceipt(withoutAssignmentAttemptCompletion),
    DecodeError,
  );
  for (const [path, forbidden] of [
    ["answerKey", "answerKey"],
    ["timing.key", "key"],
    ["reproductionDetails", { backend: { id: "private", version: "1" } }],
    ["submission.gradingResult.checker", "checker"],
  ]) {
    const hostile = structuredClone(receipt);
    const fields = path.split(".");
    let target = hostile.attempt;
    for (const field of fields.slice(0, -1)) {
      if (field === "answerKey") break;
      if (target[field] === null) target[field] = {};
      target = target[field];
    }
    target[fields.at(-1)] = forbidden;
    assert.throws(
      () => decodeGradedQuestionSubmissionReceipt(hostile),
      DecodeError,
      `rejects ${path}`,
    );
  }
  const { feedback: _feedback, ...withoutFeedback } = receipt;
  assert.throws(() => decodeGradedQuestionSubmissionReceipt(withoutFeedback), DecodeError);
  assert.throws(
    () =>
      decodeGradedQuestionSubmissionReceipt({
        ...receipt,
        nextIssued: {
          id: "0198e000-0000-7000-8000-000000000035",
          issuedQuestion: publishedQuestionFixture.issuedQuestions[1],
          question_seed: receipt.attempt.question_seed,
          deadline: null,
          renderedQuestionSha256: "b".repeat(64),
        },
        nextPending: true,
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeGradedQuestionSubmissionReceipt({
        ...receipt,
        assignmentAttemptCompletion: "completed",
        nextIssued: {
          id: "0198e000-0000-7000-8000-000000000035",
          issuedQuestion: publishedQuestionFixture.issuedQuestions[1],
          question_seed: receipt.attempt.question_seed,
          deadline: null,
          renderedQuestionSha256: "b".repeat(64),
        },
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeGradedQuestionSubmissionReceipt({
        ...receipt,
        assignmentAttemptCompletion: "completed",
        nextPending: true,
      }),
    DecodeError,
  );
});

test("disclosed Student Feedback rejects private grading data and malformed blocks", () => {
  const feedback = { correctness: true };
  for (const forbidden of [
    "answerKey",
    "expectedValue",
    "checkerState",
    "providerTranscript",
    "sourcePackage",
    "solutionUrl",
    "launchUrl",
    "credential",
    "token",
  ]) {
    assert.throws(
      () => decodeStudentFeedback({ ...feedback, [forbidden]: "private" }),
      DecodeError,
      `feedback must reject ${forbidden}`,
    );
  }
  assert.throws(
    () =>
      decodeStudentFeedback({
        choiceFeedback: [{ kind: "text", markdown: "Try again.", providerTranscript: "private" }],
      }),
    DecodeError,
    "Student Feedback must reject private nested provider data",
  );
});
