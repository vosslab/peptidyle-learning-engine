// test_run_page_recovery.mjs - page-composition proof for reauthentication delivery.

import assert from "node:assert/strict";
import test from "node:test";

import { createRoot } from "solid-js";

import { createSubmissionController } from "../src/components/response_widget.tsx";
import { ApiProtocolError, ApiRequestError } from "../src/api/http_client/error.ts";
import { resumeSessionAndRetry } from "../src/pages/run_page_recovery.ts";
import { createAttemptStateMachine } from "../src/features/attempt/attempt_state.ts";

function createStorage() {
  const values = new Map();
  return {
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => values.set(key, value),
    removeItem: (key) => values.delete(key),
  };
}

test("session expiry, reauthentication, and page retry submit one saved response with its original key", async () => {
  const submissions = [];
  let callCount = 0;
  const machine = createAttemptStateMachine({
    context: {
      tenantId: "tenant-a",
      runId: "run-a",
      attemptId: "attempt-a",
      questionVersion: "version-a",
      seed: 2,
      deadline: null,
    },
    storage: createStorage(),
    clock: { now: () => 1_000 },
    network: { isOnline: () => true },
    generateIdempotencyKey: () => "saved-key",
    submitResponse: async (attemptId, response, idempotencyKey) => {
      submissions.push({ attemptId, response, idempotencyKey });
      callCount += 1;
      if (callCount === 1) throw new Error("session expired");
      return {
        accepted: true,
        attempt: {
          id: "attempt-a",
          tenant: "tenant-a",
          run: "run-a",
          problem: "problem-a",
          questionVersion: "version-a",
          assignmentPosition: 0,
          seed: 2,
          parameterHash: "hash",
          response,
          result: null,
          timer: { issuedAt: 0, deadline: null, submittedAt: 1 },
          provenance: { backend: "native", implementationVersion: "1", sourceChecksum: null },
        },
        feedback: null,
        scoringStatus: "current",
        runCompletionStatus: "inProgress",
        nextIssued: null,
        nextPending: false,
      };
    },
    isSessionExpired: (error) => error instanceof Error && error.message === "session expired",
    isTransientTransportFailure: (error) => error instanceof TypeError,
  });
  const response = { kind: "numeric", value: 7 };

  machine.start();
  machine.setResponse(response, { valid: true, message: null });
  const initialOutcome = await machine.submit();
  assert.equal(machine.state().phase, "recovering");
  assert.equal(machine.state().reason, "sessionExpired");
  assert.deepEqual(initialOutcome, {
    kind: "recoveryPending",
    reason: "sessionExpired",
    message: "session expired",
  });

  let sessionChecks = 0;
  const retryOutcome = await resumeSessionAndRetry(async () => {
    sessionChecks += 1;
    return { authenticated: true };
  }, machine);

  assert.equal(sessionChecks, 1);
  assert.deepEqual(retryOutcome, { kind: "accepted" });
  assert.equal(submissions.length, 2);
  assert.deepEqual(
    submissions.map((submission) => submission.response),
    [response, response],
  );
  assert.deepEqual(
    submissions.map((submission) => submission.idempotencyKey),
    ["saved-key", "saved-key"],
  );
  assert.equal(machine.state().phase, "feedback");
});

test("the response controller exposes 422 and receipt failures for correction before resubmission", async () => {
  for (const failure of [
    new ApiRequestError(
      422,
      "/api/courses/course-a/assignments/assignment-a/attempts/attempt-a/submissions",
    ),
    new ApiProtocolError("Submission receipt attempt does not match its request"),
  ]) {
    const submissionKeys = [];
    let submissions = 0;
    const machine = createAttemptStateMachine({
      context: {
        tenantId: "tenant-a",
        runId: "run-a",
        attemptId: "attempt-a",
        questionVersion: "version-a",
        seed: 2,
        deadline: null,
      },
      storage: createStorage(),
      clock: { now: () => 1_000 },
      network: { isOnline: () => true },
      generateIdempotencyKey: () => {
        const key = `correction-key-${submissionKeys.length + 1}`;
        submissionKeys.push(key);
        return key;
      },
      submitResponse: async (attemptId, response, _idempotencyKey) => {
        submissions += 1;
        if (submissions === 1) throw failure;
        return {
          accepted: true,
          attempt: {
            id: attemptId,
            tenant: "tenant-a",
            run: "run-a",
            problem: "problem-a",
            questionVersion: "version-a",
            assignmentPosition: 0,
            seed: 2,
            parameterHash: "hash",
            response,
            result: null,
            timer: { issuedAt: 0, deadline: null, submittedAt: 1 },
            provenance: { backend: "native", implementationVersion: "1", sourceChecksum: null },
          },
          feedback: null,
          scoringStatus: "current",
          runCompletionStatus: "inProgress",
          nextIssued: null,
          nextPending: false,
        };
      },
      isSessionExpired: () => false,
      isTransientTransportFailure: (error) => error instanceof TypeError,
    });
    const controller = createRoot(() =>
      createSubmissionController({
        attemptId: "attempt-a",
        definition: { kind: "numeric", tolerance: { kind: "exact" }, unit: null },
        validator: {
          mode: "wasm",
          validateResponseFormat: async () => ({ violations: [] }),
        },
        onEscape: () => undefined,
        onResponseChange: (response, validation) => {
          machine.setResponse(response, {
            valid: validation.violations.length === 0,
            message: null,
          });
        },
        onSubmit: async (response) => {
          machine.setResponse(response, { valid: true, message: null });
          return machine.submit();
        },
      }),
    );
    const refused = { kind: "numeric", value: 7 };
    const corrected = { kind: "numeric", value: 8 };

    machine.start();
    await controller.validate(refused);
    await controller.submit(refused);

    assert.equal(machine.state().phase, "recovering");
    assert.equal(machine.state().reason, "requestFailed");
    const failurePhase = controller.phase();
    assert.equal(failurePhase.kind, "failed");
    if (failurePhase.kind === "failed") assert.equal(failurePhase.message, failure.message);
    assert.equal(controller.locked(), false);

    await controller.validate(corrected);
    await controller.submit(corrected);

    assert.equal(machine.state().phase, "feedback");
    assert.equal(controller.phase().kind, "submitted");
    assert.deepEqual(submissionKeys, ["correction-key-1", "correction-key-2"]);
  }
});
