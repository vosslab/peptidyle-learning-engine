// route_scope_provider_fixtures.ts - type-checked presentation records for
// RouteScopeProvider tests.

import type { CourseRouteView } from "../../src/api/contracts";
import type { CourseClassification } from "../../generated/api/CourseClassification";
import type { StudentAssessmentAttemptContext } from "../../src/api/assessment_attempt_navigation";
import type { StudentAssessmentAttemptHistory } from "../../src/api/assessment_attempt_history";

const FIXTURE_CLASSIFICATION = {
  disciplineUuid: "018f5e7d-01b6-7c14-8a0b-4bfef6390d6d",
  subjectUuid: null,
  topicUuid: null,
  subtopicUuid: null,
  tags: [],
} satisfies CourseClassification;

export function courseRouteData(courseInstanceId: string): CourseRouteView {
  return {
    summary: {
      id: courseInstanceId,
      shortName: `CRS ${courseInstanceId}`,
      longName: `Course ${courseInstanceId}: Molecular Biology`,
      classification: FIXTURE_CLASSIFICATION,
      term: { startDate: "2026-01-12", endDate: "2026-05-08" },
      role: "student",
    },
    appearance: { theme: "grass", banner: null },
  } satisfies CourseRouteView;
}

/** UUID-free display context for the live Student Assessment Attempt route. */
export function assignmentAttemptContext(
  courseInstanceId: string,
): StudentAssessmentAttemptContext {
  return {
    assessmentAttemptId: "00000000-0000-0000-0000-000000000001",
    attemptNumber: 1,
    displayTimeZone: "America/Chicago",
    expiresAt: 1_768_507_200_000,
    timerRemainingMilliseconds: 1_800_000,
    course: {
      id: courseInstanceId,
      shortName: `CRS ${courseInstanceId}`,
      longName: `Course ${courseInstanceId}: Molecular Biology`,
      theme: "grass",
    },
    assessment: {
      id: "A9D2RX5AF",
      assessmentType: "practice_question_assignment",
      title: "Assessment one",
    },
  } satisfies StudentAssessmentAttemptContext;
}

/** Complete direct-consumer record for an Attempt history route. */
export function assignmentAttemptHistoryData(
  courseInstanceId: string,
): StudentAssessmentAttemptHistory {
  return {
    assessmentAttemptId: "00000000-0000-0000-0000-000000000001",
    attemptNumber: 1,
    course: {
      id: courseInstanceId,
      shortName: `CRS ${courseInstanceId}`,
      longName: `Course ${courseInstanceId}: Molecular Biology`,
      theme: "grass",
    },
    assessment: {
      id: "A9D2RX5AF",
      assessmentType: "practice_question_assignment",
      title: "Assessment one",
    },
    state: "submitted",
    questions: [],
  } satisfies StudentAssessmentAttemptHistory;
}
