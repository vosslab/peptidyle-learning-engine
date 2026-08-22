import assert from "node:assert/strict";
import test from "node:test";

import {
  adoptReloadedRevision,
  canonicalCourseLocalDateTime,
  emptyPatchDraft,
  policyRequest,
  scheduleOffsetRequest,
  sourceLabel,
  startLabel,
} from "../src/pages/assignment_access/model";

test("access policy writes preserve every explicit patch state", () => {
  const draft = {
    ...emptyPatchDraft(),
    dueAt: { kind: "set" as const, value: "2026-08-20T09:30" },
    attemptLimit: { kind: "unrestricted" as const, value: "" },
  };
  const request = policyRequest("extendOnly", draft);
  assert.equal(request.mode, "extendOnly");
  assert.equal(request.patch.availableAt.kind, "inherit");
  assert.deepEqual(request.patch.attemptLimit, { kind: "unrestricted" });
  assert.deepEqual(request.patch.dueAt, { kind: "set", value: "2026-08-20T09:30:00.000" });
});

test("synthetic accommodation requests keep dates inherited and accept both modes", () => {
  const draft = {
    ...emptyPatchDraft(),
    timeLimitSeconds: { kind: "set" as const, value: "180" },
    attemptLimit: { kind: "set" as const, value: "2" },
  };
  const extendOnly = policyRequest("extendOnly", draft);
  const override = policyRequest("override", draft);
  assert.equal(extendOnly.mode, "extendOnly");
  assert.equal(override.mode, "override");
  assert.deepEqual(extendOnly.patch.availableAt, { kind: "inherit" });
  assert.deepEqual(extendOnly.patch.dueAt, { kind: "inherit" });
  assert.deepEqual(extendOnly.patch.closesAt, { kind: "inherit" });
  assert.deepEqual(extendOnly.patch.timeLimitSeconds, { kind: "set", value: 180 });
  assert.deepEqual(extendOnly.patch.attemptLimit, { kind: "set", value: 2 });
});

test("course-local inputs are canonical strings with no epoch conversion", () => {
  assert.equal(canonicalCourseLocalDateTime("2026-08-20T09:30"), "2026-08-20T09:30:00.000");
  assert.equal(canonicalCourseLocalDateTime("2026-08-20T09:30:45"), "2026-08-20T09:30:45.000");
  assert.equal(canonicalCourseLocalDateTime("2026-08-20T09:30:45.123"), "2026-08-20T09:30:45.123");
  assert.throws(() => canonicalCourseLocalDateTime("2026-08-20T09:30:45.1"));
});

test("reloading a revision preserves the caller-owned modifier draft", () => {
  const draft = { dueAt: { kind: "set", value: "2026-08-20T09:30:00.000" } };
  const reloaded = adoptReloadedRevision("42", draft);
  assert.equal(reloaded.revision, "42");
  assert.equal(reloaded.draft, draft);
});

test("schedule offsets reject zero and accept signed seconds", () => {
  assert.throws(() => scheduleOffsetRequest("0"), /nonzero/u);
  assert.deepEqual(scheduleOffsetRequest("-900"), { offsetSeconds: -900 });
});

test("safe preview copy uses only display labels and closed verdict copy", () => {
  assert.equal(
    sourceLabel({ kind: "membership", membership: "M-private", label: "Jordan Lee" }),
    "Jordan Lee",
  );
  assert.equal(startLabel("dueDateRejectsNewRun"), "Due date prevents a new run");
});
