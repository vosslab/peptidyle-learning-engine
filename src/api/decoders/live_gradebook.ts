// Strict decoding for the answer-free Gradebook projection.

import type { AssessmentReference } from "../../../generated/api/AssessmentReference";
import type { AssessmentAttemptCompletion } from "../../../generated/api/AssessmentAttemptCompletion";
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
import { decodeAssessmentTitle, field, requireOnlyFields } from "./shared";
import { decodeRosterName } from "./course_roster";

const ASSIGNMENT_ATTEMPT_COMPLETIONS = [
  "inProgress",
  "completed",
] as const satisfies ReadonlyArray<AssessmentAttemptCompletion>;

function courseReference(value: unknown, path: string): CourseInstanceReference {
  const decoded = decodeString(value, path);
  if (!/^CI[0-9ABCDEFGHJKMNPQRSTVWXYZ]{6}$/u.test(decoded)) {
    throw new DecodeError(path, "a canonical opaque Course Instance reference");
  }
  return decoded;
}

function assessmentReference(value: unknown, path: string): AssessmentReference {
  const decoded = decodeString(value, path);
  if (!/^A[0-9ABCDEFGHJKMNPQRSTVWXYZ]{6}$/u.test(decoded)) {
    throw new DecodeError(path, "a canonical opaque Assessment reference");
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
    "rosterName",
    "assessmentReference",
    "assessmentTitle",
    "assessmentAttemptCompletion",
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
          return { pointsEarned, pointsPossible };
        })();
  const assessmentAttemptCompletion = decodeNullable(
    field(record, "assessmentAttemptCompletion", path),
    `${path}.assessmentAttemptCompletion`,
    (candidate, candidatePath) =>
      decodeStringEnum(candidate, candidatePath, ASSIGNMENT_ATTEMPT_COMPLETIONS),
  );
  if (
    (assessmentAttemptCompletion === null && (score !== null || expiredSubmitting)) ||
    (expiredSubmitting && (assessmentAttemptCompletion !== "inProgress" || score !== null))
  ) {
    throw new DecodeError(path, "internally consistent answer-free progress totals");
  }
  return {
    rosterId: rosterId(field(record, "rosterId", path), `${path}.rosterId`),
    rosterName: decodeRosterName(field(record, "rosterName", path), `${path}.rosterName`),
    assessmentReference: assessmentReference(
      field(record, "assessmentReference", path),
      `${path}.assessmentReference`,
    ),
    assessmentAttemptCompletion,
    assessmentTitle: decodeAssessmentTitle(
      field(record, "assessmentTitle", path),
      `${path}.assessmentTitle`,
    ),
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
