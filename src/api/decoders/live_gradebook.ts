// Strict decoding for the answer-free Gradebook projection.

import type { AssignmentReference } from "../../../generated/api/AssignmentReference";
import type { AssignmentAttemptCompletion } from "../../../generated/api/AssignmentAttemptCompletion";
import type { CourseInstanceReference } from "../../../generated/api/CourseInstanceReference";
import type { CourseGradebook, CourseGradebookStudentWork } from "../live_gradebook";
import {
  DecodeError,
  decodeArray,
  decodeNullable,
  decodeRecord,
  decodeString,
  decodeStringEnum,
} from "../decoder";
import { field, requireOnlyFields } from "./shared";

const MAX_REFERENCE = 2_147_483_647;
const ASSIGNMENT_ATTEMPT_COMPLETIONS = [
  "inProgress",
  "completed",
] as const satisfies ReadonlyArray<AssignmentAttemptCompletion>;

function courseReference(value: unknown, path: string): CourseInstanceReference {
  const decoded = decodeString(value, path);
  if (!/^C-[1-9][0-9]{0,9}$/u.test(decoded) || Number(decoded.slice(2)) > MAX_REFERENCE) {
    throw new DecodeError(path, "a canonical Course Instance public reference");
  }
  return decoded;
}

function assignmentReference(value: unknown, path: string): AssignmentReference {
  const decoded = decodeString(value, path);
  if (!/^A-[1-9][0-9]{0,9}$/u.test(decoded) || Number(decoded.slice(2)) > MAX_REFERENCE) {
    throw new DecodeError(path, "a canonical Assignment public reference");
  }
  return decoded;
}

function rosterId(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  if (!/^[A-Za-z0-9._-]{1,64}$/u.test(decoded)) {
    throw new DecodeError(path, "a bounded roster identifier");
  }
  return decoded;
}

function nonNegativeFinite(value: unknown, path: string): number {
  if (typeof value !== "number") throw new DecodeError(path, "a number");
  const decoded = value;
  if (!Number.isFinite(decoded) || decoded < 0) {
    throw new DecodeError(path, "a non-negative finite number");
  }
  return decoded;
}

function studentWork(value: unknown, path: string): CourseGradebookStudentWork {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "rosterId",
    "assignmentReference",
    "assignmentAttemptCompletion",
    "expiredSubmitting",
    "score",
  ]);
  const scoreValue = field(record, "score", path);
  const expiredSubmitting = field(record, "expiredSubmitting", path);
  if (typeof expiredSubmitting !== "boolean")
    throw new DecodeError(`${path}.expiredSubmitting`, "a boolean");
  const score =
    scoreValue === null
      ? null
      : ((): NonNullable<CourseGradebookStudentWork["score"]> => {
          const scoreRecord = decodeRecord(scoreValue, `${path}.score`);
          requireOnlyFields(scoreRecord, `${path}.score`, ["pointsEarned", "pointsPossible"]);
          const pointsEarned = nonNegativeFinite(
            field(scoreRecord, "pointsEarned", `${path}.score`),
            `${path}.score.pointsEarned`,
          );
          const pointsPossible = nonNegativeFinite(
            field(scoreRecord, "pointsPossible", `${path}.score`),
            `${path}.score.pointsPossible`,
          );
          if (pointsEarned > pointsPossible)
            throw new DecodeError(`${path}.score`, "an ordered non-negative Assignment score");
          return { pointsEarned, pointsPossible };
        })();
  const assignmentAttemptCompletion = decodeNullable(
    field(record, "assignmentAttemptCompletion", path),
    `${path}.assignmentAttemptCompletion`,
    (candidate, candidatePath) =>
      decodeStringEnum(candidate, candidatePath, ASSIGNMENT_ATTEMPT_COMPLETIONS),
  );
  if (
    (assignmentAttemptCompletion === null && (score !== null || expiredSubmitting)) ||
    (expiredSubmitting && (assignmentAttemptCompletion !== "inProgress" || score !== null))
  ) {
    throw new DecodeError(path, "internally consistent answer-free progress totals");
  }
  return {
    rosterId: rosterId(field(record, "rosterId", path), `${path}.rosterId`),
    assignmentReference: assignmentReference(
      field(record, "assignmentReference", path),
      `${path}.assignmentReference`,
    ),
    assignmentAttemptCompletion,
    expiredSubmitting,
    score,
  };
}

/** Rejects any field outside the declared answer-free projection. */
export function decodeCourseGradebook(value: unknown, path = "response"): CourseGradebook {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["courseReference", "studentWork"]);
  return {
    courseReference: courseReference(
      field(record, "courseReference", path),
      `${path}.courseReference`,
    ),
    studentWork: decodeArray(
      field(record, "studentWork", path),
      `${path}.studentWork`,
      studentWork,
    ),
  };
}
