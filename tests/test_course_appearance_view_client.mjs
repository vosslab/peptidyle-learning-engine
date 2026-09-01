// Browser HTTP-client contract for the safe current Course Appearance View.

import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { createHttpApiClient, ApiProtocolError } from "../src/api/http_client.ts";
import { createRecordingFetch } from "./http_client_test_support.mjs";

const COURSE_ID = "00000000-0000-0000-0000-000000000001";

function appearanceView() {
  return { theme: "grass", revision: "7", banner: null };
}

function appearanceResponse(body, etag = '"7"') {
  const headers = {
    "cache-control": "no-store",
    "content-type": "application/json; charset=utf-8",
  };
  if (typeof etag === "string") headers.etag = etag;
  return new Response(JSON.stringify(body), { headers });
}

function appearanceClient(response) {
  return createHttpApiClient({ fetch: async () => response });
}

test("Course Appearance View client requests the exact no-store reader and accepts its matching revision", async () => {
  const { recordingFetch, requests } = createRecordingFetch(async () =>
    appearanceResponse(appearanceView()),
  );
  const client = createHttpApiClient({ fetch: recordingFetch, basePath: "/live" });

  const appearance = await client.getCourseAppearanceView(COURSE_ID);

  assert.deepEqual(appearance, appearanceView());
  assert.equal(
    requests[0].url,
    `https://client.example.test/live/api/courses/${COURSE_ID}/appearance`,
  );
  assert.equal(requests[0].method, "GET");
  assert.equal(requests[0].cache, "no-store");
  assert.equal(requests[0].headers.get("accept"), "application/json");
  assert.equal(requests[0].credentials, "same-origin");
});

test("Course Appearance View client requires its exact strong revision ETag", async () => {
  await assert.rejects(
    appearanceClient(appearanceResponse(appearanceView(), null)).getCourseAppearanceView(COURSE_ID),
    ApiProtocolError,
  );
  await assert.rejects(
    appearanceClient(appearanceResponse(appearanceView(), '"8"')).getCourseAppearanceView(
      COURSE_ID,
    ),
    /ETag does not match its appearance revision/u,
  );
});

test("Course Appearance View client rejects surplus and retired reader properties", async () => {
  const surplusView = { ...appearanceView(), privateAppearanceField: "must-not-be-accepted" };
  await assert.rejects(
    appearanceClient(appearanceResponse(surplusView)).getCourseAppearanceView(COURSE_ID),
    DecodeError,
  );

  const retiredBanner = {
    ...appearanceView(),
    banner: {
      id: "00000000-0000-0000-0000-000000000007",
      alternativeText: { kind: "decorative" },
    },
  };
  await assert.rejects(
    appearanceClient(appearanceResponse(retiredBanner)).getCourseAppearanceView(COURSE_ID),
    DecodeError,
  );
});
