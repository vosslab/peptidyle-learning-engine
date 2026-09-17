// Stable Profile composition contract: square bounds, reachable edges, and invalid inputs.
import assert from "node:assert/strict";
import test from "node:test";

import { profileImageCrop } from "../src/features/profile_avatar/profile_image_crop.ts";
import { decodeProfileImageCropInput } from "../src/api/decoders/profile_avatar.ts";
import { createResponseClient } from "../src/api/http_client/response.ts";

function input(sourceWidth, sourceHeight, horizontal = 50, vertical = 50, zoomPercent = 100) {
  return { sourceWidth, sourceHeight, horizontal, vertical, zoomPercent };
}

test("Profile crop preserves a square and reaches the chosen edges at each zoom", () => {
  assert.deepEqual(profileImageCrop(input(512, 256)), { x: 128, y: 0, side: 256 });
  assert.deepEqual(profileImageCrop(input(256, 512, 0, 100)), { x: 0, y: 256, side: 256 });
  assert.deepEqual(profileImageCrop(input(256, 256, 100, 0, 200)), { x: 128, y: 0, side: 128 });
  // Same non-divisible fixture as Rust: integer edges must not drift across the boundary.
  assert.deepEqual(profileImageCrop(input(515, 259, 37, 83, 155)), { x: 128, y: 76, side: 167 });
  for (const [width, height] of [
    [128, 128],
    [512, 128],
    [128, 512],
  ]) {
    for (const zoom of [100, 155, 200, 400]) {
      const first = profileImageCrop(input(width, height, 0, 0, zoom));
      const last = profileImageCrop(input(width, height, 100, 100, zoom));
      assert.equal(first.x, 0);
      assert.equal(first.y, 0);
      assert.equal(last.x + last.side, width);
      assert.equal(last.y + last.side, height);
      assert.equal(first.side, last.side);
    }
  }
});

test("Profile crop rejects undersized, oversized, and invalid crop inputs", () => {
  for (const [width, height] of [
    [127, 256],
    [256, 127],
    [5000, 5000],
    [NaN, 256],
  ]) {
    assert.throws(() => profileImageCrop(input(width, height)));
  }
  for (const [horizontal, vertical, zoom] of [
    [-1, 50, 100],
    [50, 101, 100],
    [50, 50, 0],
    [50, 50, NaN],
    [50, 50, Infinity],
    [50, 50, 401],
    [50, 50, 155.5],
    [0.5, 50, 100],
    [50, "50", 100],
  ]) {
    assert.throws(() => profileImageCrop(input(256, 256, horizontal, vertical, zoom)));
  }
  assert.throws(() => decodeProfileImageCropInput({ ...input(128, 128), unowned: 1 }));
});

test("Profile upload sends original bytes plus bounded crop geometry without trusting MIME", async () => {
  const original = new Blob([new Uint8Array([0, 1, 2, 3])], { type: "text/plain" });
  const crop = input(515, 259, 37, 83, 155);
  const requests = [];
  const client = createResponseClient(async (path, request) => {
    requests.push({ path, request });
    return Response.json({ avatar: null }, { headers: { "cache-control": "no-store" } });
  }, "/live");
  assert.deepEqual(await client.replaceProfileAvatarImage(original, crop), { avatar: null });
  const [{ path, request }] = requests;
  assert.equal(path, "/live/api/profile/avatar/profile-image");
  assert.equal(request.body, original);
  assert.equal(request.method, "POST");
  assert.equal(request.credentials, "same-origin");
  assert.equal(request.cache, "no-store");
  assert.equal(request.headers["content-type"], "application/octet-stream");
  assert.deepEqual(JSON.parse(request.headers["x-ple-profile-crop"]), crop);
  assert.ok(request.headers["x-ple-profile-crop"].length <= 128);
});

test("Profile upload rejects invalid geometry and empty or oversized originals before dispatch", async () => {
  let requests = 0;
  const client = createResponseClient(async () => {
    requests += 1;
  }, "");
  for (const crop of [
    { ...input(128, 128), zoomPercent: NaN },
    { ...input(128, 128), sourceWidth: 127 },
    { ...input(128, 128), horizontal: 101 },
    { ...input(128, 128), extra: true },
  ]) {
    await assert.rejects(client.replaceProfileAvatarImage(new Blob(["original"]), crop));
  }
  for (const original of [new Blob(), new Blob([new Uint8Array(8 * 1024 * 1024 + 1)])]) {
    await assert.rejects(client.replaceProfileAvatarImage(original, input(128, 128)));
  }
  assert.equal(requests, 0);
});
