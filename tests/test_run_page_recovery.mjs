// test_run_page_recovery.mjs - page-composition proof for reauthentication delivery.

import assert from "node:assert/strict";
import test from "node:test";

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
      };
    },
    isSessionExpired: (error) => error instanceof Error && error.message === "session expired",
  });
  const response = { kind: "numeric", value: 7 };

  machine.start();
  machine.setResponse(response, { valid: true, message: null });
  await machine.submit();
  assert.equal(machine.state().phase, "recovering");
  assert.equal(machine.state().reason, "sessionExpired");

  let sessionChecks = 0;
  await resumeSessionAndRetry(async () => {
    sessionChecks += 1;
    return { authenticated: true };
  }, machine);

  assert.equal(sessionChecks, 1);
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
