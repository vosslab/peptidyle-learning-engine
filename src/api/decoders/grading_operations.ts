// Strict browser decoders for Instructor-facing automated-grading recovery data.

import { MAX_TEACHING_DISPLAY_LABEL_UNICODE_SCALARS } from "../../../generated/api/MAX_TEACHING_DISPLAY_LABEL_UNICODE_SCALARS";
import type { CourseMembershipReference } from "../../../generated/api/CourseMembershipReference";
import type { GradingOperationAction } from "../../../generated/api/GradingOperationAction";
import type { GradingOperationReason } from "../../../generated/api/GradingOperationReason";
import type { InstructorGradingOperationReference } from "../../../generated/api/InstructorGradingOperationReference";
import type { InstructorGradingOperationState } from "../../../generated/api/InstructorGradingOperationState";
import type { QuestionId } from "../../../generated/api/QuestionId";
import {
  DecodeError,
  decodeNullable,
  decodeRecord,
  decodeSafeInteger,
  decodeString,
  decodeStringEnum,
  decodeUuid,
} from "../decoder";
import {
  MAX_CURSOR_PAGE_ITEMS,
  decodeBoundedArray,
  decodeCursor,
  decodeEnvelopeTitle,
  decodeQuestionId,
  field,
  requireOnlyFields,
} from "./shared";

const MAX_U32 = 4_294_967_295;
const MAX_ROUTE_REFERENCE = MAX_U32;
const MAX_AFFECTED_STUDENTS = MAX_U32;
const MAX_SERVER_SAFE_INTEGER = Number.MAX_SAFE_INTEGER;
const REASONS = [
  "grader_contract_failure",
  "grader_execution_failure",
  "issued_evidence_integrity",
  "retry_exhausted",
  "scoring_recalculation_requested",
  "instructor_requested_recalculation",
  "scoring_recalculation_failed",
] as const satisfies ReadonlyArray<GradingOperationReason>;
const STATES = [
  "actionable",
  "action_in_progress",
  "completed",
  "repair_required",
  "failed",
  "superseded",
] as const satisfies ReadonlyArray<InstructorGradingOperationState>;
const ACTIONS = ["retry", "recalculate"] as const satisfies ReadonlyArray<GradingOperationAction>;

export type GradingOperationFocus = "question" | "student";
export type GradingOperationStrongEtag = string;
export type GradingOperationActionId = string;

export interface InstructorGradingOperation {
  readonly reference: InstructorGradingOperationReference;
  readonly reason: GradingOperationReason;
  readonly state: InstructorGradingOperationState;
  readonly revision: number;
  readonly nextAction: GradingOperationAction | null;
}

export type GradingOperationSubject =
  | { readonly kind: "question"; readonly questionId: QuestionId; readonly title: string }
  | {
      readonly kind: "student";
      readonly membership: CourseMembershipReference;
      readonly displayName: string;
    }
  | { readonly kind: "assignment" };

export type GradingOperationTrustGeneration =
  | { readonly kind: "execution"; readonly generation: number }
  | { readonly kind: "assignmentScoring"; readonly generation: number };

export interface InstructorGradingOperationRow {
  readonly operation: InstructorGradingOperation;
  readonly subject: GradingOperationSubject;
  readonly affectedStudentCount: number;
  readonly trustGeneration: GradingOperationTrustGeneration;
}

export interface InstructorGradingOperationsPage {
  readonly items: ReadonlyArray<InstructorGradingOperationRow>;
  readonly nextCursor: string | null;
}

export type GradingOperationActionReceipt =
  | {
      readonly kind: "retry";
      readonly action: GradingOperationActionId;
      readonly operation: InstructorGradingOperationReference;
      readonly resultingOperationRevision: number;
      readonly occurredAt: number;
    }
  | {
      readonly kind: "recalculation";
      readonly action: GradingOperationActionId;
      readonly operation: InstructorGradingOperationReference;
      readonly resultingOperationRevision: number;
      readonly assignmentRevision: number;
      readonly scoringGeneration: number;
      readonly occurredAt: number;
    };

function closed(
  value: unknown,
  path: string,
  keys: ReadonlyArray<string>,
): Record<string, unknown> {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, keys);
  for (const key of keys) field(record, key, path);
  return record;
}

function routeReference(value: unknown, path: string, prefix: "GO" | "M"): string {
  const reference = decodeString(value, path);
  const pattern = new RegExp(`^${prefix}-[1-9][0-9]{0,9}$`, "u");
  const numericPart = reference.slice(prefix.length + 1);
  if (!pattern.test(reference) || Number(numericPart) > MAX_ROUTE_REFERENCE) {
    throw new DecodeError(path, `a canonical ${prefix}- route reference`);
  }
  return reference;
}

function positiveSafeInteger(value: unknown, path: string, label: string): number {
  const decoded = decodeSafeInteger(value, path);
  if (decoded < 1 || decoded > MAX_SERVER_SAFE_INTEGER) {
    throw new DecodeError(path, `a positive browser-safe ${label}`);
  }
  return decoded;
}

function boundedText(value: unknown, path: string): string {
  const text = decodeString(value, path);
  if (
    text.trim() !== text ||
    text.length === 0 ||
    Array.from(text).length > MAX_TEACHING_DISPLAY_LABEL_UNICODE_SCALARS
  ) {
    throw new DecodeError(
      path,
      `trimmed nonblank text no longer than ${MAX_TEACHING_DISPLAY_LABEL_UNICODE_SCALARS} Unicode scalars`,
    );
  }
  return text;
}

function decodeOperation(value: unknown, path: string): InstructorGradingOperation {
  const record = closed(value, path, ["reference", "reason", "state", "revision", "nextAction"]);
  return {
    reference: routeReference(field(record, "reference", path), `${path}.reference`, "GO"),
    reason: decodeStringEnum(field(record, "reason", path), `${path}.reason`, REASONS),
    state: decodeStringEnum(field(record, "state", path), `${path}.state`, STATES),
    revision: positiveSafeInteger(field(record, "revision", path), `${path}.revision`, "revision"),
    nextAction: decodeNullable(
      field(record, "nextAction", path),
      `${path}.nextAction`,
      (item, itemPath) => decodeStringEnum(item, itemPath, ACTIONS),
    ),
  };
}

function decodeSubject(value: unknown, path: string): GradingOperationSubject {
  const record = decodeRecord(value, path);
  const subjectKind = decodeString(field(record, "kind", path), `${path}.kind`);
  switch (subjectKind) {
    case "question":
      requireOnlyFields(record, path, ["kind", "questionId", "title"]);
      return {
        kind: subjectKind,
        questionId: decodeQuestionId(field(record, "questionId", path), `${path}.questionId`),
        title: decodeEnvelopeTitle(field(record, "title", path), `${path}.title`),
      };
    case "student":
      requireOnlyFields(record, path, ["kind", "membership", "displayName"]);
      return {
        kind: subjectKind,
        membership: routeReference(field(record, "membership", path), `${path}.membership`, "M"),
        displayName: boundedText(field(record, "displayName", path), `${path}.displayName`),
      };
    case "assignment":
      requireOnlyFields(record, path, ["kind"]);
      return { kind: subjectKind };
    default:
      throw new DecodeError(`${path}.kind`, "a known grading-operation subject kind");
  }
}

function decodeTrustGeneration(value: unknown, path: string): GradingOperationTrustGeneration {
  const record = decodeRecord(value, path);
  const generationKind = decodeString(field(record, "kind", path), `${path}.kind`);
  if (generationKind !== "execution" && generationKind !== "assignmentScoring") {
    throw new DecodeError(`${path}.kind`, "a known grading-operation generation kind");
  }
  requireOnlyFields(record, path, ["kind", "generation"]);
  return {
    kind: generationKind,
    generation: positiveSafeInteger(
      field(record, "generation", path),
      `${path}.generation`,
      "generation",
    ),
  };
}

function decodeRow(value: unknown, path: string): InstructorGradingOperationRow {
  const record = closed(value, path, [
    "operation",
    "subject",
    "affectedStudentCount",
    "trustGeneration",
  ]);
  const affectedStudentCount = decodeSafeInteger(
    field(record, "affectedStudentCount", path),
    `${path}.affectedStudentCount`,
  );
  if (affectedStudentCount < 0 || affectedStudentCount > MAX_AFFECTED_STUDENTS) {
    throw new DecodeError(`${path}.affectedStudentCount`, "a bounded affected Student count");
  }
  return {
    operation: decodeOperation(field(record, "operation", path), `${path}.operation`),
    subject: decodeSubject(field(record, "subject", path), `${path}.subject`),
    affectedStudentCount,
    trustGeneration: decodeTrustGeneration(
      field(record, "trustGeneration", path),
      `${path}.trustGeneration`,
    ),
  };
}

export function decodeInstructorGradingOperationsPage(
  value: unknown,
  path = "response",
): InstructorGradingOperationsPage {
  const record = closed(value, path, ["items", "nextCursor"]);
  return {
    items: decodeBoundedArray(
      field(record, "items", path),
      `${path}.items`,
      MAX_CURSOR_PAGE_ITEMS,
      decodeRow,
    ),
    nextCursor: decodeNullable(
      field(record, "nextCursor", path),
      `${path}.nextCursor`,
      decodeCursor,
    ),
  };
}

export function decodeInstructorGradingOperationReference(
  value: unknown,
  path = "operation",
): InstructorGradingOperationReference {
  return routeReference(value, path, "GO");
}

export function decodeGradingOperationActionId(
  value: unknown,
  path = "idempotencyKey",
): GradingOperationActionId {
  return decodeUuid(value, path);
}

export function decodeGradingOperationStrongEtag(
  value: unknown,
  path = "revision",
): GradingOperationStrongEtag {
  const etag = decodeString(value, path);
  if (
    !/^"[1-9][0-9]{0,18}"$/u.test(etag) ||
    BigInt(etag.slice(1, -1)) > 9_223_372_036_854_775_807n
  ) {
    throw new DecodeError(path, "a strong positive PostgreSQL bigint ETag");
  }
  return etag;
}

export function decodeGradingOperationActionReceipt(
  value: unknown,
  path = "response",
): GradingOperationActionReceipt {
  const record = decodeRecord(value, path);
  const receiptKind = decodeString(field(record, "kind", path), `${path}.kind`);
  if (receiptKind === "retry") {
    requireOnlyFields(record, path, [
      "kind",
      "action",
      "operation",
      "resultingOperationRevision",
      "occurredAt",
    ]);
    return {
      kind: receiptKind,
      action: decodeGradingOperationActionId(field(record, "action", path), `${path}.action`),
      operation: decodeInstructorGradingOperationReference(
        field(record, "operation", path),
        `${path}.operation`,
      ),
      resultingOperationRevision: positiveSafeInteger(
        field(record, "resultingOperationRevision", path),
        `${path}.resultingOperationRevision`,
        "operation revision",
      ),
      occurredAt: decodeSafeInteger(field(record, "occurredAt", path), `${path}.occurredAt`),
    };
  }
  if (receiptKind === "recalculation") {
    requireOnlyFields(record, path, [
      "kind",
      "action",
      "operation",
      "resultingOperationRevision",
      "assignmentRevision",
      "scoringGeneration",
      "occurredAt",
    ]);
    return {
      kind: receiptKind,
      action: decodeGradingOperationActionId(field(record, "action", path), `${path}.action`),
      operation: decodeInstructorGradingOperationReference(
        field(record, "operation", path),
        `${path}.operation`,
      ),
      resultingOperationRevision: positiveSafeInteger(
        field(record, "resultingOperationRevision", path),
        `${path}.resultingOperationRevision`,
        "operation revision",
      ),
      assignmentRevision: positiveSafeInteger(
        field(record, "assignmentRevision", path),
        `${path}.assignmentRevision`,
        "assignment revision",
      ),
      scoringGeneration: positiveSafeInteger(
        field(record, "scoringGeneration", path),
        `${path}.scoringGeneration`,
        "scoring generation",
      ),
      occurredAt: decodeSafeInteger(field(record, "occurredAt", path), `${path}.occurredAt`),
    };
  }
  throw new DecodeError(`${path}.kind`, "a known grading-operation action receipt kind");
}
