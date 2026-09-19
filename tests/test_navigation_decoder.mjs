import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeNavigationResolution } from "../src/api/decoders/navigation.ts";

test("navigation decoder accepts the exact Assessment Attempt route resolution", () => {
  assert.deepEqual(
    decodeNavigationResolution({
      kind: "assessmentAttempt",
      courseInstanceId: "00000000-0000-4000-8000-000000000001",
      assessmentId: "00000000-0000-4000-8000-000000000002",
      studentRecordId: "00000000-0000-4000-8000-000000000003",
      assessmentAttemptId: "00000000-0000-4000-8000-000000000004",
    }),
    {
      kind: "assessmentAttempt",
      courseInstanceId: "00000000-0000-4000-8000-000000000001",
      assessmentId: "00000000-0000-4000-8000-000000000002",
      studentRecordId: "00000000-0000-4000-8000-000000000003",
      assessmentAttemptId: "00000000-0000-4000-8000-000000000004",
    },
  );
});

test("navigation decoder rejects the retired run route shape", () => {
  assert.throws(
    () =>
      decodeNavigationResolution({
        kind: "run",
        courseId: "00000000-0000-4000-8000-000000000001",
        assessmentId: "00000000-0000-4000-8000-000000000002",
        studentRecordId: "00000000-0000-4000-8000-000000000003",
        runId: "00000000-0000-4000-8000-000000000004",
      }),
    DecodeError,
  );
});
