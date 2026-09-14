// Strict decoder for the Rust-owned Student Assignment decision summary.

import type { AssignmentStartDecision } from "../../../generated/api/AssignmentStartDecision";
import type { LateWorkRule } from "../../../generated/api/LateWorkRule";
import type { StudentAssignmentDecisionSummary } from "../../../generated/api/StudentAssignmentDecisionSummary";
import {
  DecodeError,
  decodeNullable,
  decodePositiveInteger,
  decodeRecord,
  decodeString,
  decodeStringEnum,
} from "../decoder";
import { decodeTimestamp, field, requireOnlyFields } from "./shared";

const START_DECISIONS = [
  "may_start",
  "not_yet_available",
  "closed",
  "attempt_limit_reached",
  "late_work_refused",
] as const satisfies ReadonlyArray<AssignmentStartDecision>;

const LATE_WORK_RULES = [
  "accept",
  "mark_late",
  "reject",
] as const satisfies ReadonlyArray<LateWorkRule>;

const PUBLIC_REASONS: Readonly<Record<Exclude<AssignmentStartDecision, "may_start">, string>> = {
  not_yet_available: "This Assignment is not yet available.",
  closed: "This Assignment is closed for new work.",
  attempt_limit_reached: "The allowed number of Assignment Attempts has been reached.",
  late_work_refused: "New work is not available under the late-work policy.",
};

export function decodeAccountTimeZone(value: unknown, path: string): string {
  const timeZone = decodeString(value, path);
  if (timeZone.length === 0 || timeZone.length > 255 || timeZone.trim() !== timeZone) {
    throw new DecodeError(path, "a bounded exact IANA time-zone name");
  }
  try {
    new Intl.DateTimeFormat(undefined, { timeZone });
  } catch {
    throw new DecodeError(path, "a browser-supported IANA time-zone name");
  }
  return timeZone;
}

/**
 * ASVS 1.5.2 and 14.2.6: admit only the closed public decision DTO and reject
 * storage, accommodation-identity, worker, and grading fields at the boundary.
 */
export function decodeStudentAssignmentDecision(
  value: unknown,
  path: string,
): StudentAssignmentDecisionSummary {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "availableAt",
    "dueAt",
    "closesAt",
    "timeLimitSeconds",
    "attemptLimit",
    "lateWorkRule",
    "displayTimeZone",
    "evaluatedAt",
    "startDecision",
    "publicReason",
  ]);
  const startDecision = decodeStringEnum(
    field(record, "startDecision", path),
    `${path}.startDecision`,
    START_DECISIONS,
  );
  const publicReason = decodeNullable(
    field(record, "publicReason", path),
    `${path}.publicReason`,
    decodeString,
  );
  const expectedReason = startDecision === "may_start" ? null : PUBLIC_REASONS[startDecision];
  if (publicReason !== expectedReason) {
    throw new DecodeError(`${path}.publicReason`, "the public reason for the server decision");
  }
  const availableAt = decodeNullable(
    field(record, "availableAt", path),
    `${path}.availableAt`,
    decodeTimestamp,
  );
  const dueAt = decodeNullable(field(record, "dueAt", path), `${path}.dueAt`, decodeTimestamp);
  const closesAt = decodeNullable(
    field(record, "closesAt", path),
    `${path}.closesAt`,
    decodeTimestamp,
  );
  if (
    (availableAt !== null && dueAt !== null && availableAt > dueAt) ||
    (availableAt !== null && closesAt !== null && availableAt > closesAt) ||
    (dueAt !== null && closesAt !== null && dueAt > closesAt)
  ) {
    throw new DecodeError(path, "an ordered Assignment schedule");
  }
  return {
    availableAt,
    dueAt,
    closesAt,
    timeLimitSeconds: decodeNullable(
      field(record, "timeLimitSeconds", path),
      `${path}.timeLimitSeconds`,
      decodePositiveInteger,
    ),
    attemptLimit: decodeNullable(
      field(record, "attemptLimit", path),
      `${path}.attemptLimit`,
      decodePositiveInteger,
    ),
    lateWorkRule: decodeStringEnum(
      field(record, "lateWorkRule", path),
      `${path}.lateWorkRule`,
      LATE_WORK_RULES,
    ),
    displayTimeZone: decodeAccountTimeZone(
      field(record, "displayTimeZone", path),
      `${path}.displayTimeZone`,
    ),
    evaluatedAt: decodeTimestamp(field(record, "evaluatedAt", path), `${path}.evaluatedAt`),
    startDecision,
    publicReason,
  };
}
