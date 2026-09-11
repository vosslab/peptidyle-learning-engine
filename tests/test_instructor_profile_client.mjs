import assert from "node:assert/strict";
import test from "node:test";

import { ApiRequestError, createHttpApiClient } from "../src/api/http_client.ts";
import { DecodeError } from "../src/api/decoder.ts";

test("Instructor Profile uses the self-only no-store profile contract", async () => {
  const requests = [];
  const recordingFetch = async (input, init) => {
    requests.push({ input, init });
    return new Response(JSON.stringify({ timeZone: "America/New_York" }), {
      headers: { "content-type": "application/json", "cache-control": "no-store" },
    });
  };
  const client = createHttpApiClient({ fetch: recordingFetch, basePath: "/live" });
  assert.deepEqual(await client.updateInstructorProfile({ timeZone: "America/New_York" }), {
    timeZone: "America/New_York",
  });
  assert.deepEqual(requests, [
    {
      input: "/live/api/instructor-profile",
      init: {
        method: "PATCH",
        headers: { accept: "application/json", "content-type": "application/json" },
        body: '{"timeZone":"America/New_York"}',
        credentials: "same-origin",
        cache: "no-store",
      },
    },
  ]);
});

test("Instructor Profile refuses surplus profile fields and non-success responses", async () => {
  const extra = createHttpApiClient({
    fetch: async () =>
      new Response(JSON.stringify({ timeZone: "UTC", accountId: "no" }), {
        headers: { "content-type": "application/json", "cache-control": "no-store" },
      }),
  });
  await assert.rejects(extra.getInstructorProfile(), DecodeError);
  const denied = createHttpApiClient({
    fetch: async () =>
      new Response("not found", { status: 404, headers: { "cache-control": "no-store" } }),
  });
  await assert.rejects(denied.getInstructorProfile(), ApiRequestError);
});

test("Instructor Profile thumbnail sends image bytes only and checks protected delivery", async () => {
  const requests = [];
  const reference = "018f0a00-1111-7222-8333-444444444444";
  const image = new Blob(["image bytes"], { type: "image/png" });
  const client = createHttpApiClient({
    basePath: "/live",
    fetch: async (input, init) => {
      requests.push({ input, init });
      if (init?.method === "POST" && String(input).endsWith("/thumbnail")) {
        return new Response(JSON.stringify({ reference }), {
          headers: { "content-type": "application/json", "cache-control": "no-store" },
        });
      }
      return new Response("webp", {
        headers: {
          "content-type": "image/webp",
          "cache-control": "no-store",
          "x-content-type-options": "nosniff",
        },
      });
    },
  });
  assert.deepEqual(await client.replaceInstructorProfileThumbnail(image), { reference });
  assert.equal((await client.fetchInstructorProfileThumbnail(reference)).type, "image/webp");
  assert.equal(requests[0].input, "/live/api/instructor-profile/thumbnail");
  assert.equal(requests[0].init.body, image);
  assert.deepEqual(requests[0].init.headers, {
    accept: "application/json",
    "content-type": "application/octet-stream",
  });
  assert.equal(requests[1].input, `/live/api/instructor-profile/thumbnails/${reference}/delivery`);
});
