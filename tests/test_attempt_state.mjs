// test_attempt_state.mjs - permanent behavior checks for learner attempt recovery semantics.

import assert from "node:assert/strict";
import test from "node:test";

import { createAttemptStateMachine } from "../src/features/attempt/attempt_state.ts";
import { ApiProtocolError, ApiRequestError } from "../src/api/http_client/error.ts";
import { validateSavedResponse } from "./http_client_test_support.mjs";

function createStorage() {
  const values = new Map();
  return {
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => values.set(key, value),
    removeItem: (key) => values.delete(key),
    value: (key) => values.get(key) ?? null,
  };
}

function createContext(overrides = {}) {
  return {
    tenantId: "tenant-a",
    runId: "run-a",
    attemptId: "attempt-a",
    questionVersion: "version-a",
    seed: 2,
    deadline: null,
    ...overrides,
  };
}

function receipt() {
  return {
    kind: "completed",
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
      response: { kind: "numeric", value: 7 },
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
}

function numericResponse(value = 7) {
  return { kind: "numeric", value };
}

function createMachine(overrides = {}) {
  let online = true;
  let now = 1_000;
  const storage = createStorage();
  const keys = [];
  const submissionCalls = [];
  const options = {
    context: createContext(),
    storage,
    clock: { now: () => now },
    network: { isOnline: () => online },
    generateIdempotencyKey: () => {
      const key = `key-${keys.length + 1}`;
      keys.push(key);
      return key;
    },
    submitResponse: async (attemptId, response, key) => {
      submissionCalls.push({ attemptId, response, key });
      return receipt();
    },
    getSubmissionStatus: async () => receipt(),
    isSessionExpired: (error) => error instanceof Error && error.message === "session expired",
    isTransientTransportFailure: (error) => error instanceof TypeError,
    ...overrides,
  };
  const machine = createAttemptStateMachine(options);
  return {
    machine,
    storage,
    keys,
    submissionCalls,
    setOnline: (value) => {
      online = value;
    },
    setNow: (value) => {
      now = value;
    },
  };
}

function ready(machine, response = numericResponse()) {
  machine.start();
  machine.setResponse(response, { valid: true, message: null });
}

test("a retry reuses one idempotency key for the logical response", async () => {
  let calls = 0;
  const fixture = createMachine({
    submitResponse: async (attemptId, response, key) => {
      calls += 1;
      fixture.submissionCalls.push({ attemptId, response, key });
      if (calls === 1) throw new TypeError("temporary outage");
      return receipt();
    },
  });
  ready(fixture.machine);

  await fixture.machine.submit();
  assert.equal(fixture.machine.state().phase, "recovering");
  await fixture.machine.retry();

  assert.equal(fixture.submissionCalls.length, 2);
  assert.equal(fixture.submissionCalls[0].key, fixture.submissionCalls[1].key);
  assert.deepEqual(fixture.keys, ["key-1"]);
});

test("an in-flight submit cannot create a duplicate grading request", async () => {
  let resolveSubmission;
  const pending = new Promise((resolve) => {
    resolveSubmission = resolve;
  });
  const fixture = createMachine({
    submitResponse: async (attemptId, response, key) => {
      fixture.submissionCalls.push({ attemptId, response, key });
      await pending;
      return receipt();
    },
  });
  ready(fixture.machine);

  const first = fixture.machine.submit();
  const duplicate = fixture.machine.submit();
  assert.equal(fixture.machine.state().phase, "submitting");
  assert.equal(fixture.submissionCalls.length, 1);
  resolveSubmission();
  await Promise.all([first, duplicate]);
  assert.equal(fixture.submissionCalls.length, 1);
});

test("a receipt with a pending successor preserves feedback without resubmitting", async () => {
  const fixture = createMachine({
    submitResponse: async (attemptId, response, key) => {
      fixture.submissionCalls.push({ attemptId, response, key });
      return { ...receipt(), nextPending: true };
    },
  });
  ready(fixture.machine);

  await fixture.machine.submit();

  const state = fixture.machine.state();
  assert.equal(state.phase, "feedback");
  assert.equal(state.acknowledgement.nextPending, true);
  assert.equal(fixture.submissionCalls.length, 1);
  assert.equal(state.response.kind, "numeric");
});

test("an acknowledged pending submission clears its replay and checks status without another post", async () => {
  let statusReads = 0;
  const fixture = createMachine({
    submitResponse: async (attemptId, response, key) => {
      fixture.submissionCalls.push({ attemptId, response, key });
      return {
        kind: "accepted_pending",
        accepted: true,
        attemptId,
        automatedGradingStatus: "pending",
        nextAction: "check_status",
      };
    },
    getSubmissionStatus: async () => {
      statusReads += 1;
      return statusReads === 1
        ? {
            kind: "instructor_attention",
            accepted: true,
            attemptId: "attempt-a",
            automatedGradingStatus: "instructor_attention",
            nextAction: "check_status",
          }
        : receipt();
    },
  });
  ready(fixture.machine, numericResponse(11));

  await fixture.machine.submit();
  assert.equal(fixture.machine.state().phase, "acceptedPending");
  assert.equal(fixture.storage.value("ple:attempt:tenant-a:run-a:attempt-a"), null);

  await fixture.machine.submit();
  await fixture.machine.checkGradingStatus();
  assert.equal(fixture.submissionCalls.length, 1);
  assert.equal(fixture.machine.state().phase, "acceptedPending");
  assert.equal(
    fixture.machine.state().acknowledgement.automatedGradingStatus,
    "instructor_attention",
  );

  await fixture.machine.checkGradingStatus();
  assert.equal(statusReads, 2);
  assert.equal(fixture.machine.state().phase, "feedback");
});

test("offline submission keeps the controlled response locally and retries after reconnect", async () => {
  const fixture = createMachine();
  ready(fixture.machine, numericResponse(11));
  fixture.setOnline(false);

  await fixture.machine.submit();
  assert.equal(fixture.machine.state().phase, "recovering");
  assert.equal(fixture.machine.state().response.value, 11);
  assert.equal(
    fixture.machine.state().message,
    "Your response is retained in this browser. Reconnect, then retry submission.",
  );
  assert.equal(fixture.submissionCalls.length, 0);

  fixture.setOnline(true);
  await fixture.machine.retryWhenOnline();
  assert.equal(fixture.submissionCalls.length, 1);
  assert.equal(fixture.machine.state().phase, "feedback");
});

test("a browser transport outage retains the response and gives a stable restoration action", async () => {
  const fixture = createMachine({
    submitResponse: async () => {
      throw new TypeError("gateway connection reset by peer");
    },
  });
  ready(fixture.machine, numericResponse(11));

  await fixture.machine.submit();

  const state = fixture.machine.state();
  assert.equal(state.phase, "recovering");
  assert.equal(state.reason, "network");
  assert.equal(state.response.value, 11);
  assert.equal(
    state.message,
    "Your response is retained in this browser. Retry submission after the service is restored.",
  );
});

test("a server refusal keeps its actionable API message instead of claiming a service outage", async () => {
  const refusal = new ApiRequestError(
    422,
    "/api/courses/course-a/assignments/assignment-a/attempts/attempt-a/submissions",
  );
  let calls = 0;
  const fixture = createMachine({
    submitResponse: async () => {
      calls += 1;
      throw refusal;
    },
  });
  ready(fixture.machine, numericResponse(11));

  const outcome = await fixture.machine.submit();

  const state = fixture.machine.state();
  assert.equal(state.phase, "recovering");
  assert.equal(state.reason, "requestFailed");
  assert.equal(state.message, refusal.message);
  assert.equal(state.response.value, 11);
  assert.deepEqual(outcome, { kind: "rejected", message: refusal.message });
  await fixture.machine.retry();
  assert.equal(calls, 1);
});

test("a submission receipt protocol failure keeps its correction message instead of claiming a service outage", async () => {
  const protocolFailure = new ApiProtocolError(
    "Submission receipt attempt does not match its request",
  );
  let calls = 0;
  const fixture = createMachine({
    submitResponse: async () => {
      calls += 1;
      throw protocolFailure;
    },
  });
  ready(fixture.machine, numericResponse(11));

  const outcome = await fixture.machine.submit();

  const state = fixture.machine.state();
  assert.equal(state.phase, "recovering");
  assert.equal(state.reason, "requestFailed");
  assert.equal(state.message, protocolFailure.message);
  assert.equal(state.response.value, 11);
  assert.deepEqual(outcome, { kind: "rejected", message: protocolFailure.message });
  await fixture.machine.retry();
  assert.equal(calls, 1);
});

test("reload restores the saved response and its existing replay key", async () => {
  const storage = createStorage();
  const first = createMachine({ storage });
  ready(first.machine, numericResponse(12));
  first.setOnline(false);
  await first.machine.submit();

  const second = createMachine({ storage });
  second.machine.start();
  assert.deepEqual(second.machine.state().response, numericResponse(12));
  second.machine.setResponse(numericResponse(12), { valid: true, message: null });
  second.setOnline(true);
  await second.machine.submit();
  assert.equal(second.submissionCalls[0].key, "key-1");
});

test("run exit clears only the active attempt buffer", () => {
  const storage = createStorage();
  const otherAttemptKey = "ple:attempt:tenant-a:run-a:attempt-b";
  storage.setItem(otherAttemptKey, "other-attempt-buffer");
  const fixture = createMachine({ storage });
  ready(fixture.machine, numericResponse(12));

  assert.notEqual(storage.getItem("ple:attempt:tenant-a:run-a:attempt-a"), null);
  fixture.machine.dispose();

  assert.equal(storage.getItem("ple:attempt:tenant-a:run-a:attempt-a"), null);
  assert.equal(storage.getItem(otherAttemptKey), "other-attempt-buffer");
});

test("a hostile saved multiple-choice response is discarded before it reaches an uneditable control", async () => {
  const storage = createStorage();
  storage.setItem(
    "ple:attempt:tenant-a:run-a:attempt-a",
    JSON.stringify({
      response: { kind: "multipleChoice", selected: ["known", "forged", "known"] },
      idempotencyKey: "hostile-key",
    }),
  );
  const fixture = createMachine({ storage, validateSavedResponse });
  const definition = {
    kind: "multipleChoice",
    choices: [{ id: "known", body: [] }],
    selection: { kind: "anyNumber" },
  };

  fixture.machine.start(definition);
  await new Promise((resolve) => setImmediate(resolve));

  assert.equal(fixture.machine.state().phase, "answering");
  assert.equal(fixture.machine.state().response, null);
  assert.match(fixture.machine.state().storageWarning ?? "", /not valid for this question/);
  assert.equal(storage.getItem("ple:attempt:tenant-a:run-a:attempt-a"), null);
});

test("a hostile saved ordering is discarded and a valid saved permutation retains its replay key", async () => {
  const orderingDefinition = {
    kind: "ordering",
    items: [
      { id: "first", body: [] },
      { id: "second", body: [] },
    ],
  };
  const hostileStorage = createStorage();
  hostileStorage.setItem(
    "ple:attempt:tenant-a:run-a:attempt-a",
    JSON.stringify({
      response: { kind: "ordering", order: ["first", "forged"] },
      idempotencyKey: "hostile-order-key",
    }),
  );
  const hostile = createMachine({
    storage: hostileStorage,
    validateSavedResponse,
  });
  hostile.machine.start(orderingDefinition);
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(hostile.machine.state().response, null);

  const validStorage = createStorage();
  validStorage.setItem(
    "ple:attempt:tenant-a:run-a:attempt-a",
    JSON.stringify({
      response: { kind: "ordering", order: ["second", "first"] },
      idempotencyKey: "saved-order-key",
    }),
  );
  const valid = createMachine({
    storage: validStorage,
    validateSavedResponse,
  });
  valid.machine.start(orderingDefinition);
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(valid.machine.state().response, {
    kind: "ordering",
    order: ["second", "first"],
  });
  valid.machine.setResponse(
    { kind: "ordering", order: ["second", "first"] },
    { valid: true, message: null },
  );
  await valid.machine.submit();
  assert.equal(valid.submissionCalls[0].key, "saved-order-key");
});

test("session expiry requests reauthentication without losing the response", async () => {
  const fixture = createMachine({
    submitResponse: async () => {
      throw new Error("session expired");
    },
  });
  ready(fixture.machine, numericResponse(4));

  await fixture.machine.submit();
  assert.equal(fixture.machine.state().phase, "recovering");
  assert.equal(fixture.machine.state().reason, "sessionExpired");
  fixture.machine.resumeAfterReauthentication();
  assert.equal(fixture.machine.state().phase, "answering");
  assert.equal(fixture.machine.state().response.value, 4);
});

test("renderer recovery preserves the entered response and validation until the question display retries", () => {
  const fixture = createMachine();
  const response = numericResponse(4);
  const validation = { valid: true, message: null };
  fixture.machine.start();
  fixture.machine.setResponse(response, validation);

  fixture.machine.reportRendererFailure("Question diagram could not be displayed.");
  const recovering = fixture.machine.state();
  assert.equal(recovering.phase, "recovering");
  assert.equal(recovering.reason, "renderer");
  assert.equal(recovering.message, "Question diagram could not be displayed.");
  assert.deepEqual(recovering.response, response);
  assert.deepEqual(recovering.validation, validation);
  assert.equal(recovering.rendererFailure, "Question diagram could not be displayed.");

  fixture.machine.retryRenderer();
  const answering = fixture.machine.state();
  assert.equal(answering.phase, "answering");
  assert.deepEqual(answering.response, response);
  assert.deepEqual(answering.validation, validation);
  assert.equal(answering.rendererFailure, null);
});

test("server deadline submits the last valid response exactly once", async () => {
  const fixture = createMachine({ context: createContext({ deadline: 1_200 }) });
  ready(fixture.machine);
  fixture.setNow(1_199);
  fixture.machine.tick();
  assert.equal(fixture.machine.state().phase, "answering");
  assert.equal(fixture.machine.state().remainingMilliseconds, 1);
  fixture.setNow(1_200);
  fixture.machine.tick();
  assert.equal(fixture.machine.state().phase, "submitting");
  fixture.machine.tick();
  await Promise.resolve();
  assert.equal(fixture.submissionCalls.length, 1);
  assert.equal(fixture.submissionCalls[0].key, "key-1");
  assert.equal(fixture.machine.state().phase, "feedback");
});

test("a failed deadline delivery is retried only by an explicit recovery action", async () => {
  let calls = 0;
  const fixture = createMachine({
    context: createContext({ deadline: 1_200 }),
    submitResponse: async (attemptId, response, key) => {
      calls += 1;
      fixture.submissionCalls.push({ attemptId, response, key });
      if (calls === 1) throw new TypeError("temporary outage");
      return receipt();
    },
  });
  ready(fixture.machine);
  fixture.setNow(1_200);

  fixture.machine.tick();
  await Promise.resolve();
  assert.equal(fixture.machine.state().phase, "recovering");
  fixture.machine.tick();
  await Promise.resolve();
  assert.equal(fixture.submissionCalls.length, 1);

  await fixture.machine.retry();
  assert.equal(fixture.submissionCalls.length, 2);
  assert.equal(fixture.submissionCalls[0].key, fixture.submissionCalls[1].key);
  assert.equal(fixture.machine.state().phase, "feedback");
});

test("deadline ticks do not reissue an offline or reauthentication-blocked delivery", async () => {
  const offlineFixture = createMachine({ context: createContext({ deadline: 1_200 }) });
  ready(offlineFixture.machine);
  offlineFixture.setOnline(false);
  offlineFixture.setNow(1_200);
  offlineFixture.machine.tick();
  offlineFixture.machine.tick();
  assert.equal(offlineFixture.machine.state().phase, "recovering");
  assert.equal(offlineFixture.machine.state().reason, "offline");
  assert.equal(offlineFixture.submissionCalls.length, 0);

  const sessionFixture = createMachine({
    context: createContext({ deadline: 1_200 }),
    submitResponse: async (attemptId, response, key) => {
      sessionFixture.submissionCalls.push({ attemptId, response, key });
      throw new Error("session expired");
    },
  });
  ready(sessionFixture.machine);
  sessionFixture.setNow(1_200);
  sessionFixture.machine.tick();
  await Promise.resolve();
  sessionFixture.machine.tick();
  await Promise.resolve();
  assert.equal(sessionFixture.machine.state().phase, "recovering");
  assert.equal(sessionFixture.machine.state().reason, "sessionExpired");
  assert.equal(sessionFixture.submissionCalls.length, 1);
});

test("timer expiry names an unsent invalid response without grading it", () => {
  const fixture = createMachine({ context: createContext({ deadline: 1_200 }) });
  fixture.machine.start();
  fixture.setNow(1_200);

  fixture.machine.tick();
  assert.equal(fixture.machine.state().phase, "expired");
  assert.equal(fixture.machine.state().reason, "missingOrInvalidResponse");
  assert.equal(fixture.submissionCalls.length, 0);
});

test("editing an offline response creates a new logical submission key", async () => {
  const fixture = createMachine();
  ready(fixture.machine, numericResponse(7));
  fixture.setOnline(false);
  await fixture.machine.submit();

  fixture.machine.setResponse(numericResponse(8), { valid: true, message: null });
  fixture.setOnline(true);
  await fixture.machine.retry();

  assert.deepEqual(fixture.keys, ["key-1", "key-2"]);
  assert.equal(fixture.submissionCalls.length, 1);
  assert.equal(fixture.submissionCalls[0].key, "key-2");
  assert.deepEqual(fixture.submissionCalls[0].response, numericResponse(8));
});

test("editing after a failed request does not reuse that request's key", async () => {
  let calls = 0;
  const fixture = createMachine({
    submitResponse: async (attemptId, response, key) => {
      calls += 1;
      fixture.submissionCalls.push({ attemptId, response, key });
      if (calls === 1) throw new TypeError("temporary outage");
      return receipt();
    },
  });
  ready(fixture.machine, numericResponse(7));

  await fixture.machine.submit();
  fixture.machine.setResponse(numericResponse(8), { valid: true, message: null });
  await fixture.machine.retry();

  assert.equal(fixture.submissionCalls.length, 2);
  assert.notEqual(fixture.submissionCalls[0].key, fixture.submissionCalls[1].key);
  assert.deepEqual(fixture.keys, ["key-1", "key-2"]);
});

test("advance retry reloads the retained envelope without resubmitting a committed response", async () => {
  const fixture = createMachine();
  ready(fixture.machine);
  await fixture.machine.submit();
  let loads = 0;
  const next = {
    context: createContext({ attemptId: "attempt-b", questionVersion: "version-b", seed: 3 }),
    envelope: {
      version: "version-b",
      seed: 3,
      prompt: [],
      response: { kind: "numeric", tolerance: { kind: "exact" }, unit: null },
    },
  };
  async function loadNext() {
    loads += 1;
    if (loads === 1) throw new Error("next question unavailable");
    return next;
  }

  await fixture.machine.advance(loadNext);
  assert.equal(fixture.machine.state().phase, "recovering");
  assert.equal(fixture.machine.state().reason, "advanceFailed");
  await fixture.machine.retry();
  assert.equal(fixture.submissionCalls.length, 1);

  await fixture.machine.retryAdvance();
  assert.equal(fixture.submissionCalls.length, 1);
  assert.equal(fixture.machine.state().phase, "answering");
  assert.deepEqual(fixture.machine.state().envelope, next.envelope);
});

test("a mismatched next envelope preserves feedback and exposes a recoverable content error", async () => {
  for (const envelope of [
    {
      version: "version-c",
      seed: 3,
      prompt: [],
      response: { kind: "numeric", tolerance: { kind: "exact" }, unit: null },
    },
    {
      version: "version-b",
      seed: 4,
      prompt: [],
      response: { kind: "numeric", tolerance: { kind: "exact" }, unit: null },
    },
  ]) {
    const fixture = createMachine();
    ready(fixture.machine);
    await fixture.machine.submit();
    const prior = fixture.machine.state();
    assert.equal(prior.phase, "feedback");
    const invalidNext = {
      context: createContext({ attemptId: "attempt-b", questionVersion: "version-b", seed: 3 }),
      envelope,
    };

    await fixture.machine.advance(async () => invalidNext);
    const state = fixture.machine.state();
    assert.equal(state.phase, "recovering");
    assert.equal(state.reason, "advanceFailed");
    assert.match(state.message, /did not match its issued attempt/i);
    assert.equal(state.context.attemptId, "attempt-a");
    assert.deepEqual(state.feedback, prior.feedback);
    assert.equal(state.envelope, null);
  }
});

test("storage exceptions retain accepted state without exposing a raw receipt", async () => {
  const storage = {
    getItem: () => {
      throw new Error("storage disabled");
    },
    setItem: () => {
      throw new Error("storage disabled");
    },
    removeItem: () => {
      throw new Error("storage disabled");
    },
  };
  const fixture = createMachine({ storage });
  ready(fixture.machine);

  await fixture.machine.submit();
  const state = fixture.machine.state();
  assert.equal(fixture.submissionCalls.length, 1);
  assert.equal(state.phase, "feedback");
  assert.match(state.storageWarning, /accepted|saved/i);
  assert.deepEqual(state.acknowledgement, {
    accepted: true,
    attemptId: "attempt-a",
    runCompletionStatus: "inProgress",
    nextIssued: null,
    nextPending: false,
  });
  assert.equal("receipt" in state, false);
});

test("a withheld receipt is explicitly awaiting and never infers a grade from attempt.result", async () => {
  const fixture = createMachine({
    submitResponse: async () => ({
      ...receipt(),
      attempt: {
        ...receipt().attempt,
        result: { correct: true, pointsEarned: 1, pointsPossible: 1 },
      },
      feedback: null,
    }),
  });
  ready(fixture.machine);
  await fixture.machine.submit();
  assert.equal(fixture.machine.state().phase, "feedback");
  assert.deepEqual(fixture.machine.state().feedback, { kind: "awaiting", feedback: null });
});

test("an injected external-tool local buffer with provider data is discarded", () => {
  const storage = createStorage();
  storage.setItem(
    "ple:attempt:tenant-a:run-a:attempt-a",
    JSON.stringify({
      response: { kind: "externalTool", score: 100, token: "forged" },
      idempotencyKey: "key-forged",
    }),
  );
  const fixture = createMachine({ storage });
  fixture.machine.start();
  assert.equal(fixture.machine.state().response, null);
});

test("a marker-only external-tool local buffer restores unchanged", () => {
  const storage = createStorage();
  storage.setItem(
    "ple:attempt:tenant-a:run-a:attempt-a",
    JSON.stringify({ response: { kind: "externalTool" }, idempotencyKey: "key-external" }),
  );
  const fixture = createMachine({ storage });
  fixture.machine.start();
  assert.deepEqual(fixture.machine.state().response, { kind: "externalTool" });
});
