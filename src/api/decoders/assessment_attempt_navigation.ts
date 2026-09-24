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
import {
  decodeAssessmentId,
  decodeCourseInstanceId,
  decodeCourseName,
  field,
  requireOnlyFields,
} from "./shared";
import {
  type AssessmentAttemptRouteId,
  parseAssessmentAttemptId,
} from "../../navigation/public_route";
import { COURSE_THEME_VALUES } from "../../../generated/api/CourseTheme";
import { ASSESSMENT_TYPE_VALUES } from "../../../generated/api/AssessmentType";
import { decodeAccountTimeZone } from "./student_assessment_decision";
import { decodeStudentQuestionPresentation } from "./presentation_delivery";
import { decodeStudentResponse } from "./question_delivery";

function state(value: unknown, path: string): StudentAssessmentAttemptResponseState {
  if (value === "unanswered" || value === "saved" || value === "submitted" || value === "closed")
    return value;
  throw new DecodeError(path, "an answer-free response state");
}

function decodeAssessmentAttemptId(value: unknown, path: string): AssessmentAttemptRouteId {
  if (typeof value !== "string") throw new DecodeError(path, "an Assessment Attempt UUID");
  const parsed = parseAssessmentAttemptId(value);
  if (parsed === null) throw new DecodeError(path, "an Assessment Attempt UUID");
  return parsed;
}

/** Strict browser-safe Student Attempt context used by the persistent route scope. */
export function decodeStudentAssessmentAttemptContext(
  value: unknown,
  path = "response",
): StudentAssessmentAttemptContext {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "assessmentAttemptId",
    "attemptNumber",
    "displayTimeZone",
    "expiresAt",
    "timerRemainingMilliseconds",
    "course",
    "assessment",
  ]);
  const course = decodeRecord(field(record, "course", path), `${path}.course`);
  requireOnlyFields(course, `${path}.course`, ["id", "shortName", "longName", "theme"]);
  const assessment = decodeRecord(field(record, "assessment", path), `${path}.assessment`);
  requireOnlyFields(assessment, `${path}.assessment`, ["id", "assessmentType", "title"]);
  const remaining = field(record, "timerRemainingMilliseconds", path);
  const expiresAt = field(record, "expiresAt", path);
  return {
    assessmentAttemptId: decodeAssessmentAttemptId(
      field(record, "assessmentAttemptId", path),
      `${path}.assessmentAttemptId`,
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
      id: decodeCourseInstanceId(field(course, "id", `${path}.course`), `${path}.course.id`),
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
      id: decodeAssessmentId(
        field(assessment, "id", `${path}.assessment`),
        `${path}.assessment.id`,
      ),
      assessmentType: decodeStringEnum(
        field(assessment, "assessmentType", `${path}.assessment`),
        `${path}.assessment.assessmentType`,
        ASSESSMENT_TYPE_VALUES,
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
    "assessmentAttemptId",
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
      requireOnlyFields(position, itemPath, ["position", "responseState", "displayDurationMs"]);
      const displayDurationMs =
        position.displayDurationMs === null
          ? null
          : decodeNonnegativeInteger(
              field(position, "displayDurationMs", itemPath),
              `${itemPath}.displayDurationMs`,
            );
      return {
        position: decodePositiveInteger(
          field(position, "position", itemPath),
          `${itemPath}.position`,
        ),
        responseState: state(
          field(position, "responseState", itemPath),
          `${itemPath}.responseState`,
        ),
        displayDurationMs,
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
  const assessmentAttemptId = decodeAssessmentAttemptId(
    field(record, "assessmentAttemptId", path),
    `${path}.assessmentAttemptId`,
  );
  return { assessmentAttemptId, questionCount, recommendedPosition, positions };
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
  requireOnlyFields(record, path, ["assessmentAttemptId", "position", "responseState"]);
  const responseState = field(record, "responseState", path);
  if (responseState !== "saved")
    throw new DecodeError(`${path}.responseState`, 'the durable response state "saved"');
  return {
    assessmentAttemptId: decodeAssessmentAttemptId(
      field(record, "assessmentAttemptId", path),
      `${path}.assessmentAttemptId`,
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
  requireOnlyFields(record, path, ["assessmentAttemptId", "submissionState"]);
  const submissionState = field(record, "submissionState", path);
  if (submissionState !== "submitted")
    throw new DecodeError(
      `${path}.submissionState`,
      'the Assessment Attempt submission state "submitted"',
    );
  return {
    assessmentAttemptId: decodeAssessmentAttemptId(
      field(record, "assessmentAttemptId", path),
      `${path}.assessmentAttemptId`,
    ),
    submissionState,
  };
}
