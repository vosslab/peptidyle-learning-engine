import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import {
  decodeDerivedPreviewSubjectRequest,
  decodeInstructorPreviewSchedulePage,
  decodePreviewPlaneResponse,
  decodeSyntheticPreviewSubjectRequest,
} from "../src/api/decoders.ts";
import {
  ApiProtocolError,
  PreviewPlaneConflictError,
  createHttpApiClient,
} from "../src/api/http_client.ts";

const course = "C-12";
const assignment = "A-34";
const revision = "7";
const selectedMoment = { value: "2026-08-25T09:00:00.000", timeZone: "America/Chicago" };
const inheritPatch = {
  availableAt: { kind: "inherit" },
  dueAt: { kind: "inherit" },
  closesAt: { kind: "inherit" },
  timeLimitSeconds: { kind: "inherit" },
  attemptLimit: { kind: "inherit" },
};

function jsonResponse(value, status = 200, cacheControl = "no-store") {
  return new Response(JSON.stringify(value), {
    status,
    headers: { "content-type": "application/json", "cache-control": cacheControl },
  });
}

function schedule(source = "base") {
  return {
    availableAt: { value: "2026-08-24T09:00:00.000", source },
    dueAt: { value: "2026-08-26T09:00:00.000", source },
    closesAt: { value: "2026-08-27T09:00:00.000", source },
    timeLimitSeconds: { value: 3600, source },
    attemptLimit: { value: 2, source },
    lateSubmission: { value: "accept", source },
    deadlineBehavior: { value: "autoSubmit", source },
  };
}

function allowedEvaluation() {
  return {
    kind: "allowed",
    subject: {
      kind: "derived",
      assignment,
      revision,
      selectedMoment,
      policy: schedule("accommodation"),
      priorRunCount: 0,
    },
    entitlement: "activeStudentCourseMembership",
    schedule: schedule("accommodation"),
    disclosure: [
      {
        kind: "available",
        moment: "now",
        flags: {
          scoreShown: false,
          correctnessShown: false,
          feedbackShown: false,
          solutionShown: false,
          statisticsShown: false,
        },
      },
      {
        kind: "available",
        moment: "due",
        flags: {
          scoreShown: true,
          correctnessShown: true,
          feedbackShown: true,
          solutionShown: false,
          statisticsShown: false,
        },
      },
      { kind: "unavailable", moment: "close", reason: "boundaryMissing" },
    ],
  };
}

function previewResponse() {
  return {
    evaluation: allowedEvaluation(),
    accommodation: { before: schedule("base"), after: schedule("accommodation") },
  };
}

test("preview decoders accept the closed server projections", () => {
  const response = previewResponse();
  const decoded = decodePreviewPlaneResponse(response);
  assert.deepEqual(decoded, response);

  const schedulePage = {
    revision,
    rows: [
      {
        kind: "granted",
        membership: "M-9",
        display: "Mary Student",
        entitlement: "activeStudentCourseMembership",
        schedule: schedule(),
      },
      { kind: "denied", membership: "M-10", display: "Jack Student", reason: "notEntitled" },
    ],
    nextCursor: "next cursor",
  };
  assert.deepEqual(decodeInstructorPreviewSchedulePage(schedulePage), schedulePage);

  const syntheticRequest = {
    assignment,
    revision,
    selectedMoment,
    modifiers: { mode: "extendOnly", patch: inheritPatch },
  };
  assert.deepEqual(decodeSyntheticPreviewSubjectRequest(syntheticRequest), syntheticRequest);
  assert.deepEqual(
    decodeDerivedPreviewSubjectRequest({ assignment, revision, selectedMoment, membership: "M-9" }),
    { assignment, revision, selectedMoment, membership: "M-9" },
  );
});

test("preview decoders reject unknown and protected fields at closed boundaries", () => {
  const denial = { evaluation: { kind: "denied", reason: "notEntitled" }, accommodation: null };
  assert.deepEqual(decodePreviewPlaneResponse(denial), denial);
  assert.throws(
    () =>
      decodePreviewPlaneResponse({
        evaluation: { kind: "denied", reason: "notEntitled", subject: allowedEvaluation().subject },
        accommodation: null,
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodePreviewPlaneResponse({
        evaluation: { kind: "denied", reason: "notEntitled" },
        accommodation: { before: schedule(), after: schedule() },
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodePreviewPlaneResponse({
        ...previewResponse(),
        evaluation: {
          ...allowedEvaluation(),
          subject: { ...allowedEvaluation().subject, membership: "M-9" },
        },
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeInstructorPreviewSchedulePage({
        revision,
        rows: [
          {
            kind: "denied",
            membership: "M-9",
            display: "Mary",
            reason: "notEntitled",
            schedule: schedule(),
          },
        ],
        nextCursor: null,
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodePreviewPlaneResponse({
        ...previewResponse(),
        evaluation: {
          ...allowedEvaluation(),
          disclosure: [
            {
              ...allowedEvaluation().disclosure[0],
              flags: { ...allowedEvaluation().disclosure[0].flags, answer: "secret" },
            },
            ...allowedEvaluation().disclosure.slice(1),
          ],
        },
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeSyntheticPreviewSubjectRequest({
        assignment,
        revision,
        selectedMoment,
        unexpectedGroups: ["G-3"],
        modifiers: { mode: "extendOnly", patch: inheritPatch },
      }),
    DecodeError,
  );
});

test("preview transport uses the public C-/A- routes, headers, bodies, and canonical pagination", async () => {
  const requests = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      requests.push({ path: String(input), init });
      return String(input).includes("preview-schedule")
        ? jsonResponse({ revision, rows: [], nextCursor: null })
        : jsonResponse(previewResponse());
    },
  });

  await client.listPreviewSchedule(course, assignment, revision, "opaque +/", 25);
  await client.constructSyntheticPreview(course, assignment, revision, {
    selectedMoment,
    modifiers: { mode: "extendOnly", patch: inheritPatch },
  });
  await client.constructDerivedPreview(course, assignment, revision, {
    selectedMoment,
    membership: "M-9",
  });

  assert.equal(
    requests[0].path,
    "/api/courses/C-12/assignments/A-34/preview-schedule?after=opaque+%2B%2F&size=25",
  );
  for (const request of requests) {
    assert.equal(request.init.credentials, "same-origin");
    assert.equal(request.init.cache, "no-store");
    assert.equal(request.init.headers["if-match"], '"7"');
    assert.equal(request.init.headers.accept, "application/json");
  }
  assert.equal(requests[0].init.method, "GET");
  assert.equal(requests[1].init.method, "POST");
  assert.equal(requests[1].init.headers["content-type"], "application/json");
  assert.deepEqual(JSON.parse(requests[1].init.body), {
    selectedMoment,
    modifiers: { mode: "extendOnly", patch: inheritPatch },
  });
  assert.deepEqual(JSON.parse(requests[2].init.body), { selectedMoment, membership: "M-9" });
});

test("preview transport maps stale revisions and cache violations safely", async () => {
  const staleClient = createHttpApiClient({
    fetch: async () => jsonResponse({ error: "assignment changed; reload it" }, 412),
  });
  await assert.rejects(
    staleClient.constructDerivedPreview(course, assignment, revision, {
      selectedMoment,
      membership: "M-9",
    }),
    PreviewPlaneConflictError,
  );

  const cacheableClient = createHttpApiClient({
    fetch: async () =>
      jsonResponse({ revision, rows: [], nextCursor: null }, 200, "private, max-age=1"),
  });
  await assert.rejects(
    cacheableClient.listPreviewSchedule(course, assignment, revision),
    ApiProtocolError,
  );
});
