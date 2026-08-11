import assert from "node:assert/strict";
import test from "node:test";

import {
  classifyFailureHint,
  parseFailureTriageInput,
} from "./playwright/simulator/failure_triage.ts";

const CASES = [
  {
    input: { stage: "runner-preflight", diagnostic: "configuration-invalid" },
    category: "configuration",
  },
  {
    input: { stage: "gateway-readiness", diagnostic: "gateway-unavailable" },
    category: "gateway",
  },
  {
    input: { stage: "visible-target", diagnostic: "visible-control-unavailable" },
    category: "selector",
  },
  {
    input: { stage: "keyboard-navigation", diagnostic: "keyboard-target-unavailable" },
    category: "keyboard",
  },
  {
    input: { stage: "visible-outcome", diagnostic: "visible-state-unavailable" },
    category: "visible-outcome-mismatch",
  },
];

test("fixed walkthrough failure inputs receive their advisory category", () => {
  for (const testCase of CASES) {
    assert.deepEqual(classifyFailureHint(testCase.input), { category: testCase.category });
  }
});

test("mismatched or unbounded values are advisory-unclassified", () => {
  assert.deepEqual(
    classifyFailureHint({ stage: "gateway-readiness", diagnostic: "visible-state-unavailable" }),
    { category: "unclassified" },
  );
  assert.deepEqual(
    classifyFailureHint({ stage: "gateway-readiness", diagnostic: "https://127.0.0.1/private" }),
    { category: "unclassified" },
  );
});

test("input validation rejects extra raw detail and never returns it", () => {
  const rawDetail = {
    stage: "keyboard-navigation",
    diagnostic: "keyboard-target-unavailable",
    selector: "[data-private='answer']",
  };
  assert.equal(parseFailureTriageInput(rawDetail), undefined);
  assert.deepEqual(classifyFailureHint(rawDetail), { category: "unclassified" });
});

test("input validation rejects non-enumerable and symbol raw detail", () => {
  const nonEnumerable = {
    stage: "keyboard-navigation",
    diagnostic: "keyboard-target-unavailable",
  };
  Object.defineProperty(nonEnumerable, "detail", { value: "hidden", enumerable: false });
  const symbolDetail = {
    stage: "keyboard-navigation",
    diagnostic: "keyboard-target-unavailable",
    [Symbol("detail")]: "hidden",
  };
  assert.equal(parseFailureTriageInput(nonEnumerable), undefined);
  assert.deepEqual(classifyFailureHint(symbolDetail), { category: "unclassified" });
});

test("input validation rejects a hidden required field", () => {
  const hiddenStage = { diagnostic: "keyboard-target-unavailable" };
  Object.defineProperty(hiddenStage, "stage", {
    value: "keyboard-navigation",
    enumerable: false,
  });
  assert.equal(parseFailureTriageInput(hiddenStage), undefined);
  assert.deepEqual(classifyFailureHint(hiddenStage), { category: "unclassified" });
});
