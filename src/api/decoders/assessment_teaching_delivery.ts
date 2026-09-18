// Strict Student delivery and focused assessment-policy transport decoders.

import type { AssessmentAuthoredContentValidationFailure } from "../../../generated/api/AssessmentAuthoredContentValidationFailure";
import type { InstructorAssessmentAuthoredContentLocal } from "../../../generated/api/InstructorAssessmentAuthoredContentLocal";
import type { InstructorAssessmentAvailabilityView } from "../../../generated/api/InstructorAssessmentAvailabilityView";
import type { StudentAssessmentDetail } from "../../../generated/api/StudentAssessmentDetail";
import type { StudentAssessmentLandingSummary } from "../../../generated/api/StudentAssessmentLandingSummary";
import type { StudentAssessmentDelivery } from "../../../generated/api/StudentAssessmentDelivery";
import {
  DecodeError,
  decodeArray,
  decodeNonemptyString,
  decodeNullable,
  decodePositiveInteger,
  decodeRecord,
  decodeString,
  decodeStringEnum,
} from "../decoder";
import { decodeIdentifier, decodeTimestamp, field, requireOnlyFields } from "./shared";
import { decodeAssessmentEntry } from "./question_library";
import { decodeAssessmentId } from "./shared";

export function decodeStudentAssessmentLandingSummary(
  value: unknown,
  path = "response",
  strict = true,
): StudentAssessmentLandingSummary {
  const record = decodeRecord(value, path);
  if (strict) {
    requireOnlyFields(record, path, ["id", "reference", "title"]);
  }
  return {
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    reference: decodeAssessmentId(field(record, "reference", path), `${path}.reference`),
    title: decodeNonemptyString(field(record, "title", path), `${path}.title`),
  } satisfies StudentAssessmentLandingSummary;
}

const LATE_POLICIES = ["accept", "mark_late", "reject"] as const;
const STUDENT_LATE_WORK_STATUSES = ["on_time", "accepted_late", "marked_late"] as const;
const ASSIGNMENT_AUTHORED_CONTENT_FAILURE_FIELDS = [
  "assessmentAuthoredContent",
  "availableAt",
  "dueAt",
  "closesAt",
  "schedule",
  "assessmentAttemptTimeLimitSeconds",
  "attemptLimit",
  "instructions",
] as const;
const ASSIGNMENT_AUTHORED_CONTENT_FAILURE_REASONS = [
  "invalidInput",
  "outsideCourseTerm",
  "nonexistentLocalTime",
  "ambiguousLocalTime",
  "timestampOutOfRange",
  "scheduleOutOfOrder",
  "assessmentAttemptTimeLimitOutOfRange",
  "attemptLimitOutOfRange",
  "invalidInstructions",
] as const;
const LOCAL_TIME = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}$/u;

function decodeInstructions(value: unknown, path: string): string {
  const text = decodeString(value, path);
  if (text.includes("\0") || Array.from(text).length > 50_000)
    throw new DecodeError(
      path,
      "safe plain-text instructions no longer than 50000 Unicode scalars",
    );
  return text;
}

function decodeLocalTime(value: unknown, path: string): string {
  const text = decodeString(value, path);
  if (!LOCAL_TIME.test(text)) throw new DecodeError(path, "a canonical local timestamp");
  return text;
}

function decodePolicyLimit(value: unknown, path: string): number | null {
  return decodeNullable(value, path, (entry, entryPath) => {
    const number = decodePositiveInteger(entry, entryPath);
    if (number > 2_147_483_647) throw new DecodeError(entryPath, "a bounded positive integer");
    return number;
  });
}

export function decodeAssessmentAuthoredContentValidationFailure(
  value: unknown,
  path = "response",
): AssessmentAuthoredContentValidationFailure {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["error", "field", "reason", "message"]);
  return {
    error: decodeStringEnum(field(record, "error", path), `${path}.error`, [
      "assessmentAuthoredContentInvalid",
    ] as const),
    field: decodeStringEnum(
      field(record, "field", path),
      `${path}.field`,
      ASSIGNMENT_AUTHORED_CONTENT_FAILURE_FIELDS,
    ),
    reason: decodeStringEnum(
      field(record, "reason", path),
      `${path}.reason`,
      ASSIGNMENT_AUTHORED_CONTENT_FAILURE_REASONS,
    ),
    message: decodeNonemptyString(field(record, "message", path), `${path}.message`),
  };
}

export function decodeInstructorAssessmentAuthoredContentLocal(
  value: unknown,
  path = "response",
): InstructorAssessmentAuthoredContentLocal {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "instructions",
    "available_at",
    "due_at",
    "closes_at",
    "assessment_attempt_time_limit_seconds",
    "attempt_limit",
    "late_work_rule",
  ]);
  return {
    instructions: decodeInstructions(field(record, "instructions", path), `${path}.instructions`),
    available_at: decodeNullable(
      field(record, "available_at", path),
      `${path}.available_at`,
      decodeLocalTime,
    ),
    due_at: decodeNullable(field(record, "due_at", path), `${path}.due_at`, decodeLocalTime),
    closes_at: decodeNullable(
      field(record, "closes_at", path),
      `${path}.closes_at`,
      decodeLocalTime,
    ),
    assessment_attempt_time_limit_seconds: decodePolicyLimit(
      field(record, "assessment_attempt_time_limit_seconds", path),
      `${path}.assessment_attempt_time_limit_seconds`,
    ),
    attempt_limit: decodePolicyLimit(field(record, "attempt_limit", path), `${path}.attempt_limit`),
    late_work_rule: decodeStringEnum(
      field(record, "late_work_rule", path),
      `${path}.late_work_rule`,
      LATE_POLICIES,
    ),
  };
}

export function decodeInstructorAssessmentAvailabilityView(
  value: unknown,
  path = "response",
): InstructorAssessmentAvailabilityView {
  const record = decodeRecord(value, path);
  const state = decodeStringEnum(field(record, "state", path), `${path}.state`, [
    "unreleased",
    "scheduled",
    "available",
    "closed",
    "archived",
  ] as const);
  if (state === "scheduled") {
    requireOnlyFields(record, path, ["state", "available_at"]);
    return {
      state,
      available_at: decodeLocalTime(field(record, "available_at", path), `${path}.available_at`),
    };
  }
  if (state === "closed") {
    requireOnlyFields(record, path, ["state", "closed_at"]);
    return {
      state,
      closed_at: decodeNullable(
        field(record, "closed_at", path),
        `${path}.closed_at`,
        decodeLocalTime,
      ),
    };
  }
  requireOnlyFields(record, path, ["state"]);
  return { state };
}

export function decodeStudentAssessmentDetail(
  value: unknown,
  path = "response",
): StudentAssessmentDetail {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "id",
    "reference",
    "title",
    "instructions",
    "display_time_zone",
    "delivery",
    "entries",
  ]);
  const deliveryRecord = decodeRecord(field(record, "delivery", path), `${path}.delivery`);
  requireOnlyFields(deliveryRecord, `${path}.delivery`, [
    "available_at",
    "due_at",
    "closes_at",
    "assessment_attempt_time_limit_seconds",
    "attempt_limit",
    "late_work_rule",
    "student_late_work_status",
  ]);
  const delivery: StudentAssessmentDelivery = {
    available_at: decodeNullable(
      field(deliveryRecord, "available_at", `${path}.delivery`),
      `${path}.delivery.available_at`,
      decodeTimestamp,
    ),
    due_at: decodeNullable(
      field(deliveryRecord, "due_at", `${path}.delivery`),
      `${path}.delivery.due_at`,
      decodeTimestamp,
    ),
    closes_at: decodeNullable(
      field(deliveryRecord, "closes_at", `${path}.delivery`),
      `${path}.delivery.closes_at`,
      decodeTimestamp,
    ),
    assessment_attempt_time_limit_seconds: decodePolicyLimit(
      field(deliveryRecord, "assessment_attempt_time_limit_seconds", `${path}.delivery`),
      `${path}.delivery.assessment_attempt_time_limit_seconds`,
    ),
    attempt_limit: decodePolicyLimit(
      field(deliveryRecord, "attempt_limit", `${path}.delivery`),
      `${path}.delivery.attempt_limit`,
    ),
    late_work_rule: decodeStringEnum(
      field(deliveryRecord, "late_work_rule", `${path}.delivery`),
      `${path}.delivery.late_work_rule`,
      LATE_POLICIES,
    ),
    student_late_work_status: decodeStringEnum(
      field(deliveryRecord, "student_late_work_status", `${path}.delivery`),
      `${path}.delivery.student_late_work_status`,
      STUDENT_LATE_WORK_STATUSES,
    ),
  };
  return {
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    reference: decodeAssessmentId(field(record, "reference", path), `${path}.reference`),
    title: decodeNonemptyString(field(record, "title", path), `${path}.title`),
    instructions: decodeInstructions(field(record, "instructions", path), `${path}.instructions`),
    display_time_zone: decodeNonemptyString(
      field(record, "display_time_zone", path),
      `${path}.display_time_zone`,
    ),
    delivery,
    entries: decodeArray(field(record, "entries", path), `${path}.entries`, decodeAssessmentEntry),
  };
}
