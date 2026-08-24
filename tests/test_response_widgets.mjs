// test_response_widgets.mjs - permanent behavior checks for key-free response controls.

import assert from "node:assert/strict";
import test from "node:test";

import { createRoot } from "solid-js";

import {
  createSubmissionController,
  handleWidgetKeyDown,
  isExternalToolReadyMessage,
  isSafeExternalToolLaunchPath,
  numericResponseFromInput,
  validateResponseLocally,
} from "../src/components/response_widget.tsx";

const numericDefinition = { kind: "numeric", tolerance: { kind: "exact" }, unit: null };

test("invalid input is locally checked and issues no submission request", async () => {
  let validationCalls = 0;
  let submitCalls = 0;
  const validator = {
    mode: "wasm",
    validateResponseFormat: async () => {
      validationCalls += 1;
      return { violations: [{ kind: "numericNotFinite" }] };
    },
  };
  const response = {
    kind: "numeric",
    value: Number.NaN,
  };
  const controller = createRoot(() =>
    createSubmissionController({
      attemptId: "attempt-invalid",
      definition: numericDefinition,
      validator,
      onEscape: () => undefined,
      onSubmit: async () => {
        submitCalls += 1;
        return { kind: "accepted" };
      },
    }),
  );
  const report = await validateResponseLocally(validator, numericDefinition, response);
  await controller.validate(response);
  await controller.submit(response);

  assert.equal(validationCalls, 3);
  assert.equal(report.violations[0]?.kind, "numericNotFinite");
  assert.equal(submitCalls, 0);
  assert.equal(controller.phase().kind, "invalid");
});

test("blank numeric input stays invalid and never submits zero", async () => {
  let validationCalls = 0;
  let submitCalls = 0;
  const blankResponse = numericResponseFromInput("  \t");
  const validator = {
    mode: "wasm",
    validateResponseFormat: async (_definition, candidate) => {
      validationCalls += 1;
      assert.equal(candidate.kind, "numeric");
      assert.equal(Number.isNaN(candidate.value), true);
      return { violations: [{ kind: "numericNotFinite" }] };
    },
  };
  const controller = createRoot(() =>
    createSubmissionController({
      attemptId: "attempt-blank-numeric",
      definition: numericDefinition,
      validator,
      onEscape: () => undefined,
      onSubmit: async () => {
        submitCalls += 1;
        return { kind: "accepted" };
      },
    }),
  );

  await controller.validate(blankResponse);
  assert.equal(controller.canSubmit(), false);
  await controller.submit(blankResponse);

  assert.equal(validationCalls, 2);
  assert.equal(submitCalls, 0);
  assert.equal(controller.phase().kind, "invalid");
});

test("initial controlled responses are checked before a learner edits them", async () => {
  const orderingDefinition = {
    kind: "ordering",
    items: [
      { id: "first", body: [{ kind: "text", markdown: "First" }] },
      { id: "second", body: [{ kind: "text", markdown: "Second" }] },
    ],
  };
  const initialOrder = { kind: "ordering", order: ["first", "second"] };
  let submitCalls = 0;
  const validController = createRoot(() =>
    createSubmissionController(
      {
        attemptId: "attempt-initial-order",
        definition: orderingDefinition,
        initialResponse: initialOrder,
        validator: {
          mode: "wasm",
          validateResponseFormat: async (_definition, response) => {
            assert.deepEqual(response, initialOrder);
            return { violations: [] };
          },
        },
        onEscape: () => undefined,
        onSubmit: async () => {
          submitCalls += 1;
          return { kind: "accepted" };
        },
      },
      initialOrder,
    ),
  );

  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(validController.canSubmit(), true);
  await validController.submit(initialOrder);
  assert.equal(submitCalls, 1);

  let invalidSubmitCalls = 0;
  const invalidController = createRoot(() =>
    createSubmissionController(
      {
        attemptId: "attempt-invalid-initial-order",
        definition: orderingDefinition,
        initialResponse: { kind: "ordering", order: ["first", "first"] },
        validator: {
          mode: "wasm",
          validateResponseFormat: async () => ({
            violations: [{ kind: "orderingItemsMismatch" }],
          }),
        },
        onEscape: () => undefined,
        onSubmit: async () => {
          invalidSubmitCalls += 1;
          return { kind: "accepted" };
        },
      },
      { kind: "ordering", order: ["first", "first"] },
    ),
  );

  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(invalidController.phase().kind, "invalid");
  await invalidController.submit({ kind: "ordering", order: ["first", "first"] });
  assert.equal(invalidSubmitCalls, 0);
});

test("a fresh issued empty response stays neutral until the learner interacts", async () => {
  let validationCalls = 0;
  const controller = createRoot(() =>
    createSubmissionController(
      {
        attemptId: "attempt-fresh-empty",
        definition: numericDefinition,
        validator: {
          mode: "wasm",
          validateResponseFormat: async () => {
            validationCalls += 1;
            return { violations: [{ kind: "numericNotFinite" }] };
          },
        },
        onEscape: () => undefined,
        onSubmit: async () => ({ kind: "accepted" }),
      },
      { kind: "numeric", value: Number.NaN },
    ),
  );

  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(validationCalls, 0);
  assert.equal(controller.phase().kind, "idle");
  assert.equal(controller.canSubmit(), false);
});

test("Escape returns from a widget descendant unless native handling already owns it", () => {
  let escapes = 0;
  let submits = 0;
  let prevented = false;
  const event = {
    defaultPrevented: false,
    isComposing: false,
    key: "Escape",
    target: null,
    preventDefault: () => {
      prevented = true;
    },
  };

  handleWidgetKeyDown(
    event,
    () => escapes++,
    () => submits++,
    () => true,
  );

  assert.equal(escapes, 1);
  assert.equal(submits, 0);
  assert.equal(prevented, true);

  handleWidgetKeyDown(
    { ...event, defaultPrevented: true },
    () => escapes++,
    () => submits++,
    () => true,
  );
  assert.equal(escapes, 1);
});

test("local validation is key-free and preserves a ready response for the attempt controller", async () => {
  const response = { kind: "numeric", value: 3 };
  const validator = {
    mode: "wasm",
    validateResponseFormat: async (definition, candidate) => {
      assert.equal(definition, numericDefinition);
      assert.deepEqual(candidate, response);
      return { violations: [] };
    },
  };

  assert.deepEqual(await validateResponseLocally(validator, numericDefinition, response), {
    violations: [],
  });
});

test("an in-flight submission locks the response and cannot issue a duplicate request", async () => {
  let resolveSubmit;
  const submission = new Promise((resolve) => {
    resolveSubmit = resolve;
  });
  let submitCalls = 0;
  const validator = {
    mode: "wasm",
    validateResponseFormat: async () => ({ violations: [] }),
  };
  const controller = createRoot(() =>
    createSubmissionController({
      attemptId: "attempt-1",
      definition: numericDefinition,
      validator,
      onEscape: () => undefined,
      onSubmit: async () => {
        submitCalls += 1;
        await submission;
        return { kind: "accepted" };
      },
    }),
  );
  const response = { kind: "numeric", value: 3 };

  await controller.validate(response);
  const first = controller.submit(response);
  const duplicate = controller.submit(response);
  assert.equal(controller.pending(), true);
  assert.equal(submitCalls, 1);
  resolveSubmit();
  await Promise.all([first, duplicate]);
  assert.equal(controller.pending(), false);
  assert.equal(submitCalls, 1);
});

test("reset replaces a stale format check with the restored local response without submitting", async () => {
  let resolveFirst;
  const firstValidation = new Promise((resolve) => {
    resolveFirst = resolve;
  });
  const seen = [];
  let submitCalls = 0;
  const controller = createRoot(() =>
    createSubmissionController({
      attemptId: "attempt-reset",
      definition: numericDefinition,
      validator: {
        mode: "wasm",
        validateResponseFormat: async (_definition, response) => {
          seen.push(response.value);
          if (response.value === 9) return firstValidation;
          return { violations: [] };
        },
      },
      onEscape: () => undefined,
      onSubmit: async () => {
        submitCalls += 1;
        return { kind: "accepted" };
      },
    }),
  );

  const stale = controller.validate({ kind: "numeric", value: 9 });
  await controller.reset({ kind: "numeric", value: 3 });
  resolveFirst({ violations: [{ kind: "numericNotFinite" }] });
  await stale;

  assert.deepEqual(seen, [9, 3]);
  assert.equal(controller.phase().kind, "restored");
  assert.equal(controller.canSubmit(), true);
  assert.equal(submitCalls, 0);
});

test("a rejected submission keeps the response editable for a corrected resubmission", async () => {
  const submitted = [];
  const controller = createRoot(() =>
    createSubmissionController({
      attemptId: "attempt-corrected-submission",
      definition: numericDefinition,
      validator: {
        mode: "wasm",
        validateResponseFormat: async () => ({ violations: [] }),
      },
      onEscape: () => undefined,
      onSubmit: async (response) => {
        submitted.push(response);
        return submitted.length === 1
          ? { kind: "rejected", message: "The submitted response was refused (422). Correct it." }
          : { kind: "accepted" };
      },
    }),
  );
  const refused = { kind: "numeric", value: 7 };
  const corrected = { kind: "numeric", value: 8 };

  await controller.validate(refused);
  await controller.submit(refused);

  assert.equal(controller.phase().kind, "failed");
  assert.equal(controller.invalid(), true);
  assert.equal(controller.locked(), false);
  assert.equal(controller.canReset(), true);

  await controller.validate(corrected);
  assert.equal(controller.canSubmit(), true);
  await controller.submit(corrected);

  assert.equal(controller.phase().kind, "submitted");
  assert.deepEqual(submitted, [refused, corrected]);
});

test("external-tool readiness and route values admit only the narrow browser contract", () => {
  const courseId = "course-external";
  const assignmentId = "assignment-external";
  const attemptId = "attempt-external";
  assert.equal(
    isExternalToolReadyMessage({ kind: "ple.externalTool.ready", attemptId }, attemptId),
    true,
  );
  for (const message of [
    { kind: "ple.externalTool.ready", attemptId: "other-attempt" },
    { kind: "ple.externalTool.ready", attemptId, score: 1 },
    { kind: "ple.externalTool.ready", attemptId, provider: "foreign" },
    { kind: "ple.externalTool.complete", attemptId },
  ]) {
    assert.equal(isExternalToolReadyMessage(message, attemptId), false);
  }
  const origin = "https://client.example.test";
  assert.equal(
    isSafeExternalToolLaunchPath(
      "/api/courses/course-external/assignments/assignment-external/attempts/attempt-external/external-tool/launch",
      courseId,
      assignmentId,
      attemptId,
      origin,
    ),
    true,
  );
  const expected =
    "/api/courses/course-external/assignments/assignment-external/attempts/attempt-external/external-tool/launch";
  for (const unsafe of [
    `https://client.example.test${expected}`,
    `https://foreign.example${expected}`,
    `//foreign.example${expected}`,
    expected.replace(courseId, "other-course"),
    expected.replace(assignmentId, "other-assignment"),
    expected.replace(attemptId, "other-attempt"),
    `${expected}?token=secret`,
    `${expected}#fragment`,
    expected.replace("courses/course-external", "courses/../foreign"),
    expected.replace("courses/course-external", "courses/%2e%2e/foreign"),
    expected.replace("/external-tool/", "\\external-tool\\"),
  ]) {
    assert.equal(
      isSafeExternalToolLaunchPath(unsafe, courseId, assignmentId, attemptId, origin),
      false,
      unsafe,
    );
  }
});
