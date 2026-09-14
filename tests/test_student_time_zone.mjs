import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { createHttpApiClient } from "../src/api/http_client.ts";
import { decodeStudentTimeZoneProfile } from "../src/api/decoders/live_student_course_landing.ts";
import { formatAssignmentDeliveryTime } from "../src/components/student_assignment_presentation.tsx";
import { applyStudentDisplayTimeZone } from "../src/pages/student_time_zone_model.ts";

const DUE_AT = Date.parse("2026-01-15T18:30:00Z");

function assignment(timeZone) {
  return {
    reference: "A-7",
    title: "Peptide practice",
    decision: {
      availableAt: DUE_AT - 3_600_000,
      dueAt: DUE_AT,
      closesAt: DUE_AT + 3_600_000,
      timeLimitSeconds: 900,
      attemptLimit: 2,
      lateWorkRule: "reject",
      displayTimeZone: timeZone,
      evaluatedAt: DUE_AT - 7_200_000,
      startDecision: "may_start",
      publicReason: null,
    },
    assignmentAttemptNumber: null,
    assignmentAttemptCompletion: null,
    gradedQuestionCount: 0,
    questionCount: 4,
  };
}

test("Student time zone uses one self-only no-store GET and PUT contract", async () => {
  const requests = [];
  const client = createHttpApiClient({
    basePath: "/live",
    fetch: async (input, init) => {
      requests.push({ input, init });
      const timeZone = init?.method === "PUT" ? "America/Los_Angeles" : "America/New_York";
      return new Response(JSON.stringify({ timeZone }), {
        headers: { "content-type": "application/json", "cache-control": "no-store" },
      });
    },
  });

  assert.deepEqual(await client.getStudentTimeZoneProfile(), {
    timeZone: "America/New_York",
  });
  assert.deepEqual(await client.updateStudentTimeZone({ timeZone: "America/Los_Angeles" }), {
    timeZone: "America/Los_Angeles",
  });
  assert.equal(requests[0].input, "/live/api/student/profile/time-zone");
  assert.equal(requests[0].init.method, "GET");
  assert.equal(requests[1].input, "/live/api/student/profile/time-zone");
  assert.equal(requests[1].init.method, "PUT");
  assert.equal(requests[1].init.body, '{"timeZone":"America/Los_Angeles"}');
  assert.equal(requests[1].init.credentials, "same-origin");
  assert.equal(requests[1].init.cache, "no-store");
});

test("Student time zone rejects unsupported zones and surplus response fields", async () => {
  assert.throws(
    () => decodeStudentTimeZoneProfile({ timeZone: "America/Chicago", accountId: "private" }),
    DecodeError,
  );
  const client = createHttpApiClient({
    fetch: async () =>
      new Response(JSON.stringify({ timeZone: "not/a-zone" }), {
        headers: { "content-type": "application/json", "cache-control": "no-store" },
      }),
  });
  await assert.rejects(client.getStudentTimeZoneProfile(), DecodeError);
  assert.throws(() => client.updateStudentTimeZone({ timeZone: " America/Chicago" }), DecodeError);
});

test("a confirmed Student zone change re-renders the same instant differently", () => {
  const assignments = [assignment("America/New_York")];
  const before = formatAssignmentDeliveryTime(
    assignments[0].decision.dueAt,
    assignments[0].decision.displayTimeZone,
  );
  const updated = applyStudentDisplayTimeZone(assignments, "America/Los_Angeles");
  const after = formatAssignmentDeliveryTime(
    updated[0].decision.dueAt,
    updated[0].decision.displayTimeZone,
  );

  assert.notEqual(after, before);
  assert.equal(updated[0].decision.dueAt, DUE_AT);
  assert.equal(updated[0].decision.startDecision, "may_start");
});
