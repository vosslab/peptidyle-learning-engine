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
import { decodeAssignmentEntry, decodeAssignmentReference } from "./question_library";

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

const LATE_POLICIES = ["accept", "markLate", "reject"] as const;
const ASSIGNMENT_DEADLINE_RULES = ["autoSubmit"] as const;
const LATE_STATUSES = ["on_time", "accepted_late", "marked_late"] as const;
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
    "availableAt",
    "dueAt",
    "closesAt",
    "assignmentAttemptTimeLimitSeconds",
    "attemptLimit",
    "lateWorkRule",
    "assignmentDeadlineRule",
  ]);
  return {
    timeZone: decodeNonemptyString(field(record, "timeZone", path), `${path}.timeZone`),
    instructions: decodeInstructions(field(record, "instructions", path), `${path}.instructions`),
    availableAt: decodeNullable(
      field(record, "availableAt", path),
      `${path}.availableAt`,
      decodeLocalTime,
    ),
    dueAt: decodeNullable(field(record, "dueAt", path), `${path}.dueAt`, decodeLocalTime),
    closesAt: decodeNullable(field(record, "closesAt", path), `${path}.closesAt`, decodeLocalTime),
    assignmentAttemptTimeLimitSeconds: decodePolicyLimit(
      field(record, "assignmentAttemptTimeLimitSeconds", path),
      `${path}.assignmentAttemptTimeLimitSeconds`,
    ),
    attemptLimit: decodePolicyLimit(field(record, "attemptLimit", path), `${path}.attemptLimit`),
    lateWorkRule: decodeStringEnum(
      field(record, "lateWorkRule", path),
      `${path}.lateWorkRule`,
      LATE_POLICIES,
    ),
    assignmentDeadlineRule: decodeStringEnum(
      field(record, "assignmentDeadlineRule", path),
      `${path}.assignmentDeadlineRule`,
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
    requireOnlyFields(record, path, ["state", "availableAt"]);
    return {
      state,
      availableAt: decodeLocalTime(field(record, "availableAt", path), `${path}.availableAt`),
    };
  }
  if (state === "closed") {
    requireOnlyFields(record, path, ["state", "closedAt"]);
    return {
      state,
      closedAt: decodeNullable(
        field(record, "closedAt", path),
        `${path}.closedAt`,
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
    "late_status",
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
    late_status: decodeStringEnum(
      field(deliveryRecord, "late_status", `${path}.delivery`),
      `${path}.delivery.late_status`,
      LATE_STATUSES,
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
    "availableAt",
    "dueAt",
    "closesAt",
    "assignmentAttemptTimeLimitSeconds",
    "attemptLimit",
    "lateWorkRule",
    "assignmentDeadlineRule",
  ]);
  const delivery: InstructorStudentViewDelivery = {
    availableAt: decodeNullable(
      field(deliveryRecord, "availableAt", `${path}.delivery`),
      `${path}.delivery.availableAt`,
      decodeTimestamp,
    ),
    dueAt: decodeNullable(
      field(deliveryRecord, "dueAt", `${path}.delivery`),
      `${path}.delivery.dueAt`,
      decodeTimestamp,
    ),
    closesAt: decodeNullable(
      field(deliveryRecord, "closesAt", `${path}.delivery`),
      `${path}.delivery.closesAt`,
      decodeTimestamp,
    ),
    assignmentAttemptTimeLimitSeconds: decodePolicyLimit(
      field(deliveryRecord, "assignmentAttemptTimeLimitSeconds", `${path}.delivery`),
      `${path}.delivery.assignmentAttemptTimeLimitSeconds`,
    ),
    attemptLimit: decodePolicyLimit(
      field(deliveryRecord, "attemptLimit", `${path}.delivery`),
      `${path}.delivery.attemptLimit`,
    ),
    lateWorkRule: decodeStringEnum(
      field(deliveryRecord, "lateWorkRule", `${path}.delivery`),
      `${path}.delivery.lateWorkRule`,
      LATE_POLICIES,
    ),
    assignmentDeadlineRule: decodeStringEnum(
      field(deliveryRecord, "assignmentDeadlineRule", `${path}.delivery`),
      `${path}.delivery.assignmentDeadlineRule`,
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
