// Student score disclosure is a server fact, never a browser timing calculation.

import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeStudentAssignmentProgress } from "../src/api/decoders.ts";
import { studentProgressSummary, studentScoreValue } from "../src/student_progress.ts";

const available = {
  score_state: "available",
  scoring_status: "current",
  current_score: 0.75,
  best_score: 0.9,
  latest_score: 0.8,
  completed_run_count: 2,
  total_question_attempts: 5,
  last_activity_at: 1786000000000,
};

const insufficientEvidence = {
  ...available,
  class_statistics: { state: "insufficient_evidence" },
};

const classStatisticsAvailable = {
  ...available,
  class_statistics: {
    state: "available",
    completed_student_cohort_size: 5,
    assignment_average_score: 0.625,
  },
};

test("Student progress is exact, key-free, and never accepts withheld totals", () => {
  assert.deepEqual(decodeStudentAssignmentProgress(available), available);
  for (const forbidden of ["private_scope", "enrollment", "policy", "evaluated_at"]) {
    assert.throws(
      () => decodeStudentAssignmentProgress({ ...available, [forbidden]: "private" }),
      DecodeError,
    );
  }
  assert.throws(
    () => decodeStudentAssignmentProgress({ ...available, score_state: "withheld" }),
    DecodeError,
  );
});

test("Student class statistics are an exact, optional safe union", () => {
  assert.deepEqual(decodeStudentAssignmentProgress(insufficientEvidence), insufficientEvidence);
  assert.deepEqual(
    decodeStudentAssignmentProgress(classStatisticsAvailable),
    classStatisticsAvailable,
  );

  for (const malformed of [
    { state: "insufficient_evidence", assignment_average_score: 0.75 },
    { state: "insufficient_evidence", completed_student_cohort_size: 8 },
    { state: "available", completed_student_cohort_size: 5 },
    { state: "available", completed_student_cohort_size: 0, assignment_average_score: 0.75 },
    { state: "available", completed_student_cohort_size: 1, assignment_average_score: 0.75 },
    { state: "available", completed_student_cohort_size: 4, assignment_average_score: 0.75 },
    { state: "available", completed_student_cohort_size: 5, assignment_average_score: 1.01 },
    { state: "unknown" },
  ]) {
    assert.throws(
      () => decodeStudentAssignmentProgress({ ...available, class_statistics: malformed }),
      DecodeError,
    );
  }
});

test("Student score copy distinguishes no activity, withheld, and available nulls", () => {
  const noActivity = {
    score_state: "no_activity",
    scoring_status: "current",
    current_score: null,
    best_score: null,
    latest_score: null,
    completed_run_count: 0,
    total_question_attempts: 0,
    // Starting a run records activity time, but no score activity exists until submission.
    last_activity_at: 1786000000000,
  };
  const withheld = {
    ...noActivity,
    score_state: "withheld",
    total_question_attempts: 1,
    last_activity_at: 1786000000000,
  };
  assert.match(studentProgressSummary(decodeStudentAssignmentProgress(noActivity)), /No score yet/);
  assert.match(
    studentProgressSummary(decodeStudentAssignmentProgress(withheld)),
    /Score is currently unavailable/,
  );
  assert.equal(studentScoreValue(null), "No score yet");
  assert.match(studentProgressSummary(decodeStudentAssignmentProgress(available)), /75%/);
  assert.match(
    studentProgressSummary(
      decodeStudentAssignmentProgress({
        ...available,
        scoring_status: "recalculating",
        current_score: null,
        best_score: null,
        latest_score: null,
      }),
    ),
    /recalculating/i,
  );
});
