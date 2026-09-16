// Browser HTTP-client contract for one authorized Course Instance route summary read.

import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { createHttpApiClient } from "../src/api/http_client.ts";
import { ApiProtocolError } from "../src/api/http_client/error.ts";
import { createRecordingFetch } from "./http_client_test_support.mjs";

const COURSE_REFERENCE = "CI7K3M2Q";

function courseSummary() {
  return {
    reference: COURSE_REFERENCE,
    shortName: "BIOL 301",
    longName: "Molecular Biology 301: Gene Expression",
    term: { startDate: "2026-01-01", endDate: "2026-05-01" },
    role: "student",
    classification: {
      disciplineUuid: "00000000-0000-4000-8000-000000000001",
      subjectUuid: null,
      topicUuid: null,
      subtopicUuid: null,
      tags: ["Molecular biology"],
    },
  };
}

test("Course Instance route summary client requests the exact no-store member reader", async () => {
  const { recordingFetch, requests } = createRecordingFetch(
    async () =>
      new Response(JSON.stringify(courseSummary()), {
        headers: {
          "cache-control": "no-store",
          "content-type": "application/json; charset=utf-8",
        },
      }),
  );
  const client = createHttpApiClient({ fetch: recordingFetch, basePath: "/live" });

  assert.deepEqual(await client.getCourseInstanceRouteSummary(COURSE_REFERENCE), courseSummary());
  assert.equal(
    requests[0].url,
    `https://client.example.test/live/api/course-instances/${COURSE_REFERENCE}/summary`,
  );
  assert.equal(requests[0].cache, "no-store");
  assert.equal(requests[0].credentials, "same-origin");
});

test("Course Instance route summary rejects noncanonical references before dispatch", () => {
  const { recordingFetch, requests } = createRecordingFetch(async () =>
    Promise.reject(new Error("must not dispatch")),
  );
  const client = createHttpApiClient({ fetch: recordingFetch });

  assert.throws(() => client.getCourseInstanceRouteSummary("C-1"), ApiProtocolError);
  assert.equal(requests.length, 0);
});

test("Course Instance route summary decoder rejects UUID and nonmembership response fields", async () => {
  for (const response of [
    { ...courseSummary(), id: "00000000-0000-0000-0000-000000000001" },
    { ...courseSummary(), role: "sysadmin" },
  ]) {
    const client = createHttpApiClient({
      fetch: async () =>
        new Response(JSON.stringify(response), {
          headers: {
            "cache-control": "no-store",
            "content-type": "application/json; charset=utf-8",
          },
        }),
    });
    await assert.rejects(client.getCourseInstanceRouteSummary(COURSE_REFERENCE), DecodeError);
  }
});
