// Strict decoding for the minimal current Student Course Landing projection.

import type {
  LiveStudentAssessmentLandingSummary,
  AssessmentGradeContribution,
  LiveStudentCourseInvitationSummary,
  LiveStudentCourseLandingSummary,
} from "../live_student_course_landing";
import type { AssessmentAttemptCompletion } from "../../../generated/api/AssessmentAttemptCompletion";
import { ASSESSMENT_TYPE_VALUES } from "../../../generated/api/AssessmentType";
import {
  DecodeError,
  decodeArray,
  decodeBoolean,
  decodeFiniteNumber,
  decodeNonnegativeInteger,
  decodeNullable,
  decodePositiveInteger,
  decodeRecord,
  decodeStringEnum,
  decodeString,
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
import { decodeCourseTerm } from "./course_term";

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
  // ASVS 1.5.2/8.2.3: this closed projection contains no private identity fields.
  requireOnlyFields(record, path, [
    "reference",
    "shortName",
    "longName",
    "instructorDisplayName",
    "term",
  ]);
  const instructorDisplayName = decodeString(
    field(record, "instructorDisplayName", path),
    `${path}.instructorDisplayName`,
  );
  if (
    instructorDisplayName !== instructorDisplayName.trim() ||
    Array.from(instructorDisplayName).length === 0 ||
    Array.from(instructorDisplayName).length > 200 ||
    /[\p{Cc}]/u.test(instructorDisplayName)
  ) {
    throw new DecodeError(`${path}.instructorDisplayName`, "one verified Instructor display name");
  }
  return {
    reference: decodeCourseInstanceReference(field(record, "reference", path), `${path}.reference`),
    shortName: decodeCourseName(field(record, "shortName", path), `${path}.shortName`),
    longName: decodeCourseName(field(record, "longName", path), `${path}.longName`),
    instructorDisplayName,
    term: decodeCourseTerm(field(record, "term", path), `${path}.term`),
  };
}

function decodeAssessmentGradeContribution(
  value: unknown,
  path: string,
): AssessmentGradeContribution {
  // ASVS 1.5.1/2.2.1: allow-list the complete finite pair at the browser boundary.
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["pointsEarned", "pointsPossible"]);
  const pointsEarned = decodeFiniteNumber(
    field(record, "pointsEarned", path),
    `${path}.pointsEarned`,
  );
  const pointsPossible = decodeFiniteNumber(
    field(record, "pointsPossible", path),
    `${path}.pointsPossible`,
  );
  if (pointsEarned < 0 || pointsPossible < 0) {
    throw new DecodeError(path, "a complete nonnegative Assessment grade contribution");
  }
  return { pointsEarned, pointsPossible };
}

function decodeAssessmentSummary(
  value: unknown,
  path: string,
): LiveStudentAssessmentLandingSummary {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "reference",
    "title",
    "assessmentType",
    "decision",
    "assessmentAttemptNumber",
    "assessmentAttemptCompletion",
    "canResumeAssessmentAttempt",
    "gradedQuestionCount",
    "savedQuestionCount",
    "questionCount",
    "assessmentScore",
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
  // ASVS 2.2.1/2.2.3: saved progress is bounded by the retained issued total.
  const savedQuestionCount = decodeNonnegativeInteger(
    field(record, "savedQuestionCount", path),
    `${path}.savedQuestionCount`,
  );
  const assessmentScoreValue = record.assessmentScore;
  const assessmentScore =
    assessmentScoreValue === undefined
      ? undefined
      : decodeAssessmentGradeContribution(assessmentScoreValue, `${path}.assessmentScore`);
  if (
    gradedQuestionCount > questionCount ||
    savedQuestionCount > questionCount ||
    (assessmentAttemptCompletion === null &&
      (assessmentAttemptNumber !== null ||
        gradedQuestionCount !== 0 ||
        savedQuestionCount !== 0)) ||
    (assessmentAttemptCompletion !== null && assessmentAttemptNumber === null)
  ) {
    throw new DecodeError(path, "internally consistent self-only Assessment progress");
  }
  return {
    reference: decodeAssessmentReference(field(record, "reference", path), `${path}.reference`),
    title: decodeAssessmentTitle(field(record, "title", path), `${path}.title`),
    assessmentType: decodeStringEnum(
      field(record, "assessmentType", path),
      `${path}.assessmentType`,
      ASSESSMENT_TYPE_VALUES,
    ),
    decision: decodeStudentAssessmentDecision(field(record, "decision", path), `${path}.decision`),
    assessmentAttemptNumber,
    assessmentAttemptCompletion,
    canResumeAssessmentAttempt: decodeBoolean(
      field(record, "canResumeAssessmentAttempt", path),
      `${path}.canResumeAssessmentAttempt`,
    ),
    gradedQuestionCount,
    savedQuestionCount,
    questionCount,
    ...(assessmentScore === undefined ? {} : { assessmentScore }),
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
