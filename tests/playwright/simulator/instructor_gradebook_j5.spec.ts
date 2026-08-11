// instructor_gradebook_j5.spec.ts - offline public-only J5 fragment contract.

import { expect, test } from "@playwright/test";

import { j5V2Input } from "./j5_v2_handoff";

test("gradebook handoff rejects noncanonical public identifiers", () => {
  expect(() => j5V2Input("not-a-uuid", "123e4567-e89b-12d3-a456-426614174001")).toThrow(
    "J5 requires canonical public course and assignment identifiers",
  );
});
