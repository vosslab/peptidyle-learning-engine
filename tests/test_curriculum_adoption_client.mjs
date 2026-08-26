import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import {
  decodeForkAlphaPreviewView,
  decodeCourseTermShiftPreviewOutcome,
  decodeCurriculumCourseImportView,
} from "../src/api/decoders/curriculum_adoption.ts";
import { ApiProtocolError } from "../src/api/http_client/error.ts";
import { createHttpApiClient } from "../src/api/http_client.ts";

const forkPreview = {
  source: { reference: "AC-3", revision: "2" },
  resultingAlphaTitle: "Independent Alpha",
  replacements: [],
  pinCorrection: null,
};

const importInspection = {
  witness: {
    course: "C-7",
    scheduleRevision: "3",
    assignmentRevisions: [
      { assignment: "A-4", revision: "5" },
      { assignment: "A-5", revision: "6" },
    ],
  },
  origin: { kind: "ordinary" },
  term: { startDate: "2026-01-01", endDate: "2026-12-31", timeZone: "America/Chicago" },
  assignments: [
    {
      assignment: "A-4",
      title: "Protein Structure Practice",
      source: {
        kind: "reusable",
        definition: { kind: "blueprint", reference: "BP-3", revision: "2" },
      },
      revision: "4",
      reusableMeaningMatchesBaseline: true,
    },
  ],
};

function noStoreJson(value) {
  return new Response(JSON.stringify(value), {
    status: 200,
    headers: { "cache-control": "no-store", "content-type": "application/json" },
  });
}

test("B2 decoders refuse unknown fields and preserve closed recovery unions", () => {
  assert.deepEqual(decodeForkAlphaPreviewView(forkPreview).source, {
    reference: "AC-3",
    revision: "2",
  });

  const hostile = structuredClone(forkPreview);
  hostile.source.answer = "must not cross the browser boundary";
  assert.throws(() => decodeForkAlphaPreviewView(hostile), DecodeError);

  assert.throws(
    () =>
      decodeCourseTermShiftPreviewOutcome({
        kind: "ineligible",
        course: "C-4",
        reason: "issuedWork",
        recovery: "rolloverCourse",
        internalId: "private",
      }),
    DecodeError,
  );

  assert.deepEqual(decodeCurriculumCourseImportView(importInspection).witness, {
    course: "C-7",
    scheduleRevision: "3",
    assignmentRevisions: [
      { assignment: "A-4", revision: "5" },
      { assignment: "A-5", revision: "6" },
    ],
  });
  assert.equal(
    decodeCurriculumCourseImportView(importInspection).assignments[0].title,
    "Protein Structure Practice",
  );
  assert.throws(
    () => decodeCurriculumCourseImportView({ ...importInspection, course: "C-7" }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeCurriculumCourseImportView({
        ...importInspection,
        assignments: [{ ...importInspection.assignments[0], assignment: "A-9" }],
      }),
    DecodeError,
  );
});

test("B2 client binds apply to the checked preview and uses no-store same-origin transport", async () => {
  const requests = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(input.toString(), "https://ple.example"), init);
      requests.push({ request, body: request.body === null ? null : await request.text() });
      const path = new URL(request.url).pathname;
      if (path.endsWith("/api/alpha-courses/AC-3/fork/preview")) return noStoreJson(forkPreview);
      if (path.endsWith("/api/alpha-courses/AC-3/fork/apply"))
        return noStoreJson({
          source: forkPreview.source,
          alpha: "AC-4",
          replay: "applied",
          receipt: { idempotencyKey: "fork-2026" },
        });
      return noStoreJson({});
    },
  });

  const preview = await client.previewForkAlpha({
    source: forkPreview.source,
    replacements: [],
  });
  const result = await client.applyForkAlpha(preview, "fork-2026");

  assert.equal(result.alpha, "AC-4");
  const applyRequest = requests.find(({ request }) => request.url.endsWith("/fork/apply"));
  assert.ok(applyRequest);
  assert.equal(applyRequest.request.cache, "no-store");
  assert.equal(applyRequest.request.credentials, "same-origin");
  assert.deepEqual(JSON.parse(applyRequest.body), {
    preview: forkPreview,
    idempotencyKey: "fork-2026",
  });

  const hostile = structuredClone(preview);
  hostile.privateAnswer = "no";
  await assert.rejects(client.applyForkAlpha(hostile, "fork-2026"), DecodeError);

  await assert.rejects(
    client.applyCourseTermShift(
      {
        kind: "ineligible",
        course: "C-3",
        reason: "issuedWork",
        recovery: "rolloverCourse",
      },
      "term-shift-2026",
    ),
    ApiProtocolError,
  );
  assert.equal(requests.length, 2);
});

test("B2 inspection binds its canonical witness to the requested course", async () => {
  const requests = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(input.toString(), "https://ple.example"), init);
      requests.push(request);
      return noStoreJson(importInspection);
    },
  });

  const inspection = await client.inspectCurriculumImports("C-7");
  assert.equal(inspection.witness.course, "C-7");
  assert.equal(requests[0].url, "https://ple.example/api/courses/C-7/curriculum-imports");

  const mismatched = structuredClone(importInspection);
  mismatched.witness.course = "C-8";
  const mismatchedClient = createHttpApiClient({
    fetch: async () => noStoreJson(mismatched),
  });
  await assert.rejects(mismatchedClient.inspectCurriculumImports("C-7"), ApiProtocolError);
});
