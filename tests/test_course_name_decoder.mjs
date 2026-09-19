import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeCourseSummary } from "../src/api/decoders.ts";

function courseSummary(shortName, longName) {
  return {
    id: "CI7K3M2QAZ",
    classification: {
      disciplineUuid: "018f5e7d-01b6-7c14-8a0b-4bfef6390d6d",
      subjectUuid: null,
      topicUuid: null,
      subtopicUuid: null,
      tags: [],
    },
    shortName,
    longName,
    term: { startDate: "2026-01-01", endDate: "2026-05-01" },
    role: "instructor",
  };
}

test("Course Instance names remain distinct from the bounded Question Title decoder", () => {
  const longName = "C".repeat(200);
  const decoded = decodeCourseSummary(courseSummary("Mol Bio", longName));
  assert.equal(decoded.shortName, "Mol Bio");
  assert.equal(decoded.longName, longName);
  assert.throws(() => decodeCourseSummary(courseSummary("  ", longName)), DecodeError);
  assert.throws(() => decodeCourseSummary(courseSummary("Mol Bio", "  ")), DecodeError);
  assert.throws(() => decodeCourseSummary(courseSummary(" Mol Bio", longName)), DecodeError);
  assert.throws(() => decodeCourseSummary(courseSummary("Mol Bio", "C".repeat(201))), DecodeError);
});
