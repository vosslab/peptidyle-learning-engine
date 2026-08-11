import assert from "node:assert/strict";
import test from "node:test";

import { passedStudentLeaveResumeEvidence } from "./playwright/simulator/student_leave_resume_evidence.ts";

const COURSE = "123e4567-e89b-12d3-a456-426614174000";
const ASSIGNMENT = "123e4567-e89b-12d3-a456-426614174001";

test("leave-and-resume evidence rejects invalid identifiers and unbounded time", () => {
  assert.throws(() => passedStudentLeaveResumeEvidence("not-a-uuid", ASSIGNMENT, 12));
  assert.throws(() => passedStudentLeaveResumeEvidence(COURSE, ASSIGNMENT, 30 * 60 * 1000 + 1));
});
