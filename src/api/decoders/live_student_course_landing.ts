// Strict decoding for the minimal current Student Course Landing projection.

import type {
  LiveStudentAssessmentLandingSummary,
  LiveStudentCourseInvitationSummary,
  LiveStudentCourseLandingSummary,
} from "../live_student_course_landing";
import type { AssessmentAttemptCompletion } from "../../../generated/api/AssessmentAttemptCompletion";
import {
  DecodeError,
  decodeArray,
  decodeFiniteNumber,
  decodeNonnegativeInteger,
  decodeNullable,
  decodePositiveInteger,
  decodeRecord,
  decodeStringEnum,
} from "../decoder";
import {
  decodeAssessmentReference,
  decodeAssessmentTitle,
  decodeCourseInstanceReference,
  decodeCourseName,
  field,
  requireOnlyFields,
} from "./shared";
import { decodeStudentAssessmentDecision } from "./student_assessment_decision";

const ASSIGNMENT_ATTEMPT_COMPLETIONS = [
  "inProgress",
  "completed",
] as const satisfies ReadonlyArray<AssessmentAttemptCompletion>;

function decodeCourseSummary(value: unknown, path: string): LiveStudentCourseLandingSummary {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["reference", "shortName", "longName"]);
  return {
    reference: decodeCourseInstanceReference(field(record, "reference", path), `${path}.reference`),
    shortName: decodeCourseName(field(record, "shortName", path), `${path}.shortName`),
    longName: decodeCourseName(field(record, "longName", path), `${path}.longName`),
  };
}

function decodeInvitationSummary(value: unknown, path: string): LiveStudentCourseInvitationSummary {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["reference", "shortName", "longName"]);
  return {
    reference: decodeCourseInstanceReference(field(record, "reference", path), `${path}.reference`),
    shortName: decodeCourseName(field(record, "shortName", path), `${path}.shortName`),
    longName: decodeCourseName(field(record, "longName", path), `${path}.longName`),
  };
}

function decodeAssessmentSummary(
  value: unknown,
  path: string,
): LiveStudentAssessmentLandingSummary {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "reference",
    "title",
    "decision",
    "assessmentAttemptNumber",
    "assessmentAttemptCompletion",
    "gradedQuestionCount",
    "questionCount",
    "score",
  ]);
  const assessmentAttemptNumber = decodeNullable(
    field(record, "assessmentAttemptNumber", path),
    `${path}.assessmentAttemptNumber`,
    decodePositiveInteger,
  );
  const assessmentAttemptCompletion = decodeNullable(
    field(record, "assessmentAttemptCompletion", path),
    `${path}.assessmentAttemptCompletion`,
    (candidate, candidatePath) =>
      decodeStringEnum(candidate, candidatePath, ASSIGNMENT_ATTEMPT_COMPLETIONS),
  );
  const gradedQuestionCount = decodeNonnegativeInteger(
    field(record, "gradedQuestionCount", path),
    `${path}.gradedQuestionCount`,
  );
  const questionCount = decodePositiveInteger(
    field(record, "questionCount", path),
    `${path}.questionCount`,
  );
  const scoreValue = record.score;
  let score: { readonly pointsEarned: number; readonly pointsPossible: number } | undefined;
  if (scoreValue !== undefined) {
    const scoreRecord = decodeRecord(scoreValue, `${path}.score`);
    requireOnlyFields(scoreRecord, `${path}.score`, ["pointsEarned", "pointsPossible"]);
    const pointsEarned = decodeFiniteNumber(
      field(scoreRecord, "pointsEarned", `${path}.score`),
      `${path}.score.pointsEarned`,
    );
    const pointsPossible = decodeFiniteNumber(
      field(scoreRecord, "pointsPossible", `${path}.score`),
      `${path}.score.pointsPossible`,
    );
    if (pointsEarned < 0 || pointsPossible < pointsEarned) {
      throw new DecodeError(`${path}.score`, "an ordered nonnegative Assessment score");
    }
    score = { pointsEarned, pointsPossible };
  }
  if (
    gradedQuestionCount > questionCount ||
    (score !== undefined && gradedQuestionCount !== questionCount) ||
    (assessmentAttemptCompletion === null &&
      (assessmentAttemptNumber !== null || gradedQuestionCount !== 0 || score !== undefined)) ||
    (assessmentAttemptCompletion !== null && assessmentAttemptNumber === null)
  ) {
    throw new DecodeError(path, "internally consistent self-only Assessment progress");
  }
  return {
    reference: decodeAssessmentReference(field(record, "reference", path), `${path}.reference`),
    title: decodeAssessmentTitle(field(record, "title", path), `${path}.title`),
    decision: decodeStudentAssessmentDecision(field(record, "decision", path), `${path}.decision`),
    assessmentAttemptNumber,
    assessmentAttemptCompletion,
    gradedQuestionCount,
    questionCount,
    ...(score === undefined ? {} : { score }),
  };
}

/** Rejects anything beyond the current Student's minimal Course landing projection. */
export function decodeLiveStudentCourseLandings(
  value: unknown,
  path = "response",
): ReadonlyArray<LiveStudentCourseLandingSummary> {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["courses"]);
  return decodeArray(field(record, "courses", path), `${path}.courses`, decodeCourseSummary);
}

/** Rejects anything beyond the current Student's minimal pending-invitation projection. */
export function decodeLiveStudentCourseInvitations(
  value: unknown,
  path = "response",
): ReadonlyArray<LiveStudentCourseInvitationSummary> {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["invitations"]);
  return decodeArray(
    field(record, "invitations", path),
    `${path}.invitations`,
    decodeInvitationSummary,
  );
}

/** Rejects anything beyond the current Student's minimal Assessment landing projection. */
export function decodeLiveStudentAssessmentLandings(
  value: unknown,
  path = "response",
): ReadonlyArray<LiveStudentAssessmentLandingSummary> {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["assessments"]);
  return decodeArray(
    field(record, "assessments", path),
    `${path}.assessments`,
    decodeAssessmentSummary,
  );
}
