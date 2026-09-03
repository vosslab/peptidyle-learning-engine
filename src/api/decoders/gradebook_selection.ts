// Strict browser contracts for Gradebook Student and submitted Assignment Attempt selection.

import { MAX_TEACHING_DISPLAY_LABEL_UNICODE_SCALARS } from "../../../generated/api/MAX_TEACHING_DISPLAY_LABEL_UNICODE_SCALARS";
import type { AssignmentReference } from "../../../generated/api/AssignmentReference";
import type { CourseMembershipReference } from "../../../generated/api/CourseMembershipReference";
import type { CourseRosterChangeNumber } from "../../../generated/api/CourseRosterChangeNumber";
import type { InstructorGradingOperationReference } from "../../../generated/api/InstructorGradingOperationReference";
import type { AssignmentAttemptReference } from "../../../generated/api/AssignmentAttemptReference";
import {
  DecodeError,
  decodeBoolean,
  decodeRecord,
  decodeSafeInteger,
  decodeString,
  decodeStringEnum,
} from "../decoder";
import {
  MAX_CURSOR_PAGE_ITEMS,
  decodeBoundedArray,
  decodeCursor,
  decodeTimestamp,
  field,
  requireOnlyFields,
} from "./shared";

const ASSIGNMENT_ATTEMPT_SELECTION_BASES = [
  "first",
  "latest",
  "highest",
  "instructorSelected",
] as const;

export type AssignmentInspectionChoice =
  | {
      readonly kind: "selectedAssignmentAttempt";
      readonly basis: (typeof ASSIGNMENT_ATTEMPT_SELECTION_BASES)[number];
      readonly assignmentAttempt: AssignmentAttemptReference;
      readonly submittedAt: number;
    }
  | { readonly kind: "chooseAssignmentAttempt"; readonly completedAssignmentAttemptCount: number }
  | { readonly kind: "noSubmittedAssignmentAttempt" };

export type GradebookSelectionFilter =
  | { readonly kind: "assignment"; readonly assignment: AssignmentReference }
  | { readonly kind: "operation"; readonly operation: InstructorGradingOperationReference };

export interface GradebookSelectionQuery {
  readonly cursor?: string;
  readonly pageSize?: number;
  readonly filter: GradebookSelectionFilter;
}

export interface StudentSelectionRow {
  readonly membership: CourseMembershipReference;
  readonly displayLabel: string;
  readonly assignment: AssignmentReference;
  readonly inspectionChoice: AssignmentInspectionChoice;
}

export type GradebookSelectionResult =
  | {
      readonly kind: "singleStudent";
      readonly membership: CourseMembershipReference;
      readonly assignment: AssignmentReference;
      readonly inspectionChoice: AssignmentInspectionChoice;
    }
  | {
      readonly kind: "studentSelection";
      readonly rows: ReadonlyArray<StudentSelectionRow>;
      readonly nextCursor: string | null;
    };

export interface SubmittedAssignmentAttemptChoice {
  readonly assignmentAttempt: AssignmentAttemptReference;
  readonly submittedAt: number;
  readonly scoreSelected: boolean;
}

export interface SubmittedAssignmentAttemptChoicesQuery {
  readonly cursor?: string;
  readonly pageSize?: number;
  readonly operationRef?: InstructorGradingOperationReference;
}

export interface SubmittedAssignmentAttemptChoicesPage {
  readonly rosterChangeNumber: CourseRosterChangeNumber;
  readonly nextCursor: string | null;
  readonly rows: ReadonlyArray<SubmittedAssignmentAttemptChoice>;
}

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

function optionalField(record: Record<string, unknown>, key: string): unknown {
  return Object.prototype.hasOwnProperty.call(record, key) ? record[key] : undefined;
}

function publicReference(value: unknown, path: string, prefix: "A" | "M" | "R"): string {
  const reference = decodeString(value, path);
  const pattern = new RegExp(`^${prefix}-[1-9][0-9]{0,9}$`, "u");
  const numericPart = reference.slice(prefix.length + 1);
  if (!pattern.test(reference) || Number(numericPart) > 2_147_483_647) {
    throw new DecodeError(path, `a canonical ${prefix}- reference`);
  }
  return reference;
}

function positiveSafeInteger(value: unknown, path: string): number {
  const decoded = decodeSafeInteger(value, path);
  if (decoded < 1) throw new DecodeError(path, "a positive browser-safe integer");
  return decoded;
}

function decodeCourseRosterChangeNumber(value: unknown, path: string): CourseRosterChangeNumber {
  const parsed = decodeString(value, path);
  if (!/^[1-9][0-9]{0,18}$/u.test(parsed) || BigInt(parsed) > 9_223_372_036_854_775_807n) {
    throw new DecodeError(path, "a canonical positive PostgreSQL bigint decimal");
  }
  return parsed;
}

function boundedDisplayLabel(value: unknown, path: string): string {
  const label = decodeString(value, path);
  if (
    label.trim() !== label ||
    label.length === 0 ||
    Array.from(label).length > MAX_TEACHING_DISPLAY_LABEL_UNICODE_SCALARS
  ) {
    throw new DecodeError(
      path,
      `trimmed text of 1 to ${MAX_TEACHING_DISPLAY_LABEL_UNICODE_SCALARS} Unicode scalars`,
    );
  }
  return label;
}

export function decodeAssignmentInspectionChoice(
  value: unknown,
  path: string,
): AssignmentInspectionChoice {
  const record = decodeRecord(value, path);
  const choiceKind = decodeString(field(record, "kind", path), `${path}.kind`);
  switch (choiceKind) {
    case "selectedAssignmentAttempt":
      requireOnlyFields(record, path, ["kind", "basis", "assignmentAttempt", "submittedAt"]);
      return {
        kind: choiceKind,
        basis: decodeStringEnum(
          field(record, "basis", path),
          `${path}.basis`,
          ASSIGNMENT_ATTEMPT_SELECTION_BASES,
        ),
        assignmentAttempt: publicReference(
          field(record, "assignmentAttempt", path),
          `${path}.assignmentAttempt`,
          "R",
        ),
        submittedAt: decodeTimestamp(field(record, "submittedAt", path), `${path}.submittedAt`),
      };
    case "chooseAssignmentAttempt":
      requireOnlyFields(record, path, ["kind", "completedAssignmentAttemptCount"]);
      return {
        kind: choiceKind,
        completedAssignmentAttemptCount: positiveSafeInteger(
          field(record, "completedAssignmentAttemptCount", path),
          `${path}.completedAssignmentAttemptCount`,
        ),
      };
    case "noSubmittedAssignmentAttempt":
      requireOnlyFields(record, path, ["kind"]);
      return { kind: choiceKind };
    default:
      throw new DecodeError(`${path}.kind`, "a known Gradebook inspection choice");
  }
}

function decodeStudentSelectionRow(value: unknown, path: string): StudentSelectionRow {
  const record = closed(value, path, [
    "membership",
    "displayLabel",
    "assignment",
    "inspectionChoice",
  ]);
  return {
    membership: publicReference(field(record, "membership", path), `${path}.membership`, "M"),
    displayLabel: boundedDisplayLabel(field(record, "displayLabel", path), `${path}.displayLabel`),
    assignment: publicReference(field(record, "assignment", path), `${path}.assignment`, "A"),
    inspectionChoice: decodeAssignmentInspectionChoice(
      field(record, "inspectionChoice", path),
      `${path}.inspectionChoice`,
    ),
  };
}

export function decodeGradebookSelectionResult(
  value: unknown,
  path = "response",
): GradebookSelectionResult {
  const record = decodeRecord(value, path);
  const resultKind = decodeString(field(record, "kind", path), `${path}.kind`);
  if (resultKind === "singleStudent") {
    requireOnlyFields(record, path, ["kind", "membership", "assignment", "inspectionChoice"]);
    return {
      kind: resultKind,
      membership: publicReference(field(record, "membership", path), `${path}.membership`, "M"),
      assignment: publicReference(field(record, "assignment", path), `${path}.assignment`, "A"),
      inspectionChoice: decodeAssignmentInspectionChoice(
        field(record, "inspectionChoice", path),
        `${path}.inspectionChoice`,
      ),
    };
  }
  if (resultKind !== "studentSelection") {
    throw new DecodeError(`${path}.kind`, "singleStudent or studentSelection");
  }
  requireOnlyFields(record, path, ["kind", "rows", "nextCursor"]);
  const nextCursor = optionalField(record, "nextCursor");
  return {
    kind: resultKind,
    rows: decodeBoundedArray(
      field(record, "rows", path),
      `${path}.rows`,
      MAX_CURSOR_PAGE_ITEMS,
      decodeStudentSelectionRow,
    ),
    nextCursor: nextCursor === undefined ? null : decodeCursor(nextCursor, `${path}.nextCursor`),
  };
}

function decodeSubmittedAssignmentAttemptChoice(
  value: unknown,
  path: string,
): SubmittedAssignmentAttemptChoice {
  const record = closed(value, path, ["assignmentAttempt", "submittedAt", "scoreSelected"]);
  return {
    assignmentAttempt: publicReference(
      field(record, "assignmentAttempt", path),
      `${path}.assignmentAttempt`,
      "R",
    ),
    submittedAt: decodeTimestamp(field(record, "submittedAt", path), `${path}.submittedAt`),
    scoreSelected: decodeBoolean(field(record, "scoreSelected", path), `${path}.scoreSelected`),
  };
}

export function decodeSubmittedAssignmentAttemptChoicesPage(
  value: unknown,
  path = "response",
): SubmittedAssignmentAttemptChoicesPage {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["rosterChangeNumber", "nextCursor", "rows"]);
  const nextCursor = optionalField(record, "nextCursor");
  return {
    rosterChangeNumber: decodeCourseRosterChangeNumber(
      field(record, "rosterChangeNumber", path),
      `${path}.rosterChangeNumber`,
    ),
    nextCursor: nextCursor === undefined ? null : decodeCursor(nextCursor, `${path}.nextCursor`),
    rows: decodeBoundedArray(
      field(record, "rows", path),
      `${path}.rows`,
      MAX_CURSOR_PAGE_ITEMS,
      decodeSubmittedAssignmentAttemptChoice,
    ),
  };
}
