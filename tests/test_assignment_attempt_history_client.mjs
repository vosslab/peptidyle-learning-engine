import assert from "node:assert/strict";
import test from "node:test";

import { ApiProtocolError, createHttpApiClient } from "../src/api/http_client.ts";
import { createRecordingFetch } from "./http_client_test_support.mjs";

function history() {
  return {
    assignmentAttempt: "R-12",
    attemptNumber: 3,
    course: { reference: "C-2", title: "Biochemistry", theme: "ocean" },
    assignment: { reference: "A-4", title: "Peptide structure practice" },
    state: "submitted",
    questions: [{ position: 1, responseState: "submitted" }],
  };
}

test("selected Attempt history uses the exact same-origin no-store reader and strict decoder", async () => {
  const { recordingFetch, requests } = createRecordingFetch(
    async () =>
      new Response(JSON.stringify(history()), {
        headers: { "cache-control": "no-store", "content-type": "application/json" },
      }),
  );
  const client = createHttpApiClient({ fetch: recordingFetch, basePath: "/live" });

  assert.deepEqual(await client.getStudentAssignmentAttemptHistory("R-12"), history());
  assert.equal(
    requests[0]?.url,
    "https://client.example.test/live/api/assignment-attempts/R-12/history",
  );
  assert.equal(requests[0]?.method, "GET");
  assert.equal(requests[0]?.credentials, "same-origin");
  assert.equal(requests[0]?.cache, "no-store");
});

test("selected Attempt history rejects invalid references before transport", async () => {
  const client = createHttpApiClient({
    fetch: async () => {
      throw new Error("transport must not run");
    },
  });

  await assert.rejects(client.getStudentAssignmentAttemptHistory("R-0"), ApiProtocolError);
});
