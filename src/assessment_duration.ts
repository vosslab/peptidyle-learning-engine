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

/** Hydrates a validated whole-minute base override without dropping precision. */
export function assessmentDurationOverrideMinutesDraft(seconds: number | null): string {
  if (seconds === null) return "";
  return (seconds / 60).toString();
}

/** Validates the instructor-facing whole-minute draft. */
export function assessmentDurationOverrideMinutesError(minutes: string): string | undefined {
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

/** Blank duration override keeps the calculated whole-minute default. */
export const OPTIONAL_DURATION_OVERRIDE_GUIDANCE =
  "Leave the override blank to use this calculated default.";

/** Blank available date and time mean the Assessment has no opening time. */
export const NO_OPENING_TIME_GUIDANCE =
  "Leave the available date and time blank for no opening time.";

/** Blank closing date and time mean the Assessment has no closing time. */
export const NO_CLOSING_TIME_GUIDANCE =
  "Leave the closing date and time blank for no closing time.";

/** Blank Attempt limit means Weekly, Unit Review, and Bonus stay unlimited. */
export const UNLIMITED_ATTEMPTS_GUIDANCE = "Leave blank for unlimited Attempts.";

export function assessmentDurationDefaultDescription(questionCount: number): string {
  const minutes = calculatedAssessmentDurationMinutes(questionCount);
  return minutes === null
    ? "Default: 1.5 minutes per Question, rounded up to a whole minute (1 to 250 Questions)."
    : `Calculated default: ${minutes} minutes for ${questionCount} Questions (1.5 minutes per Question, rounded up).`;
}
