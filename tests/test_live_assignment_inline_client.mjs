import assert from "node:assert/strict";
import test from "node:test";

import { createHttpApiClient } from "../src/api/http_client.ts";
import { createRecordingFetch } from "./http_client_test_support.mjs";

test("inline Assignment save sends raw local time with its row edit number", async () => {
  const { recordingFetch, requests } = createRecordingFetch(
    async () =>
      new Response(
        JSON.stringify({
          reference: "A-2",
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

  const saved = await createHttpApiClient({ fetch: recordingFetch }).saveLiveAssignmentInline(
    "C-1",
    "A-2",
    { title: "Edited peptide bonds", dueAt: "2026-09-11T14:30:00.125" },
    "3",
  );

  assert.equal(saved.editNumber, "4");
  assert.equal(
    new URL(requests[0].url).pathname,
    "/api/course-instances/C-1/assignments/A-2/inline",
  );
  assert.equal(requests[0].method, "PUT");
  assert.equal(requests[0].headers.get("if-match"), '"3"');
  assert.equal(
    await requests[0].text(),
    JSON.stringify({ title: "Edited peptide bonds", dueAt: "2026-09-11T14:30:00.125" }),
  );
  assert.equal(requests[0].cache, "no-store");
});
