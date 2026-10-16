// Strict decoder for the self-only, global Attempt History.

import type {
  StudentAttemptHistoryFacetPage,
  StudentAttemptHistoryPage,
} from "../student_course_attempt_history";
import {
  DecodeError,
  decodeArray,
  decodeFiniteNumber,
  decodeNonnegativeInteger,
  decodePositiveInteger,
  decodeRecord,
  decodeString,
} from "../decoder";
import { parseAssessmentAttemptId, parseCourseInstanceId } from "../../navigation/public_route";
import {
  decodeAssessmentId,
  decodeAssessmentTitle,
  decodeTimestamp,
  field,
  requireOnlyFields,
} from "./shared";

function score(value: unknown, path: string): { pointsEarned: number; pointsPossible: number } {
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
    throw new DecodeError(path, "a complete nonnegative disclosed score");
  }
  return { pointsEarned, pointsPossible };
}

function entry(value: unknown, path: string): StudentAttemptHistoryPage["items"][number] {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "assessmentAttemptId",
    "assessmentId",
    "assessmentTitle",
    "assessmentAttemptNumber",
    "startedAt",
    "submittedAt",
    "assessmentScore",
    "courseInstanceId",
    "courseShortName",
    "courseLongName",
  ]);
  const rawAttemptId = decodeString(
    field(record, "assessmentAttemptId", path),
    `${path}.assessmentAttemptId`,
  );
  const assessmentAttemptId = parseAssessmentAttemptId(rawAttemptId);
  if (assessmentAttemptId === null) {
    throw new DecodeError(`${path}.assessmentAttemptId`, "an Assessment Attempt UUID");
  }
  const courseInstanceId = parseCourseInstanceId(
    decodeString(field(record, "courseInstanceId", path), `${path}.courseInstanceId`),
  );
  if (courseInstanceId === null) {
    throw new DecodeError(`${path}.courseInstanceId`, "a canonical Course ID");
  }
  const courseShortName = decodeString(
    field(record, "courseShortName", path),
    `${path}.courseShortName`,
  );
  const courseLongName = decodeString(
    field(record, "courseLongName", path),
    `${path}.courseLongName`,
  );
  if (
    !courseShortName.trim() ||
    !courseLongName.trim() ||
    [...courseShortName].length > 200 ||
    [...courseLongName].length > 200
  ) {
    throw new DecodeError(path, "bounded Course names");
  }
  const startedAt = decodeTimestamp(field(record, "startedAt", path), `${path}.startedAt`);
  const submittedAt =
    record.submittedAt === undefined
      ? undefined
      : decodeTimestamp(record.submittedAt, `${path}.submittedAt`);
  const assessmentScore =
    record.assessmentScore === undefined
      ? undefined
      : score(record.assessmentScore, `${path}.assessmentScore`);
  if (
    (submittedAt !== undefined && submittedAt < startedAt) ||
    (assessmentScore !== undefined && submittedAt === undefined)
  ) {
    throw new DecodeError(path, "consistent Attempt dates and score disclosure");
  }
  return {
    courseInstanceId,
    courseShortName,
    courseLongName,
    assessmentAttemptId,
    assessmentId: decodeAssessmentId(field(record, "assessmentId", path), `${path}.assessmentId`),
    assessmentTitle: decodeAssessmentTitle(
      field(record, "assessmentTitle", path),
      `${path}.assessmentTitle`,
    ),
    assessmentAttemptNumber: decodePositiveInteger(
      field(record, "assessmentAttemptNumber", path),
      `${path}.assessmentAttemptNumber`,
    ),
    startedAt,
    ...(submittedAt === undefined ? {} : { submittedAt }),
    ...(assessmentScore === undefined ? {} : { assessmentScore }),
  };
}

export function decodeStudentAttemptHistoryPage(
  value: unknown,
  path = "response",
): StudentAttemptHistoryPage {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["items", "nextCursor"]);
  const items = decodeArray(field(record, "items", path), `${path}.items`, entry);
  const nextCursor =
    record.nextCursor === undefined
      ? undefined
      : decodeString(record.nextCursor, `${path}.nextCursor`);
  if (nextCursor !== undefined && (nextCursor.length === 0 || nextCursor.length > 512)) {
    throw new DecodeError(`${path}.nextCursor`, "an opaque cursor of at most 512 characters");
  }
  if (items.length > 100) {
    throw new DecodeError(`${path}.items`, "a bounded page of at most 100 Attempts");
  }
  return { items, ...(nextCursor === undefined ? {} : { nextCursor }) };
}

export function decodeStudentAttemptHistoryFacetPage(
  value: unknown,
  path = "response",
): StudentAttemptHistoryFacetPage {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["choices", "nextCursor"]);
  const choices = decodeArray(
    field(record, "choices", path),
    `${path}.choices`,
    (value, choicePath) => {
      const choice = decodeRecord(value, choicePath);
      requireOnlyFields(choice, choicePath, ["key", "label", "count"]);
      const key = decodeString(field(choice, "key", choicePath), `${choicePath}.key`);
      const label = decodeString(field(choice, "label", choicePath), `${choicePath}.label`);
      if (!key || !label || key.length > 200 || label.length > 200) {
        throw new DecodeError(choicePath, "bounded facet choice");
      }
      return {
        key,
        label,
        count: decodeNonnegativeInteger(field(choice, "count", choicePath), `${choicePath}.count`),
      };
    },
  );
  if (choices.length > 50) {
    throw new DecodeError(`${path}.choices`, "at most 50 facet choices");
  }
  const nextCursor =
    record.nextCursor === undefined
      ? undefined
      : decodeString(record.nextCursor, `${path}.nextCursor`);
  if (nextCursor !== undefined && (nextCursor.length === 0 || nextCursor.length > 512)) {
    throw new DecodeError(`${path}.nextCursor`, "an opaque cursor of at most 512 characters");
  }
  return { choices, ...(nextCursor === undefined ? {} : { nextCursor }) };
}
