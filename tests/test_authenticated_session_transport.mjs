// Browser transport contract for the mounted Authenticated Session route.

import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeAuthenticatedSession } from "../src/api/decoders.ts";

test("session decoder accepts one immutable Account role and rejects the retired user shape", () => {
  assert.deepEqual(
    decodeAuthenticatedSession({
      authenticated: true,
      account: { id: "00000000-0000-0000-0000-000000000001", role: "instructor" },
    }),
    {
      authenticated: true,
      account: { id: "00000000-0000-0000-0000-000000000001", role: "instructor" },
    },
  );
  assert.throws(
    () =>
      decodeAuthenticatedSession({
        authenticated: true,
        user: {
          id: "00000000-0000-0000-0000-000000000002",
          displayName: "Ada",
          roles: ["instructor"],
        },
      }),
    DecodeError,
  );
});
