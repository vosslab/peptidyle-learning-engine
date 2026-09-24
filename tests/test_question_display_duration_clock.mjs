import assert from "node:assert/strict";
import test from "node:test";

import { QuestionDisplayDurationClock } from "../src/question_display_duration_clock.ts";

test("display clock counts only visible intervals and resumes from saved cumulative time", () => {
  const clock = new QuestionDisplayDurationClock(1_200);
  clock.resume(100);
  assert.equal(clock.snapshot(2_000), 3_100);
  assert.equal(clock.pause(2_000), 3_100);
  assert.equal(clock.snapshot(9_000), 3_100);
  clock.resume(10_000);
  assert.equal(clock.checkpoint(10_750), 3_850);
  assert.equal(clock.pause(11_000), 4_100);
});

test("stored checkpoints never reduce the local cumulative total", () => {
  const clock = new QuestionDisplayDurationClock(null);
  clock.resume(0);
  assert.equal(clock.checkpoint(500), 500);
  clock.observeStoredMilliseconds(400);
  assert.equal(clock.snapshot(700), 700);
  clock.observeStoredMilliseconds(900);
  assert.equal(clock.snapshot(900), 1_300);
});

test("display clock rejects invalid or unsafe cumulative duration", () => {
  assert.throws(() => new QuestionDisplayDurationClock(-1), RangeError);
  assert.throws(() => new QuestionDisplayDurationClock(Number.MAX_SAFE_INTEGER + 1), RangeError);
  assert.throws(() => new QuestionDisplayDurationClock(0).resume(Number.NaN), RangeError);
});
