import type {
  StudentAssignmentAttemptContext,
  StudentAssignmentAttemptPresentation,
  StudentAssignmentAttemptProgress,
  StudentAssignmentAttemptResponseSaveAcknowledgement,
  StudentAssignmentAttemptResponseState,
  StudentAssignmentAttemptSubmissionAcknowledgement,
} from "../assignment_attempt_navigation";
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
  type AssignmentAttemptRouteReference,
  type AssignmentRouteReference,
  type CourseInstanceRouteReference,
  parseAssignmentAttemptReference,
  parseAssignmentReference,
  parseCourseInstanceReference,
} from "../../navigation/public_route";
import { COURSE_THEME_VALUES } from "../../../generated/api/CourseTheme";
import { decodeStudentQuestionPresentation } from "./presentation_delivery";
import { decodeStudentResponse } from "./question_delivery";

function state(value: unknown, path: string): StudentAssignmentAttemptResponseState {
  if (value === "unanswered" || value === "saved" || value === "submitted" || value === "closed")
    return value;
  throw new DecodeError(path, "an answer-free response state");
}

function decodeAssignmentAttemptReference(
  value: unknown,
  path: string,
): AssignmentAttemptRouteReference {
  if (typeof value !== "string") throw new DecodeError(path, "an Assignment Attempt R- reference");
  const parsed = parseAssignmentAttemptReference(value);
  if (parsed === null) throw new DecodeError(path, "an Assignment Attempt R- reference");
  return parsed;
}

function decodeCourseReference(value: unknown, path: string): CourseInstanceRouteReference {
  if (typeof value !== "string") throw new DecodeError(path, "a Course C- reference");
  const parsed = parseCourseInstanceReference(value);
  if (parsed === null) throw new DecodeError(path, "a Course C- reference");
  return parsed;
}

function decodeAssignmentReference(value: unknown, path: string): AssignmentRouteReference {
  if (typeof value !== "string") throw new DecodeError(path, "an Assignment A- reference");
  const parsed = parseAssignmentReference(value);
  if (parsed === null) throw new DecodeError(path, "an Assignment A- reference");
  return parsed;
}

/** Strict UUID-free Student Attempt context used by the persistent route scope. */
export function decodeStudentAssignmentAttemptContext(
  value: unknown,
  path = "response",
): StudentAssignmentAttemptContext {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "assignmentAttempt",
    "attemptNumber",
    "timerRemainingMilliseconds",
    "course",
    "assignment",
  ]);
  const course = decodeRecord(field(record, "course", path), `${path}.course`);
  requireOnlyFields(course, `${path}.course`, ["reference", "shortName", "longName", "theme"]);
  const assignment = decodeRecord(field(record, "assignment", path), `${path}.assignment`);
  requireOnlyFields(assignment, `${path}.assignment`, ["reference", "title"]);
  const remaining = field(record, "timerRemainingMilliseconds", path);
  return {
    assignmentAttempt: decodeAssignmentAttemptReference(
      field(record, "assignmentAttempt", path),
      `${path}.assignmentAttempt`,
    ),
    attemptNumber: decodePositiveInteger(
      field(record, "attemptNumber", path),
      `${path}.attemptNumber`,
    ),
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
    assignment: {
      reference: decodeAssignmentReference(
        field(assignment, "reference", `${path}.assignment`),
        `${path}.assignment.reference`,
      ),
      title: decodeNonemptyString(
        field(assignment, "title", `${path}.assignment`),
        `${path}.assignment.title`,
      ),
    },
  };
}

export function decodeStudentAssignmentAttemptProgress(
  value: unknown,
  path = "response",
): StudentAssignmentAttemptProgress {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "assignmentAttempt",
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
  const assignmentAttempt = decodeAssignmentAttemptReference(
    field(record, "assignmentAttempt", path),
    `${path}.assignmentAttempt`,
  );
  return { assignmentAttempt, questionCount, recommendedPosition, positions };
}

export function decodeStudentAssignmentAttemptPresentation(
  value: unknown,
  path = "response",
): StudentAssignmentAttemptPresentation {
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

export function decodeStudentAssignmentAttemptResponseSaveAcknowledgement(
  value: unknown,
  path = "response",
): StudentAssignmentAttemptResponseSaveAcknowledgement {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["assignmentAttempt", "position", "responseState"]);
  const responseState = field(record, "responseState", path);
  if (responseState !== "saved")
    throw new DecodeError(`${path}.responseState`, 'the durable response state "saved"');
  return {
    assignmentAttempt: decodeAssignmentAttemptReference(
      field(record, "assignmentAttempt", path),
      `${path}.assignmentAttempt`,
    ),
    position: decodePositiveInteger(field(record, "position", path), `${path}.position`),
    responseState,
  };
}

export function decodeStudentAssignmentAttemptSubmissionAcknowledgement(
  value: unknown,
  path = "response",
): StudentAssignmentAttemptSubmissionAcknowledgement {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["assignmentAttempt", "submissionState"]);
  const submissionState = field(record, "submissionState", path);
  if (submissionState !== "submitted")
    throw new DecodeError(
      `${path}.submissionState`,
      'the Assignment Attempt submission state "submitted"',
    );
  return {
    assignmentAttempt: decodeAssignmentAttemptReference(
      field(record, "assignmentAttempt", path),
      `${path}.assignmentAttempt`,
    ),
    submissionState,
  };
}
