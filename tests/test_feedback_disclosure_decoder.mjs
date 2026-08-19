// Public feedback must stay an exact policy-redacted contract at the browser boundary.

import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import {
  decodeDisclosedFeedback,
  decodeRunSummaryResponse,
  decodeSubmissionReceipt,
} from "../src/api/decoders.ts";
import { publishedProblemFixture } from "../generated/fixtures/published_problem.ts";

const learnerProgress = {
  scoreState: "available",
  currentScore: 0,
  bestScore: 1,
  latestScore: 0,
  completedRunCount: 2,
  totalQuestionAttempts: 4,
  lastActivityAt: 1786000000000,
};

test("disclosed feedback preserves allowed accessible blocks and optional omission", () => {
  const feedback = {
    correctness: false,
    hint: [{ kind: "math", latex: "x + 1", description: "x plus one" }],
  };
  assert.deepEqual(decodeDisclosedFeedback(feedback), feedback);
  assert.deepEqual(decodeDisclosedFeedback({}), {});
});

test("run summary decoder accepts only its compact redacted wire shape", () => {
  const run = publishedProblemFixture.runs[0];
  const summary = {
    course: {
      summary: publishedProblemFixture.course,
      appearance: { theme: "grass", revision: "1", banner: null },
    },
    run,
    summary: learnerProgress,
    practiceAllowed: true,
    outcomes: {
      items: [
        {
          attempt: publishedProblemFixture.attempts[0].id,
          assignmentPosition: 0,
          submittedAt: 1,
          response: null,
          feedback: null,
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
  const receipt = {
    accepted: true,
    attempt: publishedProblemFixture.attempts[0],
    feedback: { correctness: true },
    nextIssued: null,
    nextPending: false,
  };
  assert.deepEqual(decodeSubmissionReceipt(receipt), receipt);
  assert.throws(
    () => decodeSubmissionReceipt({ ...receipt, feedback: { correctness: true, token: "no" } }),
    DecodeError,
  );
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
