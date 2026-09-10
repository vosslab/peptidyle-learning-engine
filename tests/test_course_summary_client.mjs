// Browser HTTP-client contract for one authorized Course Summary read.

import assert from "node:assert/strict";
import test from "node:test";

import { createHttpApiClient } from "../src/api/http_client.ts";
import { createRecordingFetch } from "./http_client_test_support.mjs";

const COURSE_ID = "00000000-0000-0000-0000-000000000001";

function courseSummary() {
  return {
    id: COURSE_ID,
    reference: "C-1",
    title: "Molecular Biology",
    term: { startDate: "2026-01-01", endDate: "2026-05-01", timeZone: "America/Chicago" },
    role: "student",
  };
}

test("Course Summary client requests the exact no-store member reader", async () => {
  const { recordingFetch, requests } = createRecordingFetch(
    async () =>
      new Response(JSON.stringify(courseSummary()), {
        headers: { "content-type": "application/json; charset=utf-8" },
      }),
  );
  const client = createHttpApiClient({ fetch: recordingFetch, basePath: "/live" });

  assert.deepEqual(await client.getCourse(COURSE_ID), courseSummary());
  assert.equal(requests[0].url, `https://client.example.test/live/api/courses/${COURSE_ID}`);
  assert.equal(requests[0].cache, "no-store");
  assert.equal(requests[0].credentials, "same-origin");
});
