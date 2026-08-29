// Strict Student delivery and focused assignment-policy transport decoders.

import type { AssignmentTeachingSettingsValidationFailure } from "../../../generated/api/AssignmentTeachingSettingsValidationFailure";
import type { InstructorAssignmentTeachingSettingsLocal } from "../../../generated/api/InstructorAssignmentTeachingSettingsLocal";
import type { InstructorAssignmentCurrentState } from "../../../generated/api/InstructorAssignmentCurrentState";
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
import { decodeStudentDisclosurePolicy } from "./assignment_policy";
import {
  decodeAssignmentItem,
  decodeAssignmentReference,
  decodeAssignmentSelectionGroup,
} from "./catalog_course";

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

const LIFECYCLES = ["draft", "published", "closed", "archived"] as const;
const LATE_POLICIES = ["accept", "markLate", "reject"] as const;
const DEADLINE_BEHAVIORS = ["autoSubmit"] as const;
const LATE_STATUSES = ["on_time", "accepted_late", "marked_late"] as const;
const SETTINGS_FAILURE_FIELDS = [
  "teachingSettings",
  "timeZone",
  "availableAt",
  "dueAt",
  "closesAt",
  "schedule",
  "timeLimitSeconds",
  "attemptLimit",
  "lifecycle",
  "instructions",
] as const;
const SETTINGS_FAILURE_REASONS = [
  "invalidInput",
  "courseTimeZoneMismatch",
  "outsideCourseTerm",
  "nonexistentLocalTime",
  "ambiguousLocalTime",
  "timestampOutOfRange",
  "scheduleOutOfOrder",
  "timeLimitOutOfRange",
  "attemptLimitOutOfRange",
  "illegalLifecycleTransition",
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

export function decodeAssignmentTeachingSettingsValidationFailure(
  value: unknown,
  path = "response",
): AssignmentTeachingSettingsValidationFailure {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["error", "field", "reason", "message"]);
  return {
    error: decodeStringEnum(field(record, "error", path), `${path}.error`, [
      "assignmentTeachingSettingsInvalid",
    ] as const),
    field: decodeStringEnum(field(record, "field", path), `${path}.field`, SETTINGS_FAILURE_FIELDS),
    reason: decodeStringEnum(
      field(record, "reason", path),
      `${path}.reason`,
      SETTINGS_FAILURE_REASONS,
    ),
    message: decodeNonemptyString(field(record, "message", path), `${path}.message`),
  };
}

export function decodeInstructorAssignmentTeachingSettingsLocal(
  value: unknown,
  path = "response",
): InstructorAssignmentTeachingSettingsLocal {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "timeZone",
    "lifecycle",
    "instructions",
    "availableAt",
    "dueAt",
    "closesAt",
    "timeLimitSeconds",
    "attemptLimit",
    "lateSubmission",
    "deadlineBehavior",
  ]);
  return {
    timeZone: decodeNonemptyString(field(record, "timeZone", path), `${path}.timeZone`),
    lifecycle: decodeStringEnum(field(record, "lifecycle", path), `${path}.lifecycle`, LIFECYCLES),
    instructions: decodeInstructions(field(record, "instructions", path), `${path}.instructions`),
    availableAt: decodeNullable(
      field(record, "availableAt", path),
      `${path}.availableAt`,
      decodeLocalTime,
    ),
    dueAt: decodeNullable(field(record, "dueAt", path), `${path}.dueAt`, decodeLocalTime),
    closesAt: decodeNullable(field(record, "closesAt", path), `${path}.closesAt`, decodeLocalTime),
    timeLimitSeconds: decodePolicyLimit(
      field(record, "timeLimitSeconds", path),
      `${path}.timeLimitSeconds`,
    ),
    attemptLimit: decodePolicyLimit(field(record, "attemptLimit", path), `${path}.attemptLimit`),
    lateSubmission: decodeStringEnum(
      field(record, "lateSubmission", path),
      `${path}.lateSubmission`,
      LATE_POLICIES,
    ),
    deadlineBehavior: decodeStringEnum(
      field(record, "deadlineBehavior", path),
      `${path}.deadlineBehavior`,
      DEADLINE_BEHAVIORS,
    ),
  };
}

export function decodeInstructorAssignmentCurrentState(
  value: unknown,
  path = "response",
): InstructorAssignmentCurrentState {
  const record = decodeRecord(value, path);
  const state = decodeStringEnum(field(record, "state", path), `${path}.state`, [
    "draft",
    "scheduled",
    "open",
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
    "items",
    "selection_groups",
  ]);
  const deliveryRecord = decodeRecord(field(record, "delivery", path), `${path}.delivery`);
  requireOnlyFields(deliveryRecord, `${path}.delivery`, [
    "available_at",
    "due_at",
    "closes_at",
    "time_limit_seconds",
    "attempt_limit",
    "late_submission",
    "deadline_behavior",
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
    time_limit_seconds: decodePolicyLimit(
      field(deliveryRecord, "time_limit_seconds", `${path}.delivery`),
      `${path}.delivery.time_limit_seconds`,
    ),
    attempt_limit: decodePolicyLimit(
      field(deliveryRecord, "attempt_limit", `${path}.delivery`),
      `${path}.delivery.attempt_limit`,
    ),
    late_submission: decodeStringEnum(
      field(deliveryRecord, "late_submission", `${path}.delivery`),
      `${path}.delivery.late_submission`,
      LATE_POLICIES,
    ),
    deadline_behavior: decodeStringEnum(
      field(deliveryRecord, "deadline_behavior", `${path}.delivery`),
      `${path}.delivery.deadline_behavior`,
      DEADLINE_BEHAVIORS,
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
    items: decodeArray(field(record, "items", path), `${path}.items`, decodeAssignmentItem),
    selection_groups: decodeArray(
      field(record, "selection_groups", path),
      `${path}.selection_groups`,
      decodeAssignmentSelectionGroup,
    ),
  };
}

/** Decodes the deliberately identity-free Instructor Student-view projection. */
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
    "questionsPerRun",
    "variation",
    "disclosurePolicy",
  ]);
  const deliveryRecord = decodeRecord(field(record, "delivery", path), `${path}.delivery`);
  requireOnlyFields(deliveryRecord, `${path}.delivery`, [
    "availableAt",
    "dueAt",
    "closesAt",
    "timeLimitSeconds",
    "attemptLimit",
    "lateSubmission",
    "deadlineBehavior",
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
    timeLimitSeconds: decodePolicyLimit(
      field(deliveryRecord, "timeLimitSeconds", `${path}.delivery`),
      `${path}.delivery.timeLimitSeconds`,
    ),
    attemptLimit: decodePolicyLimit(
      field(deliveryRecord, "attemptLimit", `${path}.delivery`),
      `${path}.delivery.attemptLimit`,
    ),
    lateSubmission: decodeStringEnum(
      field(deliveryRecord, "lateSubmission", `${path}.delivery`),
      `${path}.delivery.lateSubmission`,
      LATE_POLICIES,
    ),
    deadlineBehavior: decodeStringEnum(
      field(deliveryRecord, "deadlineBehavior", `${path}.delivery`),
      `${path}.delivery.deadlineBehavior`,
      DEADLINE_BEHAVIORS,
    ),
  };
  return {
    title: decodeNonemptyString(field(record, "title", path), `${path}.title`),
    instructions: decodeInstructions(field(record, "instructions", path), `${path}.instructions`),
    timeZone: decodeNonemptyString(field(record, "timeZone", path), `${path}.timeZone`),
    delivery,
    questionsPerRun: decodeNonnegativeInteger(
      field(record, "questionsPerRun", path),
      `${path}.questionsPerRun`,
    ),
    variation: decodeStringEnum(field(record, "variation", path), `${path}.variation`, [
      "newSeeds",
      "selectedProblemVariants",
      "fullRegeneration",
    ] as const),
    disclosurePolicy: decodeStudentDisclosurePolicy(
      field(record, "disclosurePolicy", path),
      `${path}.disclosurePolicy`,
    ),
  } satisfies InstructorStudentView;
}
