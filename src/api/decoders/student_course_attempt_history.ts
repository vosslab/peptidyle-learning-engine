// Strict decoder for the self-only, cursor-paginated Course Attempt History.

import type { StudentCourseAttemptHistoryPage } from "../student_course_attempt_history";
import {
  DecodeError,
  decodeArray,
  decodeFiniteNumber,
  decodePositiveInteger,
  decodeRecord,
  decodeString,
} from "../decoder";
import { parseAssessmentAttemptId } from "../../navigation/public_route";
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

function entry(value: unknown, path: string): StudentCourseAttemptHistoryPage["items"][number] {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "assessmentAttemptId",
    "assessmentId",
    "assessmentTitle",
    "assessmentAttemptNumber",
    "startedAt",
    "submittedAt",
    "assessmentScore",
  ]);
  const rawAttemptId = decodeString(
    field(record, "assessmentAttemptId", path),
    `${path}.assessmentAttemptId`,
  );
  const assessmentAttemptId = parseAssessmentAttemptId(rawAttemptId);
  if (assessmentAttemptId === null) {
    throw new DecodeError(`${path}.assessmentAttemptId`, "an Assessment Attempt UUID");
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

export function decodeStudentCourseAttemptHistoryPage(
  value: unknown,
  path = "response",
): StudentCourseAttemptHistoryPage {
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
  const startedAtValues = items.map(({ startedAt }) => startedAt);
  if (
    startedAtValues.some((value, index) => {
      const previous = startedAtValues[index - 1];
      return previous !== undefined && value > previous;
    })
  ) {
    throw new DecodeError(`${path}.items`, "newest-first Attempt ordering");
  }
  // Keep this import exercised as a cross-field invariant for wire integer parsing.
  if (items.length > 100) {
    throw new DecodeError(`${path}.items`, "a bounded page of at most 100 Attempts");
  }
  return { items, ...(nextCursor === undefined ? {} : { nextCursor }) };
}
