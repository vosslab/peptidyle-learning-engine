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

function currentStudentQuestionAttempt(index = 0) {
  const { reproduction: _reproduction, ...attempt } = structuredClone(
    publishedQuestionFixture.attempts[index],
  );
  return { ...attempt, assessmentScoringState: "current" };
}

function currentStudentIssuedQuestion(index = 0) {
  const {
    sourceSelection: _sourceSelection,
    ...issuedQuestion
  } = structuredClone(publishedQuestionFixture.issuedQuestions[index]);
  return issuedQuestion;
}

test("disclosed feedback preserves allowed accessible blocks and optional omission", () => {
  const feedback = {
    correctness: false,
    choiceFeedback: [{ kind: "text", markdown: "This choice has no chromosome pair." }],
    incorrectFeedback: [{ kind: "text", markdown: "Review mitosis vocabulary." }],
    generalFeedback: [{ kind: "text", markdown: "Compare homologous chromosomes." }],
  };
  assert.deepEqual(decodeStudentFeedback(feedback), feedback);
  assert.deepEqual(decodeStudentFeedback({}), {});
});

test("Student attempts require score freshness and redact stale numeric results", () => {
  const current = currentStudentQuestionAttempt();
  assert.deepEqual(decodeStudentQuestionAttempt(current), current);

  for (const assessmentScoringState of ["recalculating", "failed"]) {
    const redacted = {
      ...current,
      submission: { ...current.submission, gradingResult: null },
      assessmentScoringState,
    };
    assert.deepEqual(decodeStudentQuestionAttempt(redacted), redacted);
    assert.throws(
      () =>
        decodeStudentQuestionAttempt({
          ...redacted,
          submission: { ...redacted.submission, gradingResult: current.submission.gradingResult },
        }),
      DecodeError,
      `${assessmentScoringState} must reject a numeric result`,
    );
  }

  const { assessmentScoringState: _assessmentScoringState, ...missingState } = current;
  assert.throws(() => decodeStudentQuestionAttempt(missingState), DecodeError);
  assert.throws(
    () => decodeStudentQuestionAttempt({ ...current, assessmentScoringState: "stale" }),
    DecodeError,
  );
});

test("Student Question Pool Selection Position exposes only a valid server-selected ordinal", () => {
  const attempt = currentStudentQuestionAttempt();
  const pooled = {
    ...attempt,
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
  const issuedQuestion = currentStudentIssuedQuestion();
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
  assert.throws(
    () => decodeStudentFeedback({ ...feedback, answerKey: "private" }),
    DecodeError,
    "Student Feedback must reject private answer data",
  );
  assert.throws(
    () =>
      decodeStudentFeedback({
        choiceFeedback: [{ kind: "text", markdown: "Try again.", providerTranscript: "private" }],
      }),
    DecodeError,
    "Student Feedback must reject private nested provider data",
  );
});
