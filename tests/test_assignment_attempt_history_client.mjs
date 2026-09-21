import assert from "node:assert/strict";
import test from "node:test";

import { ApiProtocolError, createHttpApiClient } from "../src/api/http_client.ts";
import { createRecordingFetch } from "./http_client_test_support.mjs";

function history() {
  return {
    assessmentAttemptId: "00000000-0000-0000-0000-00000000000c",
    attemptNumber: 3,
    course: {
      id: "CI7K3M2QAZ",
      shortName: "BCHM 301",
      longName: "Biochemistry 301: Proteins and Peptides",
      theme: "ocean",
    },
    assessment: { id: "A7K3M2QAS", title: "Peptide structure practice" },
    state: "submitted",
    questions: [
      {
        position: 1,
        publishedQuestionRevisionTuple: { publishedQuestionId: "7K3M-79QP", revisionNumber: 2 },
        responseState: "submitted",
      },
    ],
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

  assert.deepEqual(
    await client.getStudentAssessmentAttemptHistory("00000000-0000-0000-0000-00000000000c"),
    history(),
  );
  assert.equal(
    requests[0]?.url,
    "https://client.example.test/live/api/assessment-attempts/00000000-0000-0000-0000-00000000000c/history",
  );
  assert.equal(requests[0]?.method, "GET");
  assert.equal(requests[0]?.credentials, "same-origin");
  assert.equal(requests[0]?.cache, "no-store");
});

test("selected Attempt history rejects invalid IDs before transport", async () => {
  const client = createHttpApiClient({
    fetch: async () => {
      throw new Error("transport must not run");
    },
  });

  await assert.rejects(client.getStudentAssessmentAttemptHistory("R-0"), ApiProtocolError);
});
