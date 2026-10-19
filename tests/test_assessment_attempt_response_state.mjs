import assert from "node:assert/strict";
import test from "node:test";

import { AssessmentAttemptResponseState } from "../src/pages/assessment_attempt/assessment_attempt_response_state.ts";

test("a delayed Question validation cannot make another Question response saveable", () => {
  const state = new AssessmentAttemptResponseState();
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
  const state = new AssessmentAttemptResponseState();
  const rawBlankNumeric = { kind: "numeric", value: Number.NaN };
  const revision = state.edit(1, rawBlankNumeric);

  assert.equal(state.validate(1, rawBlankNumeric, revision, false), true);
  assert.equal(state.current(1)?.response, rawBlankNumeric);
  assert.equal(state.current(1)?.valid, false);
});
