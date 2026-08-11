// post_start_surface.ts - public visible-state classification after Mastery start/resume.

export type PostStartSurface = "run" | "fresh-practice" | "error" | "pending";
export type FinalSurface =
  "run" | "fresh-practice" | "error" | "feedback" | "neutral" | "closed" | "pending";

export interface PostStartSurfaceCounts {
  readonly radios: number;
  readonly freshPractice: boolean;
  readonly inlineErrors: number;
}

export interface FinalSurfaceCounts extends PostStartSurfaceCounts {
  readonly continueVisible: boolean;
  readonly feedbackVisible: boolean;
  readonly neutralComplete: boolean;
  readonly closedComplete: boolean;
}

/** Classifies only rendered controls; it does not infer run state from browser or server data. */
export function classifyPostStartSurface(counts: PostStartSurfaceCounts): PostStartSurface {
  if (counts.inlineErrors > 0) return "error";
  if (counts.radios > 0) return "run";
  if (counts.freshPractice) return "fresh-practice";
  return "pending";
}

/** Classifies rendered final controls without reading feedback or response content. */
export function classifyFinalSurface(counts: FinalSurfaceCounts): FinalSurface {
  if (counts.inlineErrors > 0) return "error";
  if (counts.freshPractice) return "fresh-practice";
  if (counts.radios > 0) return "run";
  if (counts.continueVisible || counts.feedbackVisible) return "feedback";
  if (counts.closedComplete) return "closed";
  if (counts.neutralComplete) return "neutral";
  return "pending";
}

/** Feedback can persist briefly after Continue, so only stable outcomes end the final wait. */
export function isFinalSurfaceTerminal(surface: FinalSurface): boolean {
  return surface !== "feedback" && surface !== "pending" && surface !== "neutral";
}
