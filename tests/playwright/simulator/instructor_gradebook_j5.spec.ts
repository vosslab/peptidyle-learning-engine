// instructor_gradebook_j5.spec.ts - offline public-only J5 fragment contract.

import { expect, test } from "@playwright/test";

import {
  instructorGradebookLinkSelector,
  passedJ5SummaryEvidence,
  passedW4Fragment,
} from "./instructor_gradebook_j5";
import { j5V2Input } from "./j5_v2_handoff";

test("J5 binds the Gradebook control to the exact arranged course route", () => {
  expect(instructorGradebookLinkSelector("123e4567-e89b-12d3-a456-426614174000")).toBe(
    'a[href="/instructor/courses/123e4567-e89b-12d3-a456-426614174000/gradebook"]',
  );
});

test("J5 gradebook fragment retains only exact public course and assignment identifiers", () => {
  const fragment = passedW4Fragment(
    "123e4567-e89b-12d3-a456-426614174000",
    "123e4567-e89b-12d3-a456-426614174001",
    123,
  );

  expect(fragment).toEqual({
    schemaVersion: 1,
    journey: "J5",
    status: "PASS",
    elapsedMs: 123,
    courseId: "123e4567-e89b-12d3-a456-426614174000",
    assignmentId: "123e4567-e89b-12d3-a456-426614174001",
    visibleOutcomeCodes: ["visible_gradebook", "visible_run_history"],
    diagnostics: [],
  });
  expect(JSON.stringify(fragment)).not.toMatch(/score|date|learner|student|credential/iu);
});

test("J5 score evidence uses only public IDs and closed browser-only milestones", () => {
  const input = j5V2Input(
    "123e4567-e89b-12d3-a456-426614174000",
    "123e4567-e89b-12d3-a456-426614174001",
  );
  const evidence = passedJ5SummaryEvidence(input.courseId, input.assignmentId, 123);

  expect(evidence).toEqual({
    schemaVersion: 2,
    journey: "J5",
    status: "PASS",
    elapsedMs: 123,
    courseId: "123e4567-e89b-12d3-a456-426614174000",
    assignmentId: "123e4567-e89b-12d3-a456-426614174001",
    visibleOutcomeCodes: ["visible_gradebook", "visible_score_summary", "visible_two_run_history"],
    diagnostics: [],
  });
  expect(JSON.stringify(evidence)).not.toMatch(/100%|learner|student|run 1|credential/iu);
});

test("J5 rejects noncanonical public handoff identifiers", () => {
  expect(() => j5V2Input("not-a-uuid", "123e4567-e89b-12d3-a456-426614174001")).toThrow(
    "J5 requires canonical public course and assignment identifiers",
  );
});
