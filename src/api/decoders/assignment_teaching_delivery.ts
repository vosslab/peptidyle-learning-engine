// Strict Student delivery and focused assignment-policy transport decoders.

import type { AssignmentAuthoredContentValidationFailure } from "../../../generated/api/AssignmentAuthoredContentValidationFailure";
import type { InstructorAssignmentAuthoredContentLocal } from "../../../generated/api/InstructorAssignmentAuthoredContentLocal";
import type { InstructorAssignmentAvailabilityView } from "../../../generated/api/InstructorAssignmentAvailabilityView";
import type { StudentAssignmentDetail } from "../../../generated/api/StudentAssignmentDetail";
import type { StudentAssignmentLandingSummary } from "../../../generated/api/StudentAssignmentLandingSummary";
import type { InstructorStudentView } from "../../../generated/api/InstructorStudentView";
import type { InstructorStudentViewDelivery } from "../../../generated/api/InstructorStudentViewDelivery";
import type { StudentAssignmentDelivery } from "../../../generated/api/StudentAssignmentDelivery";
import {
  DecodeError,
  decodeArray,
  decodeNonemptyString,
  decodeNullable,
  decodeNonnegativeInteger,
  decodePositiveInteger,
  decodeRecord,
  decodeString,
  decodeStringEnum,
} from "../decoder";
import { decodeIdentifier, decodeTimestamp, field, requireOnlyFields } from "./shared";
import { decodeStudentFeedbackReleaseRule } from "./assignment_policy";
import { decodeAssignmentEntry } from "./question_library";
import { decodeAssignmentReference } from "./shared";

export function decodeStudentAssignmentLandingSummary(
  value: unknown,
  path = "response",
  strict = true,
): StudentAssignmentLandingSummary {
  const record = decodeRecord(value, path);
  if (strict) {
    requireOnlyFields(record, path, ["id", "reference", "title"]);
  }
  return {
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    reference: decodeAssignmentReference(field(record, "reference", path), `${path}.reference`),
    title: decodeNonemptyString(field(record, "title", path), `${path}.title`),
  } satisfies StudentAssignmentLandingSummary;
}

const LATE_POLICIES = ["accept", "mark_late", "reject"] as const;
const ASSIGNMENT_DEADLINE_RULES = ["auto_submit"] as const;
const STUDENT_LATE_WORK_STATUSES = ["on_time", "accepted_late", "marked_late"] as const;
const ASSIGNMENT_AUTHORED_CONTENT_FAILURE_FIELDS = [
  "assignmentAuthoredContent",
  "timeZone",
  "availableAt",
  "dueAt",
  "closesAt",
  "schedule",
  "assignmentAttemptTimeLimitSeconds",
  "attemptLimit",
  "instructions",
] as const;
const ASSIGNMENT_AUTHORED_CONTENT_FAILURE_REASONS = [
  "invalidInput",
  "courseTimeZoneMismatch",
  "outsideCourseTerm",
  "nonexistentLocalTime",
  "ambiguousLocalTime",
  "timestampOutOfRange",
  "scheduleOutOfOrder",
  "assignmentAttemptTimeLimitOutOfRange",
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
  if (!LOCAL_TIME.test(text)) throw new DecodeError(path, "a canonical course-local timestamp");
  return text;
}

function decodePolicyLimit(value: unknown, path: string): number | null {
  return decodeNullable(value, path, (entry, entryPath) => {
    const number = decodePositiveInteger(entry, entryPath);
    if (number > 2_147_483_647) throw new DecodeError(entryPath, "a bounded positive integer");
    return number;
  });
}

export function decodeAssignmentAuthoredContentValidationFailure(
  value: unknown,
  path = "response",
): AssignmentAuthoredContentValidationFailure {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["error", "field", "reason", "message"]);
  return {
    error: decodeStringEnum(field(record, "error", path), `${path}.error`, [
      "assignmentAuthoredContentInvalid",
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

export function decodeInstructorAssignmentAuthoredContentLocal(
  value: unknown,
  path = "response",
): InstructorAssignmentAuthoredContentLocal {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "timeZone",
    "instructions",
    "available_at",
    "due_at",
    "closes_at",
    "assignment_attempt_time_limit_seconds",
    "attempt_limit",
    "late_work_rule",
    "assignment_deadline_rule",
  ]);
  return {
    timeZone: decodeNonemptyString(field(record, "timeZone", path), `${path}.timeZone`),
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
    assignment_attempt_time_limit_seconds: decodePolicyLimit(
      field(record, "assignment_attempt_time_limit_seconds", path),
      `${path}.assignment_attempt_time_limit_seconds`,
    ),
    attempt_limit: decodePolicyLimit(field(record, "attempt_limit", path), `${path}.attempt_limit`),
    late_work_rule: decodeStringEnum(
      field(record, "late_work_rule", path),
      `${path}.late_work_rule`,
      LATE_POLICIES,
    ),
    assignment_deadline_rule: decodeStringEnum(
      field(record, "assignment_deadline_rule", path),
      `${path}.assignment_deadline_rule`,
      ASSIGNMENT_DEADLINE_RULES,
    ),
  };
}

export function decodeInstructorAssignmentAvailabilityView(
  value: unknown,
  path = "response",
): InstructorAssignmentAvailabilityView {
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

export function decodeStudentAssignmentDetail(
  value: unknown,
  path = "response",
): StudentAssignmentDetail {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "id",
    "reference",
    "title",
    "instructions",
    "time_zone",
    "delivery",
    "entries",
  ]);
  const deliveryRecord = decodeRecord(field(record, "delivery", path), `${path}.delivery`);
  requireOnlyFields(deliveryRecord, `${path}.delivery`, [
    "available_at",
    "due_at",
    "closes_at",
    "assignment_attempt_time_limit_seconds",
    "attempt_limit",
    "late_work_rule",
    "assignment_deadline_rule",
    "student_late_work_status",
  ]);
  const delivery: StudentAssignmentDelivery = {
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
    assignment_attempt_time_limit_seconds: decodePolicyLimit(
      field(deliveryRecord, "assignment_attempt_time_limit_seconds", `${path}.delivery`),
      `${path}.delivery.assignment_attempt_time_limit_seconds`,
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
    assignment_deadline_rule: decodeStringEnum(
      field(deliveryRecord, "assignment_deadline_rule", `${path}.delivery`),
      `${path}.delivery.assignment_deadline_rule`,
      ASSIGNMENT_DEADLINE_RULES,
    ),
    student_late_work_status: decodeStringEnum(
      field(deliveryRecord, "student_late_work_status", `${path}.delivery`),
      `${path}.delivery.student_late_work_status`,
      STUDENT_LATE_WORK_STATUSES,
    ),
  };
  return {
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    reference: decodeAssignmentReference(field(record, "reference", path), `${path}.reference`),
    title: decodeNonemptyString(field(record, "title", path), `${path}.title`),
    instructions: decodeInstructions(field(record, "instructions", path), `${path}.instructions`),
    time_zone: decodeNonemptyString(field(record, "time_zone", path), `${path}.time_zone`),
    delivery,
    entries: decodeArray(field(record, "entries", path), `${path}.entries`, decodeAssignmentEntry),
  };
}

/** Decodes the deliberately identity-free Instructor Student View. */
export function decodeInstructorStudentView(
  value: unknown,
  path = "response",
): InstructorStudentView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "title",
    "instructions",
    "timeZone",
    "delivery",
    "questionsPerAssignmentAttempt",
    "questionPoolReuseRule",
    "questionVariationRule",
    "studentFeedbackReleaseRule",
  ]);
  const deliveryRecord = decodeRecord(field(record, "delivery", path), `${path}.delivery`);
  requireOnlyFields(deliveryRecord, `${path}.delivery`, [
    "available_at",
    "due_at",
    "closes_at",
    "assignment_attempt_time_limit_seconds",
    "attempt_limit",
    "late_work_rule",
    "assignment_deadline_rule",
  ]);
  const delivery: InstructorStudentViewDelivery = {
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
    assignment_attempt_time_limit_seconds: decodePolicyLimit(
      field(deliveryRecord, "assignment_attempt_time_limit_seconds", `${path}.delivery`),
      `${path}.delivery.assignment_attempt_time_limit_seconds`,
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
    assignment_deadline_rule: decodeStringEnum(
      field(deliveryRecord, "assignment_deadline_rule", `${path}.delivery`),
      `${path}.delivery.assignment_deadline_rule`,
      ASSIGNMENT_DEADLINE_RULES,
    ),
  };
  return {
    title: decodeNonemptyString(field(record, "title", path), `${path}.title`),
    instructions: decodeInstructions(field(record, "instructions", path), `${path}.instructions`),
    timeZone: decodeNonemptyString(field(record, "timeZone", path), `${path}.timeZone`),
    delivery,
    questionsPerAssignmentAttempt: decodeNonnegativeInteger(
      field(record, "questionsPerAssignmentAttempt", path),
      `${path}.questionsPerAssignmentAttempt`,
    ),
    questionPoolReuseRule: decodeStringEnum(
      field(record, "questionPoolReuseRule", path),
      `${path}.questionPoolReuseRule`,
      ["reuseSelection", "selectAgain"] as const,
    ),
    questionVariationRule: decodeStringEnum(
      field(record, "questionVariationRule", path),
      `${path}.questionVariationRule`,
      ["reuseVariation", "newVariation"] as const,
    ),
    studentFeedbackReleaseRule: decodeStudentFeedbackReleaseRule(
      field(record, "studentFeedbackReleaseRule", path),
      `${path}.studentFeedbackReleaseRule`,
    ),
  } satisfies InstructorStudentView;
}
