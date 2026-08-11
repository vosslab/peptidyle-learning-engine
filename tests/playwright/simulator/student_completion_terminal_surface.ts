// student_completion_terminal_surface.ts - classifier for visible completion outcomes.

export type StudentCompletionTerminalSurface =
  "pending" | "feedback" | "neutral" | "mastery" | "closed" | "inconsistent" | "error";

export interface StudentCompletionTerminalObservation {
  readonly freshPractice: boolean;
  readonly masteryHeading: boolean;
  readonly closedHeading: boolean;
  readonly neutralHeading: boolean;
  readonly feedback: boolean;
  readonly inlineErrors: number;
}

/**
 * Classifies only rendered terminal controls after Continue. Neutral completion,
 * Feedback, and absence of a terminal surface are asynchronous transients.
 */
export function classifyStudentCompletionTerminalSurface({
  freshPractice,
  masteryHeading,
  closedHeading,
  neutralHeading,
  feedback,
  inlineErrors,
}: StudentCompletionTerminalObservation): StudentCompletionTerminalSurface {
  if (inlineErrors > 0) return "error";
  if (freshPractice !== masteryHeading) return "inconsistent";
  if (freshPractice && masteryHeading) return "mastery";
  if (closedHeading) return "closed";
  if (neutralHeading) return "neutral";
  if (feedback) return "feedback";
  return "pending";
}

export function isStudentCompletionTerminalSurface(
  surface: StudentCompletionTerminalSurface,
): boolean {
  return (
    surface === "mastery" ||
    surface === "closed" ||
    surface === "inconsistent" ||
    surface === "error"
  );
}
