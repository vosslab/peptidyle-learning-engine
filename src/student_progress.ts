// student_progress.ts - server-derived Student score copy with no policy inference.

import type { StudentAssignmentProgress } from "../generated/api/StudentAssignmentProgress";

import { formatPercentScore } from "./score_format";

/** Human-readable aggregate progress from the server's key-free Student Assignment Progress. */
export function studentProgressSummary(progress: StudentAssignmentProgress): string {
  const grade = progress.student_assignment_grade;
  const activity = progress.assignment_progress;
  if (grade.assignment_scoring_state === "recalculating")
    return "Scores are recalculating. Recorded work is safe.";
  if (grade.assignment_scoring_state === "failed")
    return "Scores are temporarily unavailable. Recorded work is safe.";
  switch (grade.score_state) {
    case "no_activity":
      return "No score yet. Submit a response to record scored progress.";
    case "withheld":
      return `Score is currently unavailable. ${activity.completed_assignment_attempt_count} completed ${
        activity.completed_assignment_attempt_count === 1
          ? "Assignment Attempt"
          : "Assignment Attempts"
      } recorded.`;
    case "available": {
      const scores = [
        grade.current_score === null ? null : `Current ${formatPercentScore(grade.current_score)}`,
        grade.latest_score === null ? null : `Latest ${formatPercentScore(grade.latest_score)}`,
        grade.best_score === null ? null : `Best ${formatPercentScore(grade.best_score)}`,
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
