import assert from "node:assert/strict";
import test from "node:test";

import { createHttpApiClient } from "../src/api/http_client.ts";
import { createRecordingFetch } from "./http_client_test_support.mjs";

test("inline Assessment save sends raw local time with its row edit number", async () => {
  const { recordingFetch, requests } = createRecordingFetch(
    async () =>
      new Response(
        JSON.stringify({
          reference: "A8H4N6P",
          assessmentType: "exam",
          title: "Edited peptide bonds",
          dueAt: "2026-09-11T14:30:00.125",
          displayTimeZone: "America/Chicago",
          status: "released",
          editNumber: "4",
        }),
        {
          headers: {
            "cache-control": "no-store",
            "content-type": "application/json",
            etag: '"4"',
          },
        },
      ),
  );

  const saved = await createHttpApiClient({ fetch: recordingFetch }).saveLiveAssessmentInline(
    "CI7K3M2Q",
    "A8H4N6P",
    { title: "Edited peptide bonds", dueAt: "2026-09-11T14:30:00.125" },
    "3",
  );

  assert.equal(saved.editNumber, "4");
  assert.equal(saved.assessmentType, "exam");
  assert.equal(
    new URL(requests[0].url).pathname,
    "/api/course-instances/CI7K3M2Q/assessments/A8H4N6P/inline",
  );
  assert.equal(requests[0].method, "PUT");
  assert.equal(requests[0].headers.get("if-match"), '"3"');
  assert.equal(
    await requests[0].text(),
    JSON.stringify({ title: "Edited peptide bonds", dueAt: "2026-09-11T14:30:00.125" }),
  );
  assert.equal(requests[0].cache, "no-store");
});
