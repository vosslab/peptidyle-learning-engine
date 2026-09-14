// Public feedback must stay an exact policy-redacted contract at the browser boundary.

import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import {
  decodeStudentFeedback,
  decodeStudentQuestionAttempt,
  decodeStudentIssuedQuestion,
} from "../src/api/decoders.ts";
import { publishedQuestionFixture } from "./fixtures/published_question.ts";

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
  assert.throws(
    () => decodeStudentIssuedQuestion({ ...issuedQuestion, issuedPosition: 0 }),
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
