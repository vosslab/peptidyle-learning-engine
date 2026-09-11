// route_scope_provider_fixtures.ts - type-checked presentation records for
// RouteScopeProvider tests.

import type { AssignmentAttemptSummaryResponse, CourseRouteView } from "../../src/api/contracts";
import type { AssignmentAttempt } from "../../generated/api/AssignmentAttempt";
import type { StudentAssignmentProgress } from "../../generated/api/StudentAssignmentProgress";
import type { StudentAssignmentAttemptContext } from "../../src/api/assignment_attempt_navigation";

export function courseRouteData(reference: string): CourseRouteView {
  return {
    summary: {
      id: `course-${reference}`,
      reference,
      title: `Course ${reference}`,
      term: { startDate: "2026-01-12", endDate: "2026-05-08", timeZone: "America/Chicago" },
      role: "student",
    },
    appearance: { theme: "grass", banner: null },
  } satisfies CourseRouteView;
}

function assignmentAttempt(reference: string): AssignmentAttempt {
  return {
    id: `attempt-${reference}`,
    reference,
    studentRecord: "student-record-1",
    assignment: "assignment-1",
    assignmentRevision: { assignment: "A-1", revision_number: "1" },
    attemptNumber: 1,
    startedAt: 1_700_000_000_000,
    completedAt: null,
    score: null,
    questionPoolReuseRule: "reuseSelection",
    questionVariationRule: "reuseVariation",
  };
}

function assignmentProgress(): StudentAssignmentProgress {
  return {
    assignment_progress: {
      completed_assignment_attempt_count: 0,
      total_question_attempts: 0,
      last_activity_at: null,
    },
    student_assignment_grade: {
      score_state: "no_activity",
      assignment_scoring_state: "current",
      current_score: null,
      best_score: null,
      latest_score: null,
    },
  };
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

/** Complete direct-consumer record for an Attempt summary route. */
export function assignmentAttemptSummaryData(reference: string): AssignmentAttemptSummaryResponse {
  return {
    course: courseRouteData(reference),
    assignmentAttempt: assignmentAttempt("R-1"),
    summary: assignmentProgress(),
    outcomes: { items: [], nextCursor: null },
  } satisfies AssignmentAttemptSummaryResponse;
}
