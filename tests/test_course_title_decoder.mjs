import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeCourseSummary } from "../src/api/decoders.ts";

function courseSummary(title) {
  return {
    id: "00000000-0000-0000-0000-000000000001",
    reference: "C-1",
    title,
    term: { startDate: "2026-01-01", endDate: "2026-05-01", timeZone: "America/Chicago" },
    role: "instructor",
  };
}

test("Course Title remains distinct from the bounded Question Title decoder", () => {
  const title = "C".repeat(513);
  assert.equal(decodeCourseSummary(courseSummary(title)).title, title);
  assert.throws(() => decodeCourseSummary(courseSummary("  ")), DecodeError);
});
