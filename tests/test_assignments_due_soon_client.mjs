import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeDueSoonAssessments } from "../src/api/decoders/assessment_release.ts";
import { createHttpApiClient } from "../src/api/http_client.ts";
import { createRecordingFetch } from "./http_client_test_support.mjs";

function responseBody() {
  return {
    items: [
      {
        courseInstanceId: "CI8H4N6PAW",
        courseLongName: "Molecular Biology",
        assessmentId: "A9J5V7WA3",
        assessmentType: "quiz",
        assessmentTitle: "DNA repair",
        assessmentStatus: "released",
        dueAtMillis: 1790971200125,
      },
    ],
    nextCursor: null,
    displayTimeZone: "America/Chicago",
  };
}

test("Due Soon client reads the closed Account-zone cross-Course projection", async () => {
  const { recordingFetch, requests } = createRecordingFetch(
    async () =>
      new Response(JSON.stringify(responseBody()), {
        headers: {
          "cache-control": "no-store",
          "content-type": "application/json",
        },
      }),
  );

  const result = await createHttpApiClient({ fetch: recordingFetch }).listAssessmentsDueSoon();

  assert.equal(result.displayTimeZone, "America/Chicago");
  assert.equal(result.items[0]?.courseInstanceId, "CI8H4N6PAW");
  assert.equal(result.items[0]?.dueAtMillis, 1790971200125);
  assert.equal(new URL(requests[0].url).pathname, "/api/assessments/due-soon");
  assert.equal(requests[0].method, "GET");
  assert.equal(requests[0].cache, "no-store");
});

test("Due Soon decoder rejects hidden pagination and non-instant response fields", () => {
  assert.throws(
    () => decodeDueSoonAssessments({ ...responseBody(), nextCursor: "next" }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeDueSoonAssessments({
        ...responseBody(),
        items: [{ ...responseBody().items[0], studentIdentity: "must-not-cross" }],
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeDueSoonAssessments({
        ...responseBody(),
        items: [{ ...responseBody().items[0], courseId: "CI8H4N6PAW" }],
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeDueSoonAssessments({
        ...responseBody(),
        items: [{ ...responseBody().items[0], dueAtMillis: "1790971200125" }],
      }),
    DecodeError,
  );
});
