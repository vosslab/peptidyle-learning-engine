// Strict decoding for the minimal current Student Course Landing projection.

import type {
  LiveStudentAssignmentLandingSummary,
  LiveStudentCourseInvitationSummary,
  LiveStudentCourseLandingSummary,
} from "../live_student_course_landing";
import type { AssignmentAttemptCompletion } from "../../../generated/api/AssignmentAttemptCompletion";
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
  decodeAssignmentReference,
  decodeAssignmentTitle,
  decodeCourseInstanceReference,
  decodeCourseName,
  field,
  requireOnlyFields,
} from "./shared";

const ASSIGNMENT_ATTEMPT_COMPLETIONS = [
  "inProgress",
  "completed",
] as const satisfies ReadonlyArray<AssignmentAttemptCompletion>;

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

function decodeAssignmentSummary(
  value: unknown,
  path: string,
): LiveStudentAssignmentLandingSummary {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "reference",
    "title",
    "assignmentAttemptNumber",
    "assignmentAttemptCompletion",
    "gradedQuestionCount",
    "questionCount",
    "score",
  ]);
  const assignmentAttemptNumber = decodeNullable(
    field(record, "assignmentAttemptNumber", path),
    `${path}.assignmentAttemptNumber`,
    decodePositiveInteger,
  );
  const assignmentAttemptCompletion = decodeNullable(
    field(record, "assignmentAttemptCompletion", path),
    `${path}.assignmentAttemptCompletion`,
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
      throw new DecodeError(`${path}.score`, "an ordered nonnegative Assignment score");
    }
    score = { pointsEarned, pointsPossible };
  }
  if (
    gradedQuestionCount > questionCount ||
    (score !== undefined && gradedQuestionCount !== questionCount) ||
    (assignmentAttemptCompletion === null &&
      (assignmentAttemptNumber !== null || gradedQuestionCount !== 0 || score !== undefined)) ||
    (assignmentAttemptCompletion !== null && assignmentAttemptNumber === null)
  ) {
    throw new DecodeError(path, "internally consistent self-only Assignment progress");
  }
  return {
    reference: decodeAssignmentReference(field(record, "reference", path), `${path}.reference`),
    title: decodeAssignmentTitle(field(record, "title", path), `${path}.title`),
    assignmentAttemptNumber,
    assignmentAttemptCompletion,
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

/** Rejects anything beyond the current Student's minimal Assignment landing projection. */
export function decodeLiveStudentAssignmentLandings(
  value: unknown,
  path = "response",
): ReadonlyArray<LiveStudentAssignmentLandingSummary> {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["assignments"]);
  return decodeArray(
    field(record, "assignments", path),
    `${path}.assignments`,
    decodeAssignmentSummary,
  );
}
