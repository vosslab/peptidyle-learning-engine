import assert from "node:assert/strict";
import test from "node:test";

import {
  completedStudentAssessmentCount,
  studentAssessmentActivityLabel,
  studentAssessmentScoreDescription,
  studentAssessmentScoreStateLabel,
} from "../src/student_course_progress_presentation.ts";

function progress(overrides = {}) {
  return {
    id: "A5D9Q3XAH",
    title: "Peptide practice",
    assessmentType: "regular_assignment",
    assessmentAttemptCount: 0,
    submittedAssessmentAttemptCount: 0,
    latestAssessmentAttemptNumber: null,
    latestAssessmentAttemptCompletion: null,
    latestActivityAt: null,
    ...overrides,
  };
}

test("Course Progress keeps attempted Assessments visible while a score is unreleased", () => {
  const assessment = progress({
    assessmentAttemptCount: 2,
    submittedAssessmentAttemptCount: 1,
    latestAssessmentAttemptNumber: 2,
    latestAssessmentAttemptCompletion: "inProgress",
    latestActivityAt: 1_786_000_000_000,
  });

  assert.equal(studentAssessmentScoreStateLabel(assessment), "Score not released");
  assert.equal(studentAssessmentActivityLabel(assessment), "Latest Attempt 2 in progress");
  assert.match(studentAssessmentScoreDescription(assessment), /1 submitted Attempt/);
  assert.match(studentAssessmentScoreDescription(assessment), /stays visible/);
  assert.equal(completedStudentAssessmentCount([assessment]), 1);
});

test("Course Progress separates no activity, below-perfect, perfect, and Bonus scores", () => {
  const unstarted = progress();
  const belowPerfect = progress({
    assessmentAttemptCount: 1,
    submittedAssessmentAttemptCount: 1,
    latestAssessmentAttemptNumber: 1,
    latestAssessmentAttemptCompletion: "completed",
    latestActivityAt: 1_786_000_000_000,
    assessmentScore: { pointsEarned: 7, pointsPossible: 8 },
    assessmentScoreIsLatestAttempt: true,
  });
  const perfect = progress({
    ...belowPerfect,
    assessmentScore: { pointsEarned: 8, pointsPossible: 8 },
  });
  const bonus = progress({
    ...belowPerfect,
    assessmentScore: { pointsEarned: 2, pointsPossible: 0 },
  });

  assert.equal(studentAssessmentScoreStateLabel(unstarted), "Not started");
  assert.equal(studentAssessmentActivityLabel(unstarted), "Not started");
  assert.match(studentAssessmentScoreDescription(unstarted), /No Attempt has started/);
  assert.equal(studentAssessmentScoreStateLabel(belowPerfect), "Below 100%");
  assert.match(studentAssessmentScoreDescription(belowPerfect), /Latest released score: 7 \/ 8/);
  assert.equal(studentAssessmentScoreStateLabel(perfect), "Perfect score");
  assert.equal(studentAssessmentScoreStateLabel(bonus), "Released score · bonus points");
  assert.match(studentAssessmentScoreDescription(bonus), /no percentage score/i);
  assert.equal(completedStudentAssessmentCount([unstarted, belowPerfect, perfect, bonus]), 3);
});
