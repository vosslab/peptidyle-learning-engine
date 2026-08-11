// j4_terminal_surface.ts - public visible-terminal classifier for the J4 contrast.

export type J4TerminalSurface =
  "pending" | "feedback" | "neutral" | "mastery" | "closed" | "inconsistent" | "error";

export interface J4TerminalObservation {
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
export function classifyJ4TerminalSurface({
  freshPractice,
  masteryHeading,
  closedHeading,
  neutralHeading,
  feedback,
  inlineErrors,
}: J4TerminalObservation): J4TerminalSurface {
  if (inlineErrors > 0) return "error";
  if (freshPractice !== masteryHeading) return "inconsistent";
  if (freshPractice && masteryHeading) return "mastery";
  if (closedHeading) return "closed";
  if (neutralHeading) return "neutral";
  if (feedback) return "feedback";
  return "pending";
}

export function isJ4TerminalSurfaceTerminal(surface: J4TerminalSurface): boolean {
  return (
    surface === "mastery" ||
    surface === "closed" ||
    surface === "inconsistent" ||
    surface === "error"
  );
}
