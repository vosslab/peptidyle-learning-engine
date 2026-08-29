// Public feedback must stay an exact policy-redacted contract at the browser boundary.

import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import {
  decodeDisclosedFeedback,
  decodeStudentQuestionAttempt,
  decodeQuestionAttempt,
  decodeRunSummaryResponse,
  decodeSubmissionReceipt,
} from "../src/api/decoders.ts";
import { publishedProblemFixture } from "./fixtures/published_problem.ts";

const studentProgress = {
  scoring_status: "current",
  score_state: "available",
  current_score: 0,
  best_score: 1,
  latest_score: 0,
  completed_run_count: 2,
  total_question_attempts: 4,
  last_activity_at: 1786000000000,
};

test("disclosed feedback preserves allowed accessible blocks and optional omission", () => {
  const feedback = {
    correctness: false,
    hint: [{ kind: "math", latex: "x + 1", description: "x plus one" }],
  };
  assert.deepEqual(decodeDisclosedFeedback(feedback), feedback);
  assert.deepEqual(decodeDisclosedFeedback({}), {});
});

test("Student attempts require score freshness and redact stale numeric results", () => {
  const attempt = structuredClone(publishedProblemFixture.attempts[0]);
  const current = { ...attempt, scoringStatus: "current" };
  assert.deepEqual(decodeStudentQuestionAttempt(current), current);
  assert.throws(
    () => decodeQuestionAttempt(current),
    DecodeError,
    "the storage attempt decoder must stay exact",
  );

  for (const scoringStatus of ["recalculating", "failed"]) {
    const redacted = { ...attempt, result: null, scoringStatus };
    assert.deepEqual(decodeStudentQuestionAttempt(redacted), redacted);
    assert.throws(
      () => decodeStudentQuestionAttempt({ ...redacted, result: attempt.result }),
      DecodeError,
      `${scoringStatus} must reject a numeric result`,
    );
  }

  const { scoringStatus: _scoringStatus, ...missingStatus } = current;
  assert.throws(() => decodeStudentQuestionAttempt(missingStatus), DecodeError);
  assert.throws(
    () => decodeStudentQuestionAttempt({ ...attempt, scoringStatus: "stale" }),
    DecodeError,
  );
});

test("attempt decoder rejects the retired manual attempt status", () => {
  const attempt = structuredClone(publishedProblemFixture.attempts[0]);
  assert.throws(
    () => decodeStudentQuestionAttempt({ ...attempt, status: "needs_manual_grading" }),
    DecodeError,
  );
});

test("Student pool provenance exposes only a valid server-selected ordinal", () => {
  const attempt = structuredClone(publishedProblemFixture.attempts[0]);
  const pooled = {
    ...attempt,
    scoringStatus: "current",
    poolSelection: { itemNumber: 1, itemCount: 2 },
  };
  assert.deepEqual(decodeStudentQuestionAttempt(pooled), pooled);
  assert.throws(
    () =>
      decodeStudentQuestionAttempt({ ...pooled, poolSelection: { itemNumber: 3, itemCount: 2 } }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeStudentQuestionAttempt({
        ...pooled,
        poolSelection: { itemNumber: 1, itemCount: 2, seed: 7 },
      }),
    DecodeError,
  );
});

test("run summary decoder accepts only its compact redacted wire shape", () => {
  const run = publishedProblemFixture.runs[0];
  const summary = {
    course: {
      summary: publishedProblemFixture.course,
      appearance: { theme: "grass", revision: "1", banner: null },
    },
    run,
    summary: studentProgress,
    practiceAllowed: true,
    outcomes: {
      items: [
        {
          attempt: publishedProblemFixture.attempts[0].id,
          assignmentPosition: 0,
          submittedAt: 1,
          response: null,
          feedback: null,
          scoringStatus: "current",
        },
      ],
      nextCursor: null,
    },
  };
  assert.deepEqual(decodeRunSummaryResponse(summary), summary);
  assert.throws(() => decodeRunSummaryResponse({ ...summary, policy: "onRelease" }), DecodeError);
  assert.throws(
    () =>
      decodeRunSummaryResponse({
        ...summary,
        summary: { ...summary.summary, tenant: "0198e000-0000-7000-8000-000000000099" },
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeRunSummaryResponse({
        ...summary,
        outcomes: {
          ...summary.outcomes,
          items: [{ ...summary.outcomes.items[0], result: { correct: true } }],
        },
      }),
    DecodeError,
  );
});

test("submission receipts require an exact feedback field and reject hostile nested material", () => {
  const { poolSelection: _poolSelection, ...attempt } = publishedProblemFixture.attempts[0];
  const receipt = {
    accepted: true,
    attempt,
    feedback: { correctness: true },
    scoringStatus: "current",
    runCompletionStatus: "inProgress",
    nextIssued: null,
    nextPending: false,
  };
  assert.deepEqual(decodeSubmissionReceipt(receipt), receipt);
  assert.throws(
    () => decodeSubmissionReceipt({ ...receipt, feedback: { correctness: true, token: "no" } }),
    DecodeError,
  );

  for (const scoringStatus of ["recalculating", "failed"]) {
    const redacted = structuredClone(receipt);
    redacted.scoringStatus = scoringStatus;
    redacted.attempt.result = null;
    redacted.feedback = { correctness: true };
    assert.deepEqual(decodeSubmissionReceipt(redacted), redacted);

    const resultLeak = structuredClone(redacted);
    resultLeak.attempt.result = receipt.attempt.result;
    assert.throws(() => decodeSubmissionReceipt(resultLeak), DecodeError);

    const pointLeak = structuredClone(redacted);
    pointLeak.feedback = { correctness: true, pointsEarned: 1, pointsPossible: 1 };
    assert.throws(() => decodeSubmissionReceipt(pointLeak), DecodeError);
  }
  const { scoringStatus: _scoringStatus, ...withoutScoringStatus } = receipt;
  assert.throws(() => decodeSubmissionReceipt(withoutScoringStatus), DecodeError);
  const { runCompletionStatus: _runCompletionStatus, ...withoutRunCompletionStatus } = receipt;
  assert.throws(() => decodeSubmissionReceipt(withoutRunCompletionStatus), DecodeError);
  for (const [path, forbidden] of [
    ["answerKey", "answerKey"],
    ["timer.key", "key"],
    ["provenance.adapter.provider", "provider"],
    ["provenance.sourceArtifact.source", "source"],
    ["result.checker", "checker"],
  ]) {
    const hostile = structuredClone(receipt);
    const fields = path.split(".");
    let target = hostile.attempt;
    for (const field of fields.slice(0, -1)) {
      if (field === "answerKey") break;
      if (target[field] === null) {
        target[field] =
          field === "sourceArtifact"
            ? {
                object: "0198e000-0000-7000-8000-000000000011",
                sha256: "4cddff550d3e53f980baab609ada99a57ca7854edbfd2426f2c8db7cd43a6c01",
              }
            : {};
      }
      target = target[field];
    }
    target[fields.at(-1)] = forbidden;
    assert.throws(() => decodeSubmissionReceipt(hostile), DecodeError, `rejects ${path}`);
  }
  const { feedback: _feedback, ...withoutFeedback } = receipt;
  assert.throws(() => decodeSubmissionReceipt(withoutFeedback), DecodeError);
  assert.throws(
    () =>
      decodeSubmissionReceipt({
        ...receipt,
        nextIssued: {
          id: "0198e000-0000-7000-8000-000000000035",
          run: receipt.attempt.run,
          questionVersion: receipt.attempt.questionVersion,
          seed: receipt.attempt.seed,
          deadline: null,
          assignmentPosition: receipt.attempt.assignmentPosition + 1,
          renderedQuestionSha256: "b".repeat(64),
        },
        nextPending: true,
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeSubmissionReceipt({
        ...receipt,
        runCompletionStatus: "completed",
        nextIssued: {
          id: "0198e000-0000-7000-8000-000000000035",
          run: receipt.attempt.run,
          questionVersion: receipt.attempt.questionVersion,
          seed: receipt.attempt.seed,
          deadline: null,
          assignmentPosition: receipt.attempt.assignmentPosition + 1,
          renderedQuestionSha256: "b".repeat(64),
        },
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeSubmissionReceipt({
        ...receipt,
        runCompletionStatus: "completed",
        nextPending: true,
      }),
    DecodeError,
  );
});

test("disclosed feedback rejects private material and malformed blocks", () => {
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
      () => decodeDisclosedFeedback({ ...feedback, [forbidden]: "private" }),
      DecodeError,
      `feedback must reject ${forbidden}`,
    );
  }
  assert.throws(
    () =>
      decodeDisclosedFeedback({
        hint: [{ kind: "text", markdown: "Try again.", providerTranscript: "private" }],
      }),
    DecodeError,
    "feedback blocks must reject unknown provider fields",
  );
});
