// student_progress.ts - server-derived Student score copy with no policy inference.

import type { AssignmentProgress } from "../generated/api/AssignmentProgress";

import { formatPercentScore } from "./score_format";

/** Human-readable aggregate progress from the server's key-free projection. */
export function studentProgressSummary(progress: AssignmentProgress): string {
  if (progress.assignment_scoring_state === "recalculating")
    return "Scores are recalculating. Recorded work is safe.";
  if (progress.assignment_scoring_state === "failed")
    return "Scores are temporarily unavailable. Recorded work is safe.";
  switch (progress.score_state) {
    case "no_activity":
      return "No score yet. Submit a response to record scored progress.";
    case "withheld":
      return `Score is currently unavailable. ${progress.completed_assignment_attempt_count} completed ${
        progress.completed_assignment_attempt_count === 1 ? "Assignment Attempt" : "Assignment Attempts"
      } recorded.`;
    case "available": {
      const scores = [
        progress.current_score === null
          ? null
          : `Current ${formatPercentScore(progress.current_score)}`,
        progress.latest_score === null
          ? null
          : `Latest ${formatPercentScore(progress.latest_score)}`,
        progress.best_score === null ? null : `Best ${formatPercentScore(progress.best_score)}`,
      ].filter((score): score is string => score !== null);
      return scores.length === 0
        ? "No score yet. Your completed work is recorded."
        : `Score available: ${scores.join(", ")}.`;
    }
  }
}

/** Formats only an available score; withheld totals stay out of the score-detail UI. */
export function studentScoreValue(score: number | null): string {
  return score === null ? "No score yet" : formatPercentScore(score);
}
