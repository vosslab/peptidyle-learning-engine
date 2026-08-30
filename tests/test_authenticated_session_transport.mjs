// Browser transport contract for the mounted Authenticated Session route.

import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError, decodeAuthenticatedSession } from "../src/api/decoders.ts";

test("session decoder accepts one immutable Account role and rejects the retired user shape", () => {
  assert.deepEqual(
    decodeAuthenticatedSession({
      authenticated: true,
      account: { id: "account-a", role: "instructor" },
    }),
    { authenticated: true, account: { id: "account-a", role: "instructor" } },
  );
  assert.throws(
    () =>
      decodeAuthenticatedSession({
        authenticated: true,
        user: { id: "user-a", displayName: "Ada", roles: ["instructor"] },
      }),
    DecodeError,
  );
});
