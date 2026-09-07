// Strict decoding for the answer-free M15 Gradebook projection.

import type { AssignmentReference } from "../../../generated/api/AssignmentReference";
import type { CourseInstanceReference } from "../../../generated/api/CourseInstanceReference";
import type { LiveDemoGradebook, LiveDemoGradedStudentWork } from "../live_gradebook";
import { DecodeError, decodeArray, decodeRecord, decodeString } from "../decoder";
import { field, requireOnlyFields } from "./shared";

const MAX_REFERENCE = 2_147_483_647;

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

function gradedWork(value: unknown, path: string): LiveDemoGradedStudentWork {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "rosterId",
    "assignmentReference",
    "gradedQuestionCount",
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
  const gradedQuestionCount = nonNegativeFinite(
    field(record, "gradedQuestionCount", path),
    `${path}.gradedQuestionCount`,
  );
  if (!Number.isSafeInteger(gradedQuestionCount) || gradedQuestionCount < 1) {
    throw new DecodeError(`${path}.gradedQuestionCount`, "a positive whole number");
  }
  if (pointsEarned > pointsPossible) {
    throw new DecodeError(path, "points earned no greater than points possible");
  }
  return {
    rosterId: rosterId(field(record, "rosterId", path), `${path}.rosterId`),
    assignmentReference: assignmentReference(
      field(record, "assignmentReference", path),
      `${path}.assignmentReference`,
    ),
    gradedQuestionCount,
    pointsEarned,
    pointsPossible,
  };
}

/** Rejects any field outside the declared answer-free M15 projection. */
export function decodeLiveDemoGradebook(value: unknown, path = "response"): LiveDemoGradebook {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["courseReference", "gradedStudentWork"]);
  return {
    courseReference: courseReference(
      field(record, "courseReference", path),
      `${path}.courseReference`,
    ),
    gradedStudentWork: decodeArray(
      field(record, "gradedStudentWork", path),
      `${path}.gradedStudentWork`,
      gradedWork,
    ),
  };
}
