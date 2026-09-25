// Student Course Progress copy keeps activity, score release, and perfection distinct.

import type { StudentCourseProgressAssessment } from "./api/live_student_course_landing";
import { formatPercentScore, formatPointScore } from "./score_format";

export function completedStudentAssessmentCount(
  assessments: ReadonlyArray<StudentCourseProgressAssessment>,
): number {
  return assessments.filter((assessment) => assessment.submittedAssessmentAttemptCount > 0).length;
}

export function studentAssessmentActivityLabel(
  assessment: StudentCourseProgressAssessment,
): string {
  if (assessment.assessmentAttemptCount === 0) return "Not started";
  const attempt =
    assessment.latestAssessmentAttemptNumber === null
      ? "Latest Attempt"
      : `Latest Attempt ${assessment.latestAssessmentAttemptNumber}`;
  return assessment.latestAssessmentAttemptCompletion === "completed"
    ? `${attempt} submitted`
    : `${attempt} in progress`;
}

export function studentAssessmentScoreStateLabel(
  assessment: StudentCourseProgressAssessment,
): string {
  const score = assessment.assessmentScore;
  if (score === undefined) {
    return assessment.assessmentAttemptCount === 0 ? "Not started" : "Score not released";
  }
  if (score.pointsPossible === 0) return "Released score · bonus points";
  return score.pointsEarned >= score.pointsPossible ? "Perfect score" : "Below 100%";
}

export function studentAssessmentScoreDescription(
  assessment: StudentCourseProgressAssessment,
): string {
  const score = assessment.assessmentScore;
  if (score === undefined) {
    if (assessment.assessmentAttemptCount === 0) {
      return "No Attempt has started yet.";
    }
    return assessment.submittedAssessmentAttemptCount > 0
      ? `${assessment.submittedAssessmentAttemptCount} submitted Attempt${assessment.submittedAssessmentAttemptCount === 1 ? "" : "s"}; no score has been released.`
      : "An Attempt is still in progress, so no score is released yet.";
  }
  const label = assessment.assessmentScoreIsLatestAttempt
    ? "Latest released score"
    : "Best released score";
  if (score.pointsPossible === 0) {
    return `${label}: ${score.pointsEarned} bonus points. This Coursework item has no percentage score.`;
  }
  return `${label}: ${formatPointScore(score.pointsEarned, score.pointsPossible)} points (${formatPercentScore(score.pointsEarned / score.pointsPossible)}).`;
}
