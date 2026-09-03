import assert from "node:assert/strict";
import test from "node:test";

import {
  canonicalCourseLocalDateAndTime,
  emptyPatchDraft,
  policyRequest,
} from "../src/pages/assignment_access/model";

test("Student View Scenario modifiers preserve every explicit adjustment state", () => {
  const draft = {
    ...emptyPatchDraft(),
    dueAt: { kind: "set" as const, value: "2026-08-20T09:30" },
    attemptLimit: { kind: "unrestricted" as const, value: "" },
  };
  const request = policyRequest("extend_only", draft);
  assert.equal(request.mode, "extend_only");
  assert.equal(request.adjustment.available_at.kind, "inherit");
  assert.deepEqual(request.adjustment.attempt_limit, { kind: "unrestricted" });
  assert.deepEqual(request.adjustment.due_at, { kind: "set", value: "2026-08-20T09:30:00.000" });
});

test("Student View Scenario modifiers keep dates inherited and accept both modes", () => {
  const draft = {
    ...emptyPatchDraft(),
    assignmentAttemptTimeLimitSeconds: { kind: "set" as const, value: "180" },
    attemptLimit: { kind: "set" as const, value: "2" },
  };
  const extendOnly = policyRequest("extend_only", draft);
  const replace = policyRequest("replace", draft);
  assert.equal(extendOnly.mode, "extend_only");
  assert.equal(replace.mode, "replace");
  assert.deepEqual(extendOnly.adjustment.available_at, { kind: "inherit" });
  assert.deepEqual(extendOnly.adjustment.due_at, { kind: "inherit" });
  assert.deepEqual(extendOnly.adjustment.closes_at, { kind: "inherit" });
  assert.deepEqual(extendOnly.adjustment.assignment_attempt_time_limit_seconds, {
    kind: "set",
    value: 180,
  });
  assert.deepEqual(extendOnly.adjustment.attempt_limit, { kind: "set", value: 2 });
});

test("course-local inputs are canonical strings with no epoch conversion", () => {
  assert.equal(canonicalCourseLocalDateAndTime("2026-08-20T09:30"), "2026-08-20T09:30:00.000");
  assert.equal(canonicalCourseLocalDateAndTime("2026-08-20T09:30:45"), "2026-08-20T09:30:45.000");
  assert.equal(
    canonicalCourseLocalDateAndTime("2026-08-20T09:30:45.123"),
    "2026-08-20T09:30:45.123",
  );
  assert.throws(() => canonicalCourseLocalDateAndTime("2026-08-20T09:30:45.1"));
});
