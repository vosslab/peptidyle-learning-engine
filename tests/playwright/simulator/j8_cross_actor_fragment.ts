// j8_cross_actor_fragment.ts - public-only learner/instructor browser handoff.

import type {
  J4JourneyFragment,
  J5JourneyFragment,
  J8JourneyFragment,
} from "./visible_outcome_report";

export function passedJ8CrossActorFragment(
  j4: J4JourneyFragment,
  j5: J5JourneyFragment,
  elapsedMs: number,
): J8JourneyFragment {
  if (
    j4.courseId !== j5.courseId ||
    j4.masteryAssignmentId !== j5.assignmentId ||
    !Number.isSafeInteger(elapsedMs) ||
    elapsedMs < 0 ||
    elapsedMs > 30 * 60 * 1000
  )
    throw new Error("J8 requires matching public learner and instructor observations");
  return {
    schemaVersion: 1,
    journey: "J8",
    status: "PASS",
    elapsedMs,
    courseId: j5.courseId,
    assignmentId: j5.assignmentId,
    visibleOutcomeCodes: ["visible_instructor_gradebook", "visible_learner_completion"],
    diagnostics: [],
  };
}
