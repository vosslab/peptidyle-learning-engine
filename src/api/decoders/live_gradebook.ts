// Strict decoding for the answer-free Gradebook projection.

import type { AssignmentReference } from "../../../generated/api/AssignmentReference";
import type { AssignmentAttemptCompletion } from "../../../generated/api/AssignmentAttemptCompletion";
import type { CourseInstanceReference } from "../../../generated/api/CourseInstanceReference";
import type { LiveDemoGradebook, LiveDemoStudentWork } from "../live_gradebook";
import {
  DecodeError,
  decodeArray,
  decodeNonnegativeInteger,
  decodeNullable,
  decodePositiveInteger,
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

function studentWork(value: unknown, path: string): LiveDemoStudentWork {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "rosterId",
    "assignmentReference",
    "assignmentAttemptCompletion",
    "gradedQuestionCount",
    "questionCount",
    "pointsEarned",
    "pointsPossible",
  ]);
  const pointsEarned = nonNegativeFinite(
    field(record, "pointsEarned", path),
    `${path}.pointsEarned`,
  );
  const pointsPossible = nonNegativeFinite(
    field(record, "pointsPossible", path),
    `${path}.pointsPossible`,
  );
  const gradedQuestionCount = decodeNonnegativeInteger(
    field(record, "gradedQuestionCount", path),
    `${path}.gradedQuestionCount`,
  );
  const questionCount = decodePositiveInteger(
    field(record, "questionCount", path),
    `${path}.questionCount`,
  );
  const assignmentAttemptCompletion = decodeNullable(
    field(record, "assignmentAttemptCompletion", path),
    `${path}.assignmentAttemptCompletion`,
    (candidate, candidatePath) =>
      decodeStringEnum(candidate, candidatePath, ASSIGNMENT_ATTEMPT_COMPLETIONS),
  );
  if (
    gradedQuestionCount > questionCount ||
    pointsEarned > pointsPossible ||
    (assignmentAttemptCompletion === null &&
      (gradedQuestionCount !== 0 || pointsEarned !== 0 || pointsPossible !== 0))
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
    gradedQuestionCount,
    questionCount,
    pointsEarned,
    pointsPossible,
  };
}

/** Rejects any field outside the declared answer-free projection. */
export function decodeLiveDemoGradebook(value: unknown, path = "response"): LiveDemoGradebook {
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
