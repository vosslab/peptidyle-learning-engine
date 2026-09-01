// test_question_json_asset_client.mjs - transport and secrecy checks for protected hotspot images.

import assert from "node:assert/strict";
import test from "node:test";

import {
  PleQuestionJsonAssetProtocolError,
  PleQuestionJsonAssetRequestError,
  createPleQuestionJsonAssetClient,
  decodePleQuestionJsonAssetDescriptor,
} from "../src/features/ple_question_json_authoring/question_json_asset_client.ts";

const WORKSPACE = "00000000-0000-4000-8000-000000000010";
const ASSET = "aaaaaaaa-0000-4000-8000-000000000011";
const DESCRIPTOR = {
  assetId: ASSET,
  contentChecksum: "a".repeat(64),
  displayLabel: "Cell membrane diagram",
  mediaType: "image/png",
  intrinsicWidth: 800,
  intrinsicHeight: 600,
};

function json(value, status = 200) {
  return new Response(JSON.stringify(value), {
    status,
    headers: { "cache-control": "no-store", "content-type": "application/json; charset=utf-8" },
  });
}

test("asset descriptor decoder accepts only the safe exact browser projection", () => {
  assert.deepEqual(decodePleQuestionJsonAssetDescriptor(DESCRIPTOR), DESCRIPTOR);
  const invalid = [
    { ...DESCRIPTOR, objectKey: "private/object/key" },
    { ...DESCRIPTOR, assetId: DESCRIPTOR.assetId.toUpperCase() },
    { ...DESCRIPTOR, contentChecksum: "A".repeat(64) },
    { ...DESCRIPTOR, mediaType: "image/gif" },
    { ...DESCRIPTOR, intrinsicWidth: 0 },
    { ...DESCRIPTOR, intrinsicHeight: 4_294_967_296 },
    { ...DESCRIPTOR, displayLabel: " label" },
  ];
  for (const value of invalid) {
    assert.throws(() => decodePleQuestionJsonAssetDescriptor(value), /must be/u);
  }
  const serialized = JSON.stringify(decodePleQuestionJsonAssetDescriptor(DESCRIPTOR));
  assert.equal(serialized.includes("objectKey"), false);
  assert.equal(serialized.includes("purpose"), false);
});

test("asset list preserves an empty library and reports a safe request failure", async () => {
  const empty = createPleQuestionJsonAssetClient({ fetch: async () => json([]) });
  assert.deepEqual(await empty.list(WORKSPACE), []);

  const unavailable = createPleQuestionJsonAssetClient({
    fetch: async () => json({ error: "private" }, 503),
  });
  await assert.rejects(unavailable.list(WORKSPACE), PleQuestionJsonAssetRequestError);
});

test("asset upload transports only opaque bytes and the two server-owned descriptive headers", async () => {
  let captured;
  const client = createPleQuestionJsonAssetClient({
    fetch: async (input, init) => {
      captured = new Request(new URL(String(input), "https://ple.example"), init);
      return json(DESCRIPTOR, 201);
    },
  });
  const image = new Blob([new Uint8Array([1, 2, 3, 4])], { type: "image/png" });
  const uploaded = await client.upload(WORKSPACE, {
    image,
    displayLabel: "Cell membrane diagram",
    purpose: "Instructor-selected hotspot surface",
  });

  assert.deepEqual(uploaded, DESCRIPTOR);
  assert.notEqual(captured, undefined);
  assert.equal(captured.method, "POST");
  assert.equal(captured.credentials, "same-origin");
  assert.equal(captured.cache, "no-store");
  assert.equal(
    captured.url.endsWith(`/api/workspaces/${WORKSPACE}/ple-question-json-assets`),
    true,
  );
  assert.equal(captured.headers.get("accept"), "application/json");
  assert.equal(captured.headers.get("content-type"), "image/png");
  assert.equal(captured.headers.get("x-ple-asset-label"), "Cell membrane diagram");
  assert.equal(captured.headers.get("x-ple-asset-purpose"), "Instructor-selected hotspot surface");
  for (const forbidden of [
    "x-ple-asset-checksum",
    "x-ple-asset-width",
    "x-ple-asset-height",
    "x-ple-object-key",
    "x-ple-object-path",
  ]) {
    assert.equal(captured.headers.has(forbidden), false);
  }
  assert.deepEqual(new Uint8Array(await captured.arrayBuffer()), new Uint8Array([1, 2, 3, 4]));
});

test("asset upload refuses caller-generated metadata and unsafe image inputs before fetch", async () => {
  let calls = 0;
  const client = createPleQuestionJsonAssetClient({
    fetch: async () => {
      calls += 1;
      return json(DESCRIPTOR, 201);
    },
  });
  await assert.rejects(
    client.upload(WORKSPACE, {
      image: new Blob(["image"], { type: "image/gif" }),
      displayLabel: "Image",
      purpose: "Instructor-selected hotspot surface",
    }),
    PleQuestionJsonAssetProtocolError,
  );
  await assert.rejects(
    client.upload(WORKSPACE, {
      image: new Blob(["image"], { type: "image/png" }),
      displayLabel: " Image",
      purpose: "Instructor-selected hotspot surface",
    }),
    PleQuestionJsonAssetProtocolError,
  );
  assert.equal(calls, 0);
});
