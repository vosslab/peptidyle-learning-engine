// instructor_gradebook_j5.spec.ts - offline public-only J5 fragment contract.

import { expect, test } from "@playwright/test";

import { j5V2Input } from "./j5_v2_handoff";

test("gradebook handoff rejects malformed public route references", () => {
  expect(() => j5V2Input("not-a-route-reference", "A-73")).toThrow(
    "J5 requires canonical public course and assignment references",
  );
});
