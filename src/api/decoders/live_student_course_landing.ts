// Strict decoding for the minimal current Student Course Landing projection.

import type {
  LiveStudentAssessmentLandingSummary,
  AssessmentGradeContribution,
  LiveStudentCourseInvitationSummary,
  LiveStudentCourseLandingSummary,
  StudentCourseActiveAttempt,
  StudentCourseProgressAssessment,
  StudentLatestFeedback,
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
  decodeAssessmentId,
  decodeAssessmentTitle,
  decodeCourseInstanceId,
  decodeCourseName,
  field,
  requireOnlyFields,
} from "./shared";
import { decodeStudentAssessmentDecision } from "./student_assessment_decision";
import { decodeCourseTerm } from "./course_term";
import { parseAssessmentAttemptId } from "../../navigation/public_route";

const ASSESSMENT_ATTEMPT_COMPLETIONS = [
  "inProgress",
  "completed",
] as const satisfies ReadonlyArray<AssessmentAttemptCompletion>;

function decodeCourseSummary(value: unknown, path: string): LiveStudentCourseLandingSummary {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["id", "shortName", "longName"]);
  return {
    id: decodeCourseInstanceId(field(record, "id", path), `${path}.id`),
    shortName: decodeCourseName(field(record, "shortName", path), `${path}.shortName`),
    longName: decodeCourseName(field(record, "longName", path), `${path}.longName`),
  };
}

function decodeInvitationSummary(value: unknown, path: string): LiveStudentCourseInvitationSummary {
  const record = decodeRecord(value, path);
  // ASVS 1.5.2/8.2.3: this closed projection contains no private identity fields.
  requireOnlyFields(record, path, ["id", "shortName", "longName", "instructorDisplayName", "term"]);
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
    id: decodeCourseInstanceId(field(record, "id", path), `${path}.id`),
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
    "id",
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
      decodeStringEnum(candidate, candidatePath, ASSESSMENT_ATTEMPT_COMPLETIONS),
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
    id: decodeAssessmentId(field(record, "id", path), `${path}.id`),
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

function decodeProgressAssessment(value: unknown, path: string): StudentCourseProgressAssessment {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "id",
    "title",
    "assessmentType",
    "assessmentAttemptCount",
    "submittedAssessmentAttemptCount",
    "latestAssessmentAttemptNumber",
    "latestAssessmentAttemptCompletion",
    "latestActivityAt",
    "assessmentScore",
    "assessmentScoreIsLatestAttempt",
  ]);
  const assessmentAttemptCount = decodeNonnegativeInteger(
    field(record, "assessmentAttemptCount", path),
    `${path}.assessmentAttemptCount`,
  );
  const submittedAssessmentAttemptCount = decodeNonnegativeInteger(
    field(record, "submittedAssessmentAttemptCount", path),
    `${path}.submittedAssessmentAttemptCount`,
  );
  const latestAssessmentAttemptNumber = decodeNullable(
    field(record, "latestAssessmentAttemptNumber", path),
    `${path}.latestAssessmentAttemptNumber`,
    decodePositiveInteger,
  );
  const latestAssessmentAttemptCompletion = decodeNullable(
    field(record, "latestAssessmentAttemptCompletion", path),
    `${path}.latestAssessmentAttemptCompletion`,
    (candidate, candidatePath) =>
      decodeStringEnum(candidate, candidatePath, ASSESSMENT_ATTEMPT_COMPLETIONS),
  );
  const latestActivityAt = decodeNullable(
    field(record, "latestActivityAt", path),
    `${path}.latestActivityAt`,
    decodeNonnegativeInteger,
  );
  const assessmentScore =
    record.assessmentScore === undefined
      ? undefined
      : decodeAssessmentGradeContribution(record.assessmentScore, `${path}.assessmentScore`);
  const assessmentScoreIsLatestAttempt =
    record.assessmentScoreIsLatestAttempt === undefined
      ? undefined
      : decodeBoolean(
          record.assessmentScoreIsLatestAttempt,
          `${path}.assessmentScoreIsLatestAttempt`,
        );
  if (
    submittedAssessmentAttemptCount > assessmentAttemptCount ||
    (assessmentAttemptCount === 0 &&
      (latestAssessmentAttemptNumber !== null ||
        latestAssessmentAttemptCompletion !== null ||
        latestActivityAt !== null ||
        submittedAssessmentAttemptCount !== 0 ||
        assessmentScore !== undefined ||
        assessmentScoreIsLatestAttempt !== undefined)) ||
    (assessmentAttemptCount > 0 &&
      (latestAssessmentAttemptNumber === null ||
        latestAssessmentAttemptNumber > assessmentAttemptCount ||
        latestAssessmentAttemptCompletion === null ||
        latestActivityAt === null)) ||
    (assessmentScore !== undefined && submittedAssessmentAttemptCount === 0) ||
    (assessmentScore !== undefined && assessmentScoreIsLatestAttempt === undefined) ||
    (assessmentScore === undefined && assessmentScoreIsLatestAttempt !== undefined)
  ) {
    throw new DecodeError(path, "internally consistent self-only Course Progress");
  }
  return {
    id: decodeAssessmentId(field(record, "id", path), `${path}.id`),
    title: decodeAssessmentTitle(field(record, "title", path), `${path}.title`),
    assessmentType: decodeStringEnum(
      field(record, "assessmentType", path),
      `${path}.assessmentType`,
      ASSESSMENT_TYPE_VALUES,
    ),
    assessmentAttemptCount,
    submittedAssessmentAttemptCount,
    latestAssessmentAttemptNumber,
    latestAssessmentAttemptCompletion,
    latestActivityAt,
    ...(assessmentScore === undefined ? {} : { assessmentScore }),
    ...(assessmentScoreIsLatestAttempt === undefined ? {} : { assessmentScoreIsLatestAttempt }),
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

/** Rejects any fields outside the authenticated Student's Course Progress projection. */
export function decodeStudentCourseProgress(
  value: unknown,
  path = "response",
): ReadonlyArray<StudentCourseProgressAssessment> {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["assessments"]);
  return decodeArray(
    field(record, "assessments", path),
    `${path}.assessments`,
    decodeProgressAssessment,
  );
}

/** Rejects values outside the safe fixed Coursework Active Attempt shortcut projection. */
export function decodeStudentCourseActiveAttempt(
  value: unknown,
  path = "response",
): StudentCourseActiveAttempt {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["assessmentAttemptId", "startedAt", "latestActivityAt"]);
  const rawAttemptId = decodeNullable(
    field(record, "assessmentAttemptId", path),
    `${path}.assessmentAttemptId`,
    decodeString,
  );
  const assessmentAttemptId = rawAttemptId === null ? null : parseAssessmentAttemptId(rawAttemptId);
  if (rawAttemptId !== null && assessmentAttemptId === null) {
    throw new DecodeError(`${path}.assessmentAttemptId`, "a canonical Assessment Attempt UUID");
  }
  const startedAt = decodeNullable(
    field(record, "startedAt", path),
    `${path}.startedAt`,
    decodeNonnegativeInteger,
  );
  const latestActivityAt = decodeNullable(
    field(record, "latestActivityAt", path),
    `${path}.latestActivityAt`,
    decodeNonnegativeInteger,
  );
  if (
    (assessmentAttemptId === null && (startedAt !== null || latestActivityAt !== null)) ||
    (assessmentAttemptId !== null && (startedAt === null || latestActivityAt === null))
  ) {
    throw new DecodeError(path, "an Attempt ID with both timestamps, or three explicit nulls");
  }
  return { assessmentAttemptId, startedAt, latestActivityAt };
}

/** Rejects values outside the signed-in Student's Latest Feedback shortcut projection. */
export function decodeStudentLatestFeedback(
  value: unknown,
  path = "response",
): StudentLatestFeedback {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["assessmentAttemptId"]);
  const rawAttemptId = decodeNullable(
    field(record, "assessmentAttemptId", path),
    `${path}.assessmentAttemptId`,
    decodeString,
  );
  const assessmentAttemptId = rawAttemptId === null ? null : parseAssessmentAttemptId(rawAttemptId);
  if (rawAttemptId !== null && assessmentAttemptId === null) {
    throw new DecodeError(`${path}.assessmentAttemptId`, "a canonical Assessment Attempt UUID");
  }
  return { assessmentAttemptId };
}
