// Browser transport contract for the implemented Authenticated Session Server Route.

import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeAuthenticatedSession } from "../src/api/decoders.ts";

test("session decoder accepts one immutable Account Product Role and rejects the retired user shape", () => {
  assert.deepEqual(
    decodeAuthenticatedSession({
      authenticated: true,
      account: { id: "00000000-0000-0000-0000-000000000001", productRole: "instructor" },
    }),
    {
      authenticated: true,
      account: { id: "00000000-0000-0000-0000-000000000001", productRole: "instructor" },
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
