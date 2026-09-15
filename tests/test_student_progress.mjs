// Student score disclosure is a server fact, never a browser timing calculation.

import assert from "node:assert/strict";
import test from "node:test";

import { DecodeError } from "../src/api/decoder.ts";
import { decodeStudentAssessmentProgress } from "../src/api/decoders.ts";
import { studentProgressSummary, studentScoreValue } from "../src/student_progress.ts";

const available = {
  assessment_progress: {
    completed_assessment_attempt_count: 2,
    total_question_attempts: 5,
    last_activity_at: 1786000000000,
  },
  student_assessment_grade: {
    score_state: "available",
    assessment_scoring_state: "current",
    current_score: 0.75,
    best_score: 0.9,
    latest_score: 0.8,
  },
};

const unavailableClassStatistics = {
  ...available,
  student_assessment_grade: {
    ...available.student_assessment_grade,
    class_statistics: { state: "unavailable" },
  },
};

const classStatisticsAvailable = {
  ...available,
  student_assessment_grade: {
    ...available.student_assessment_grade,
    class_statistics: {
      state: "available",
      completed_student_cohort_size: 5,
      assessment_average_score: 0.625,
    },
  },
};

test("Student progress is exact, key-free, and never accepts withheld totals", () => {
  assert.deepEqual(decodeStudentAssessmentProgress(available), available);
  for (const forbidden of ["private_scope", "enrollment", "policy", "evaluated_at"]) {
    assert.throws(
      () => decodeStudentAssessmentProgress({ ...available, [forbidden]: "private" }),
      DecodeError,
    );
  }
  assert.throws(
    () =>
      decodeStudentAssessmentProgress({
        ...available,
        student_assessment_grade: {
          ...available.student_assessment_grade,
          score_state: "withheld",
        },
      }),
    DecodeError,
  );
});

test("Class Statistics are an exact, optional safe union", () => {
  assert.deepEqual(
    decodeStudentAssessmentProgress(unavailableClassStatistics),
    unavailableClassStatistics,
  );
  assert.deepEqual(
    decodeStudentAssessmentProgress(classStatisticsAvailable),
    classStatisticsAvailable,
  );

  for (const malformed of [
    { state: "insufficient_evidence", assessment_average_score: 0.75 },
    { state: "insufficient_evidence", completed_student_cohort_size: 8 },
    { state: "available", completed_student_cohort_size: 5 },
    { state: "available", completed_student_cohort_size: 0, assessment_average_score: 0.75 },
    { state: "available", completed_student_cohort_size: 1, assessment_average_score: 0.75 },
    { state: "available", completed_student_cohort_size: 4, assessment_average_score: 0.75 },
    { state: "available", completed_student_cohort_size: 5, assessment_average_score: 1.01 },
    { state: "unknown" },
  ]) {
    assert.throws(
      () =>
        decodeStudentAssessmentProgress({
          ...available,
          student_assessment_grade: {
            ...available.student_assessment_grade,
            class_statistics: malformed,
          },
        }),
      DecodeError,
    );
  }
});

test("Student score copy distinguishes no activity, withheld, and available nulls", () => {
  const noActivity = {
    assessment_progress: {
      completed_assessment_attempt_count: 0,
      total_question_attempts: 0,
      // Starting an Assessment Attempt records activity time, but no score activity exists until submission.
      last_activity_at: 1786000000000,
    },
    student_assessment_grade: {
      score_state: "no_activity",
      assessment_scoring_state: "current",
      current_score: null,
      best_score: null,
      latest_score: null,
    },
  };
  const withheld = {
    ...noActivity,
    assessment_progress: { ...noActivity.assessment_progress, total_question_attempts: 1 },
    student_assessment_grade: {
      ...noActivity.student_assessment_grade,
      score_state: "withheld",
    },
  };
  assert.match(studentProgressSummary(decodeStudentAssessmentProgress(noActivity)), /No score yet/);
  assert.match(
    studentProgressSummary(decodeStudentAssessmentProgress(withheld)),
    /Score is currently unavailable/,
  );
  assert.equal(studentScoreValue(null), "No score yet");
  assert.match(studentProgressSummary(decodeStudentAssessmentProgress(available)), /75%/);
  assert.match(
    studentProgressSummary(
      decodeStudentAssessmentProgress({
        ...available,
        student_assessment_grade: {
          ...available.student_assessment_grade,
          assessment_scoring_state: "recalculating",
          current_score: null,
          best_score: null,
          latest_score: null,
        },
      }),
    ),
    /recalculating/i,
  );
});
