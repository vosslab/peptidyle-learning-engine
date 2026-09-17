// Stable download transport: sensitive errors and wrong representations never become files.

import assert from "node:assert/strict";
import test from "node:test";

import { createCourseGradebookClient } from "../src/api/http_client/live_gradebook.ts";
import { ApiProtocolError, ApiRequestError } from "../src/api/http_client/error.ts";
import { createRecordingFetch } from "./http_client_test_support.mjs";

const COURSE = "CI7K3M2QAZ";

function downloadHeaders(format) {
  return {
    "cache-control": "no-store",
    "content-type":
      format === "csv" ? "text/csv; charset=utf-8" : "text/tab-separated-values; charset=utf-8",
    "content-disposition": `attachment; filename="ple_${COURSE}_grades.${format}"`,
    "x-content-type-options": "nosniff",
  };
}

test("Gradebook CSV and TSV downloads retain server bytes and use the same-origin boundary", async () => {
  for (const format of ["csv", "tsv"]) {
    const bytes = '"roster_id"\r\n"001"\r\n';
    const { recordingFetch, requests } = createRecordingFetch(
      async () => new Response(bytes, { headers: downloadHeaders(format) }),
    );
    const client = createCourseGradebookClient(recordingFetch, "/live");
    const blob = await client.downloadCourseGradebook(COURSE, format);
    assert.ok(blob instanceof Blob);
    assert.equal(await blob.text(), bytes);
    assert.equal(blob.type.split(";")[0], downloadHeaders(format)["content-type"].split(";")[0]);
    assert.equal(
      requests[0].url,
      `https://client.example.test/live/api/course-instances/${COURSE}/gradebook/export?format=${format}`,
    );
    assert.equal(requests[0].credentials, "same-origin");
    assert.equal(requests[0].cache, "no-store");
  }
});

test("Gradebook download rejects errors and altered attachment contracts before reading a body", async () => {
  const cases = [
    { status: 404, headers: {}, error: ApiRequestError },
    { status: 503, headers: downloadHeaders("csv"), error: ApiRequestError },
    { status: 201, headers: downloadHeaders("csv"), error: ApiProtocolError },
    ...["cache-control", "content-type", "content-disposition", "x-content-type-options"].map(
      (header) => ({
        status: 200,
        headers: { ...downloadHeaders("csv"), [header]: "wrong" },
        error: ApiProtocolError,
      }),
    ),
  ];
  for (const fixture of cases) {
    const response = new Response("sensitive error body", fixture);
    const client = createCourseGradebookClient(async () => response, "");
    await assert.rejects(client.downloadCourseGradebook(COURSE, "csv"), fixture.error);
    assert.equal(response.bodyUsed, false);
  }
});

test("Gradebook download rejects noncanonical courses and unsupported formats before dispatch", async () => {
  const { recordingFetch, requests } = createRecordingFetch(async () => {
    throw new Error("must not dispatch");
  });
  const client = createCourseGradebookClient(recordingFetch, "");
  for (const [course, format] of [
    ["C-1", "csv"],
    [COURSE, "xlsx"],
  ]) {
    await assert.rejects(client.downloadCourseGradebook(course, format), ApiProtocolError);
  }
  assert.equal(requests.length, 0);
});
