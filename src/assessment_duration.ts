// assessment_duration.ts - calculated Assessment duration and minute-based UI boundary helpers.

export const ASSESSMENT_DURATION_OVERRIDE_MAXIMUM_MINUTES = 12 * 60;

/** Converts a valid UI override to the seconds-only API and persistence contract. */
export function assessmentDurationOverrideSecondsFromMinutesDraft(
  minutes: string,
): number | null | undefined {
  if (minutes === "") return null;
  if (!/^[1-9][0-9]*$/u.test(minutes)) return undefined;
  const wholeMinutes = Number(minutes);
  return wholeMinutes <= ASSESSMENT_DURATION_OVERRIDE_MAXIMUM_MINUTES
    ? wholeMinutes * 60
    : undefined;
}

/** Hydrates the minute control only when stored seconds can be shown exactly. */
export function assessmentDurationOverrideMinutesDraft(seconds: number | null): string {
  if (seconds === null || seconds % 60 !== 0) return "";
  return (seconds / 60).toString();
}

/** Explains invalid minute input or a legacy stored value without changing either value. */
export function assessmentDurationOverrideMinutesError(
  minutes: string,
  legacySeconds: number | null,
): string | undefined {
  if (legacySeconds !== null) {
    return `Stored duration ${assessmentDurationDisplay(legacySeconds)} cannot be shown as whole minutes. Enter 1 to ${ASSESSMENT_DURATION_OVERRIDE_MAXIMUM_MINUTES} whole minutes or clear the override to replace it.`;
  }
  return assessmentDurationOverrideSecondsFromMinutesDraft(minutes) === undefined
    ? `Enter a whole number from 1 to ${ASSESSMENT_DURATION_OVERRIDE_MAXIMUM_MINUTES} minutes, or leave the override blank for the calculated default.`
    : undefined;
}

/** Presents stored seconds in instructor-facing units without changing the stored value. */
export function assessmentDurationDisplay(seconds: number): string {
  if (!Number.isSafeInteger(seconds) || seconds < 1)
    return `${seconds} seconds (invalid stored duration)`;
  if (seconds % 60 !== 0)
    return `${seconds} seconds (stored duration is not a whole number of minutes)`;

  const totalMinutes = seconds / 60;
  if (totalMinutes < 60) return `${totalMinutes} minute${totalMinutes === 1 ? "" : "s"}`;

  const hours = Math.floor(totalMinutes / 60);
  const minutes = totalMinutes % 60;
  const hourCopy = `${hours} hour${hours === 1 ? "" : "s"}`;
  if (minutes === 0) return hourCopy;
  return `${hourCopy} ${minutes} minute${minutes === 1 ? "" : "s"}`;
}

/** Calculates the Human Guidance default in its required whole-minute unit. */
export function calculatedAssessmentDurationMinutes(questionCount: number): number | null {
  return Number.isInteger(questionCount) && questionCount >= 1 && questionCount <= 250
    ? Math.ceil(1.5 * questionCount)
    : null;
}

/** Empty drafts have default intent but no fabricated zero-second duration. */
export function calculatedAssessmentDurationSeconds(questionCount: number): number | null {
  const minutes = calculatedAssessmentDurationMinutes(questionCount);
  return minutes === null ? null : minutes * 60;
}

export function assessmentDurationDefaultDescription(questionCount: number): string {
  const minutes = calculatedAssessmentDurationMinutes(questionCount);
  return minutes === null
    ? "Default: 1.5 minutes per Question, rounded up to a whole minute (1 to 250 Questions)."
    : `Calculated default: ${minutes} minutes for ${questionCount} Questions (1.5 minutes per Question, rounded up).`;
}
