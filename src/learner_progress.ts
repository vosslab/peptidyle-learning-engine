// learner_progress.ts - server-derived learner score copy with no policy inference.

import type { LearnerAssignmentProgress } from "../generated/api/LearnerAssignmentProgress";

import { formatPercentScore } from "./score_format";

/** Human-readable aggregate progress from the server's key-free projection. */
export function learnerProgressSummary(progress: LearnerAssignmentProgress): string {
  switch (progress.scoreState) {
    case "noActivity":
      return "No score yet. Submit a response to record scored progress.";
    case "withheld":
      return `Score is currently unavailable. ${progress.completedRunCount} completed ${
        progress.completedRunCount === 1 ? "run" : "runs"
      } recorded.`;
    case "available": {
      const scores = [
        progress.currentScore === null
          ? null
          : `Current ${formatPercentScore(progress.currentScore)}`,
        progress.latestScore === null ? null : `Latest ${formatPercentScore(progress.latestScore)}`,
        progress.bestScore === null ? null : `Best ${formatPercentScore(progress.bestScore)}`,
      ].filter((score): score is string => score !== null);
      return scores.length === 0
        ? "No score yet. Your completed work is recorded."
        : `Score available: ${scores.join(", ")}.`;
    }
  }
}

/** Formats only an available score; withheld totals stay out of the score-detail UI. */
export function learnerScoreValue(score: number | null): string {
  return score === null ? "No score yet" : formatPercentScore(score);
}
