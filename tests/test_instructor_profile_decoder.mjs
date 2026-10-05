import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeInstructorProfileView } from "../src/api/decoders/profile_avatar.ts";
import { createResponseClient } from "../src/api/http_client/response.ts";

const profile = {
  displayName: "Elena Rivera",
  avatar: { kind: "profileImage", profileImageId: "57cc9f93-964e-440f-a7fc-cf330f6574f2" },
};

test("Instructor Profile has only its public name and avatar", () => {
  assert.deepEqual(decodeInstructorProfileView(profile), profile);
  for (const extra of ["email", "accountId", "timeZone", "affiliation", "courses"]) {
    assert.throws(
      () => decodeInstructorProfileView({ ...profile, [extra]: "private" }),
      DecodeError,
    );
  }
});

test("Instructor Profile request is same-origin, encoded, and not stored", async () => {
  const requests = [];
  const client = createResponseClient(async (path, request) => {
    requests.push({ path, request });
    return Response.json(profile, { headers: { "cache-control": "no-store" } });
  }, "/live");
  assert.deepEqual(await client.getInstructorProfile("U7K3M2PA0"), profile);
  assert.equal(requests[0].path, "/live/api/instructor-profiles/U7K3M2PA0");
  assert.equal(requests[0].request.credentials, "same-origin");
  assert.equal(requests[0].request.cache, "no-store");
});
