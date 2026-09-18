import type {
  StudentAssessmentAttemptContext,
  StudentAssessmentAttemptPresentation,
  StudentAssessmentAttemptProgress,
  StudentAssessmentAttemptResponseSaveAcknowledgement,
  StudentAssessmentAttemptResponseState,
  StudentAssessmentAttemptSubmissionResult,
} from "../assessment_attempt_navigation";
import {
  DecodeError,
  decodeArray,
  decodeNonemptyString,
  decodeNonnegativeInteger,
  decodePositiveInteger,
  decodeRecord,
  decodeStringEnum,
} from "../decoder";
import { decodeCourseName, field, requireOnlyFields } from "./shared";
import {
  type AssessmentAttemptRouteReference,
  type AssessmentRouteReference,
  type CourseInstanceRouteReference,
  parseAssessmentAttemptReference,
  parseAssessmentId,
  parseCourseInstanceId,
} from "../../navigation/public_route";
import { COURSE_THEME_VALUES } from "../../../generated/api/CourseTheme";
import { decodeAccountTimeZone } from "./student_assessment_decision";
import { decodeStudentQuestionPresentation } from "./presentation_delivery";
import { decodeStudentResponse } from "./question_delivery";

function state(value: unknown, path: string): StudentAssessmentAttemptResponseState {
  if (value === "unanswered" || value === "saved" || value === "submitted" || value === "closed")
    return value;
  throw new DecodeError(path, "an answer-free response state");
}

function decodeAssessmentAttemptReference(
  value: unknown,
  path: string,
): AssessmentAttemptRouteReference {
  if (typeof value !== "string") throw new DecodeError(path, "an Assessment Attempt UUID");
  const parsed = parseAssessmentAttemptReference(value);
  if (parsed === null) throw new DecodeError(path, "an Assessment Attempt UUID");
  return parsed;
}

function decodeCourseReference(value: unknown, path: string): CourseInstanceRouteReference {
  if (typeof value !== "string") throw new DecodeError(path, "a Course CI reference");
  const parsed = parseCourseInstanceId(value);
  if (parsed === null) throw new DecodeError(path, "a Course CI reference");
  return parsed;
}

function decodeAssessmentId(value: unknown, path: string): AssessmentRouteReference {
  if (typeof value !== "string") throw new DecodeError(path, "an Assessment A reference");
  const parsed = parseAssessmentId(value);
  if (parsed === null) throw new DecodeError(path, "an Assessment A reference");
  return parsed;
}

/** Strict UUID-free Student Attempt context used by the persistent route scope. */
export function decodeStudentAssessmentAttemptContext(
  value: unknown,
  path = "response",
): StudentAssessmentAttemptContext {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "assessmentAttempt",
    "attemptNumber",
    "displayTimeZone",
    "expiresAt",
    "timerRemainingMilliseconds",
    "course",
    "assessment",
  ]);
  const course = decodeRecord(field(record, "course", path), `${path}.course`);
  requireOnlyFields(course, `${path}.course`, ["reference", "shortName", "longName", "theme"]);
  const assessment = decodeRecord(field(record, "assessment", path), `${path}.assessment`);
  requireOnlyFields(assessment, `${path}.assessment`, ["reference", "title"]);
  const remaining = field(record, "timerRemainingMilliseconds", path);
  const expiresAt = field(record, "expiresAt", path);
  return {
    assessmentAttempt: decodeAssessmentAttemptReference(
      field(record, "assessmentAttempt", path),
      `${path}.assessmentAttempt`,
    ),
    attemptNumber: decodePositiveInteger(
      field(record, "attemptNumber", path),
      `${path}.attemptNumber`,
    ),
    displayTimeZone: decodeAccountTimeZone(
      field(record, "displayTimeZone", path),
      `${path}.displayTimeZone`,
    ),
    expiresAt: expiresAt === null ? null : decodeNonnegativeInteger(expiresAt, `${path}.expiresAt`),
    timerRemainingMilliseconds:
      remaining === null
        ? null
        : decodeNonnegativeInteger(remaining, `${path}.timerRemainingMilliseconds`),
    course: {
      reference: decodeCourseReference(
        field(course, "reference", `${path}.course`),
        `${path}.course.reference`,
      ),
      shortName: decodeCourseName(
        field(course, "shortName", `${path}.course`),
        `${path}.course.shortName`,
      ),
      longName: decodeCourseName(
        field(course, "longName", `${path}.course`),
        `${path}.course.longName`,
      ),
      theme: decodeStringEnum(
        field(course, "theme", `${path}.course`),
        `${path}.course.theme`,
        COURSE_THEME_VALUES,
      ),
    },
    assessment: {
      reference: decodeAssessmentId(
        field(assessment, "reference", `${path}.assessment`),
        `${path}.assessment.reference`,
      ),
      title: decodeNonemptyString(
        field(assessment, "title", `${path}.assessment`),
        `${path}.assessment.title`,
      ),
    },
  };
}

export function decodeStudentAssessmentAttemptProgress(
  value: unknown,
  path = "response",
): StudentAssessmentAttemptProgress {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "assessmentAttempt",
    "questionCount",
    "recommendedPosition",
    "positions",
  ]);
  const questionCount = decodePositiveInteger(
    field(record, "questionCount", path),
    `${path}.questionCount`,
  );
  const recommended = field(record, "recommendedPosition", path);
  const recommendedPosition =
    recommended === null ? null : decodePositiveInteger(recommended, `${path}.recommendedPosition`);
  const positions = decodeArray(
    field(record, "positions", path),
    `${path}.positions`,
    (item, itemPath) => {
      const position = decodeRecord(item, itemPath);
      requireOnlyFields(position, itemPath, ["position", "responseState"]);
      return {
        position: decodePositiveInteger(
          field(position, "position", itemPath),
          `${itemPath}.position`,
        ),
        responseState: state(
          field(position, "responseState", itemPath),
          `${itemPath}.responseState`,
        ),
      };
    },
  );
  if (
    positions.length !== questionCount ||
    positions.some((item, index) => item.position !== index + 1)
  )
    throw new DecodeError(`${path}.positions`, "each 1-based issued position exactly once");
  if (recommendedPosition !== null && recommendedPosition > questionCount)
    throw new DecodeError(`${path}.recommendedPosition`, "an issued position or null");
  const assessmentAttempt = decodeAssessmentAttemptReference(
    field(record, "assessmentAttempt", path),
    `${path}.assessmentAttempt`,
  );
  return { assessmentAttempt, questionCount, recommendedPosition, positions };
}

export function decodeStudentAssessmentAttemptPresentation(
  value: unknown,
  path = "response",
): StudentAssessmentAttemptPresentation {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["position", "presentation", "savedResponse"]);
  const savedResponse = field(record, "savedResponse", path);
  return {
    position: decodePositiveInteger(field(record, "position", path), `${path}.position`),
    presentation: decodeStudentQuestionPresentation(
      field(record, "presentation", path),
      `${path}.presentation`,
    ),
    savedResponse:
      savedResponse === null ? null : decodeStudentResponse(savedResponse, `${path}.savedResponse`),
  };
}

export function decodeStudentAssessmentAttemptResponseSaveAcknowledgement(
  value: unknown,
  path = "response",
): StudentAssessmentAttemptResponseSaveAcknowledgement {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["assessmentAttempt", "position", "responseState"]);
  const responseState = field(record, "responseState", path);
  if (responseState !== "saved")
    throw new DecodeError(`${path}.responseState`, 'the durable response state "saved"');
  return {
    assessmentAttempt: decodeAssessmentAttemptReference(
      field(record, "assessmentAttempt", path),
      `${path}.assessmentAttempt`,
    ),
    position: decodePositiveInteger(field(record, "position", path), `${path}.position`),
    responseState,
  };
}

export function decodeStudentAssessmentAttemptSubmissionResult(
  value: unknown,
  path = "response",
): StudentAssessmentAttemptSubmissionResult {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["assessmentAttempt", "submissionState"]);
  const submissionState = field(record, "submissionState", path);
  if (submissionState !== "submitted")
    throw new DecodeError(
      `${path}.submissionState`,
      'the Assessment Attempt submission state "submitted"',
    );
  return {
    assessmentAttempt: decodeAssessmentAttemptReference(
      field(record, "assessmentAttempt", path),
      `${path}.assessmentAttempt`,
    ),
    submissionState,
  };
}
