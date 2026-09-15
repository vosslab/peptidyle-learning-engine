import assert from "node:assert/strict";
import test from "node:test";

import {
  canonicalLocalDateAndTime,
  optionalPositiveIntegerDraft,
} from "../src/pages/assessment_workspace/assessment_workspace_policy_model.ts";

test("policy local-time normalization preserves valid native time precision", () => {
  assert.equal(canonicalLocalDateAndTime("2026-09-01T17:00"), "2026-09-01T17:00:00.000");
  assert.equal(canonicalLocalDateAndTime("2026-09-01T17:00:15"), "2026-09-01T17:00:15.000");
  assert.equal(canonicalLocalDateAndTime("2026-09-01T17:00:15.1"), "2026-09-01T17:00:15.100");
  assert.equal(canonicalLocalDateAndTime("2026-09-01T17:00:15.12"), "2026-09-01T17:00:15.120");
  assert.equal(canonicalLocalDateAndTime("2026-09-01T17:00:15.123"), "2026-09-01T17:00:15.123");
  assert.equal(canonicalLocalDateAndTime("2026/09/01 17:00"), null);
});

test("numeric policy drafts preserve invalid text and provide no stale payload value", () => {
  assert.deepEqual(optionalPositiveIntegerDraft(""), { raw: "", value: null, valid: true });
  assert.deepEqual(optionalPositiveIntegerDraft("12"), { raw: "12", value: 12, valid: true });
  assert.deepEqual(optionalPositiveIntegerDraft("0"), { raw: "0", value: null, valid: false });
  assert.deepEqual(optionalPositiveIntegerDraft("2147483648"), {
    raw: "2147483648",
    value: null,
    valid: false,
  });
});
