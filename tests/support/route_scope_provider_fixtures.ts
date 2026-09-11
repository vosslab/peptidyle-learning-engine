// route_scope_provider_fixtures.ts - type-checked presentation records for
// RouteScopeProvider tests.

import type { CourseRouteView } from "../../src/api/contracts";
import type { StudentAssignmentAttemptContext } from "../../src/api/assignment_attempt_navigation";
import type { StudentAssignmentAttemptHistory } from "../../src/api/assignment_attempt_history";

export function courseRouteData(reference: string): CourseRouteView {
  return {
    summary: {
      id: `course-${reference}`,
      reference,
      title: `Course ${reference}`,
      term: { startDate: "2026-01-12", endDate: "2026-05-08" },
      role: "student",
    },
    appearance: { theme: "grass", banner: null },
  } satisfies CourseRouteView;
}

/** UUID-free display context for the live Student Assignment Attempt route. */
export function assignmentAttemptContext(reference: string): StudentAssignmentAttemptContext {
  return {
    assignmentAttempt: "R-1",
    attemptNumber: 1,
    timerRemainingMilliseconds: 1_800_000,
    course: {
      reference,
      title: `Course ${reference}`,
      theme: "grass",
    },
    assignment: { reference: "A-1", title: "Assignment one" },
  } satisfies StudentAssignmentAttemptContext;
}

/** Complete direct-consumer record for an Attempt history route. */
export function assignmentAttemptHistoryData(reference: string): StudentAssignmentAttemptHistory {
  return {
    assignmentAttempt: "R-1",
    attemptNumber: 1,
    course: { reference, title: `Course ${reference}`, theme: "grass" },
    assignment: { reference: "A-1", title: "Assignment one" },
    state: "submitted",
    questions: [],
  } satisfies StudentAssignmentAttemptHistory;
}
