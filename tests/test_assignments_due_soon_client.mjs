import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeDueSoonAssignments } from "../src/api/decoders/assignment_release.ts";
import { createHttpApiClient } from "../src/api/http_client.ts";
import { createRecordingFetch } from "./http_client_test_support.mjs";

function responseBody() {
  return {
    items: [
      {
        courseReference: "C-2",
        courseTitle: "Molecular Biology",
        assignmentReference: "A-3",
        assignmentTitle: "DNA repair",
        assignmentStatus: "released",
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

  const result = await createHttpApiClient({ fetch: recordingFetch }).listAssignmentsDueSoon();

  assert.equal(result.displayTimeZone, "America/Chicago");
  assert.equal(result.items[0]?.courseReference, "C-2");
  assert.equal(result.items[0]?.dueAtMillis, 1790971200125);
  assert.equal(new URL(requests[0].url).pathname, "/api/assignments/due-soon");
  assert.equal(requests[0].method, "GET");
  assert.equal(requests[0].cache, "no-store");
});

test("Due Soon decoder rejects hidden pagination and non-instant response fields", () => {
  assert.throws(
    () => decodeDueSoonAssignments({ ...responseBody(), nextCursor: "next" }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeDueSoonAssignments({
        ...responseBody(),
        items: [{ ...responseBody().items[0], studentIdentity: "must-not-cross" }],
      }),
    DecodeError,
  );
  assert.throws(
    () =>
      decodeDueSoonAssignments({
        ...responseBody(),
        items: [{ ...responseBody().items[0], dueAtMillis: "1790971200125" }],
      }),
    DecodeError,
  );
});
