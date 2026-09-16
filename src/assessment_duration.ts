// assessment_duration.ts - calculated Assessment duration, without persisted derived state.

/** Empty drafts have default intent but no fabricated zero-second duration. */
export function calculatedAssessmentDurationSeconds(questionCount: number): number | null {
  return Number.isInteger(questionCount) && questionCount >= 1 && questionCount <= 250
    ? Math.ceil(1.5 * questionCount) * 60
    : null;
}

export function assessmentDurationDefaultDescription(questionCount: number): string {
  const seconds = calculatedAssessmentDurationSeconds(questionCount);
  return seconds === null
    ? "Default: 1.5 minutes per Question, rounded up to a whole minute (1 to 250 Questions)."
    : `Calculated default: ${seconds / 60} minutes for ${questionCount} Questions (1.5 minutes per Question, rounded up).`;
}
