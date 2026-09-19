import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import {
  decodeBlueprintStar,
  decodeBlueprintStarredInstructors,
  decodeBlueprintWatch,
  decodeBlueprintWatchEvents,
} from "../src/api/decoders/blueprint_stewardship.ts";
import {
  createHttpApiClient,
  ApiProtocolError,
  ApiRequestError,
  BlueprintCourseConflictError,
} from "../src/api/http_client.ts";

const reference = "BP7K3M2QAF";
const blueprintEditNumber = "1";

function response(body, headers = {}) {
  return new Response(JSON.stringify(body), {
    headers: { "content-type": "application/json", "cache-control": "no-store", ...headers },
  });
}

// A privacy-contract regression requires fixing the projection, never allowing extra fields.
test("Blueprint Stars preserve exact verified names and reject substitute identities or Watch facts", () => {
  const name = "Dr. N. Garc\u00eda <Biology>";
  assert.deepEqual(
    decodeBlueprintStarredInstructors({ starredInstructors: [{ displayName: name }] }),
    [{ displayName: name }],
  );
  for (const forbidden of [
    "email",
    "accountId",
    "profileUrl",
    "avatar",
    "watching",
  ]) {
    assert.throws(
      () =>
        decodeBlueprintStarredInstructors({
          starredInstructors: [{ displayName: name, [forbidden]: "private" }],
        }),
      DecodeError,
    );
    assert.throws(
      () => decodeBlueprintStar({ starCount: 1, viewerHasStarred: true, [forbidden]: "private" }),
      DecodeError,
    );
  }
  assert.throws(
    () =>
      decodeBlueprintStarredInstructors({
        starredInstructors: [{ displayName: "  Substitute  " }],
      }),
    DecodeError,
  );
  assert.throws(() => decodeBlueprintWatch({ watching: true, watchers: [name] }), DecodeError);
  assert.throws(
    () =>
      decodeBlueprintWatchEvents({
        events: [{ kind: "revision", occurredAt: 1, actorId: "private" }],
      }),
    DecodeError,
  );
});

test("Blueprint Star and Watch changes remain explicit separate same-origin actions", async () => {
  const requests = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(input.toString(), "https://ple.example"), init);
      requests.push(request);
      const path = new URL(request.url).pathname;
      if (path.endsWith("starred-instructors"))
        return response({ starredInstructors: [{ displayName: "Dr. Rivera" }] });
      if (path.endsWith("star")) return response({ starCount: 1, viewerHasStarred: true });
      if (path.endsWith("watch")) return response({ watching: true });
      return response({ events: [{ kind: "archived", occurredAt: 1750000000000 }] });
    },
  });
  assert.deepEqual(await client.getBlueprintStar(reference), {
    starCount: 1,
    viewerHasStarred: true,
  });
  assert.deepEqual(await client.getBlueprintStarredInstructors(reference), [
    { displayName: "Dr. Rivera" },
  ]);
  assert.deepEqual(await client.getBlueprintWatch(reference), { watching: true });
  assert.deepEqual(await client.getBlueprintWatchEvents(reference), [
    { kind: "archived", occurredAt: 1750000000000 },
  ]);
  await client.setBlueprintStar(reference, false);
  await client.setBlueprintWatch(reference, false);
  assert.equal(requests.length, 6);
  assert.ok(requests.every((request) => request.credentials === "same-origin"));
  assert.deepEqual(
    requests.map((request) => request.method),
    ["GET", "GET", "GET", "GET", "PUT", "PUT"],
  );
  assert.equal(
    new URL(requests[0].url).pathname,
    `/api/course-blueprints/${reference}/stewardship/star`,
  );
  assert.deepEqual(await requests[4].json(), { starred: false });
  assert.deepEqual(await requests[5].json(), { watching: false });
  assert.equal(new URL(requests[3].url).searchParams.get("limit"), "25");
});

test("Blueprint promotion uses only the Sysadmin endpoint and the exact current metadata validator", async () => {
  const requests = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      requests.push(new Request(new URL(input.toString(), "https://ple.example"), init));
      return response(
        { promoted: true, blueprintEditNumber },
        { etag: `"${blueprintEditNumber}"` },
      );
    },
  });
  const loaded = await client.getBlueprintPromotion(reference);
  assert.deepEqual(loaded, { promoted: true, blueprintEditNumber });
  await client.setBlueprintPromotion(reference, false, loaded.blueprintEditNumber);
  assert.equal(
    new URL(requests[1].url).pathname,
    `/api/sysadmin/course-blueprints/${reference}/promotion`,
  );
  assert.equal(requests[1].headers.get("if-match"), `"${blueprintEditNumber}"`);
  assert.deepEqual(await requests[1].json(), { promoted: false });

  const mismatch = createHttpApiClient({
    fetch: async () =>
      response(
        { promoted: true, blueprintEditNumber },
        { etag: '"018f5e7d-01b6-7c14-8a0b-4bfef6390d6e"' },
      ),
  });
  await assert.rejects(mismatch.getBlueprintPromotion(reference), ApiProtocolError);
  const stale = createHttpApiClient({
    fetch: async () =>
      new Response(null, { status: 412, headers: { "cache-control": "no-store" } }),
  });
  await assert.rejects(
    stale.setBlueprintPromotion(reference, true, loaded.blueprintEditNumber),
    BlueprintCourseConflictError,
  );
});

test("Blueprint stewardship rejects invalid commands before dispatch and concealed authorization responses", async () => {
  let calls = 0;
  const client = createHttpApiClient({
    fetch: async () => {
      calls += 1;
      return new Response(null, { status: 404, headers: { "cache-control": "no-store" } });
    },
  });
  await assert.rejects(client.getBlueprintStar("BP-invalid"), DecodeError);
  await assert.rejects(client.setBlueprintStar(reference, "true"), DecodeError);
  await assert.rejects(client.setBlueprintWatch(reference, 1), DecodeError);
  await assert.rejects(client.getBlueprintWatchEvents(reference, 101), ApiProtocolError);
  assert.equal(calls, 0);
  await assert.rejects(
    client.getBlueprintStar(reference),
    (error) => error instanceof ApiRequestError && error.status === 404,
  );
  await assert.rejects(
    client.getBlueprintPromotion(reference),
    (error) => error instanceof ApiRequestError && error.status === 404,
  );
  const cached = createHttpApiClient({
    fetch: async () => response({ watching: true }, { "cache-control": "public" }),
  });
  await assert.rejects(cached.getBlueprintWatch(reference), ApiProtocolError);
});
