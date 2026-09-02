// Durable untrusted-JSON checks for the Student Response Format Check boundary.

import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeStudentResponseFormatCheck } from "../src/api/decoders/student_response_format_check.ts";

test("Student Response Format Check accepts every current Issue shape", () => {
  const check = decodeStudentResponseFormatCheck({
    issues: [
      { kind: "responseKindMismatch" },
      { kind: "numericNotFinite" },
      { kind: "selectionCount", expected: { kind: "exactly", count: 2 }, actual: 1 },
      { kind: "duplicateChoice", choice: "choice-a" },
      { kind: "unknownChoice", choice: "choice-b" },
      { kind: "textTooLong", maxLength: 10, actualLength: 11 },
      { kind: "blankSlotsMismatch" },
      { kind: "matchingPromptsMismatch" },
      { kind: "duplicateMatchChoice", choice: "choice-c" },
      { kind: "unknownMatchChoice", choice: "choice-d" },
      { kind: "orderingItemsMismatch" },
      { kind: "duplicateHotspotRegion", region: "region-a" },
      { kind: "unknownHotspotRegion", region: "region-b" },
    ],
  });

  assert.deepEqual(check.issues[2], {
    kind: "selectionCount",
    expected: { kind: "exactly", count: 2 },
    actual: 1,
  });
  assert.deepEqual(check.issues.at(-1), {
    kind: "unknownHotspotRegion",
    region: "region-b",
  });
});

test("Student Response Format Check rejects retired, extra, and unsafe JSON shapes", () => {
  for (const value of [
    { issues: [{ kind: "retiredResponseReference" }] },
    { violations: [] },
    { issues: [], violations: [] },
    { issues: [{ kind: "numericNotFinite", unexpected: true }] },
    { issues: [{ kind: "selectionCount", expected: { kind: "exactlyOne" }, actual: -1 }] },
    {
      issues: [
        {
          kind: "textTooLong",
          maxLength: Number.MAX_SAFE_INTEGER + 1,
          actualLength: 1,
        },
      ],
    },
    { issues: [], ignored: true },
  ]) {
    assert.throws(() => decodeStudentResponseFormatCheck(value), DecodeError);
  }
});
