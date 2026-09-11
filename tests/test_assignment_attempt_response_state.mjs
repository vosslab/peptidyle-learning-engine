// test_assignment_attempt_response_state.mjs - exact-position save eligibility regressions.

import assert from "node:assert/strict";
import test from "node:test";

import { AssignmentAttemptResponseState } from "../src/pages/assignment_attempt_response_state.ts";

test("a delayed Question validation cannot make another Question response saveable", () => {
  const state = new AssignmentAttemptResponseState();
  const questionOne = { kind: "shortText", text: "first" };
  const questionTwo = { kind: "shortText", text: "second" };

  const questionOneRevision = state.edit(1, questionOne);
  assert.equal(state.current(1)?.valid, false);
  state.clear();
  const questionTwoRevision = state.edit(2, questionTwo);

  assert.equal(state.validate(1, questionOne, questionOneRevision, true), false);
  assert.equal(state.current(2)?.response, questionTwo);
  assert.equal(state.current(2)?.valid, false);
  assert.equal(state.validate(2, questionTwo, questionTwoRevision, true), true);
  assert.equal(state.current(2)?.valid, true);
});

test("a rejected format retains raw input but keeps it ineligible for save", () => {
  const state = new AssignmentAttemptResponseState();
  const rawBlankNumeric = { kind: "numeric", value: Number.NaN };
  const revision = state.edit(1, rawBlankNumeric);

  assert.equal(state.validate(1, rawBlankNumeric, revision, false), true);
  const current = state.current(1);
  assert.equal(current?.response, rawBlankNumeric);
  assert.equal(current?.valid, false);
});
