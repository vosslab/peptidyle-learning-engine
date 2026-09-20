// Strict decoding for the answer-free, no-write Instructor Student View.

import type { InstructorStudentView } from "../../../generated/api/InstructorStudentView";
import type { InstructorStudentViewDelivery } from "../../../generated/api/InstructorStudentViewDelivery";
import type { InstructorStudentViewEntry } from "../../../generated/api/InstructorStudentViewEntry";
import type { InstructorStudentViewQuestion } from "../../../generated/api/InstructorStudentViewQuestion";
import { MAX_ASSESSMENT_ORDERED_ENTRIES } from "../../../generated/api/MAX_ASSESSMENT_ORDERED_ENTRIES";
import { MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY } from "../../../generated/api/MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY";
import {
  DecodeError,
  decodeNonnegativeInteger,
  decodeNullable,
  decodePositiveInteger,
  decodeRecord,
  decodeString,
  decodeStringEnum,
} from "../decoder";
import {
  decodeAssessmentTitle,
  decodeBoundedArray,
  decodeQuestionRevisionTuple,
  decodeTimestamp,
  field,
  requireOnlyFields,
} from "./shared";
import { decodeAccountTimeZone } from "./student_assessment_decision";

const ASSESSMENT_STATUSES = ["unreleased", "released", "closed", "archived"] as const;
const LATE_WORK_RULES = ["accept", "mark_late", "reject"] as const;

function decodeAssessmentEditNumber(value: unknown, path: string): string {
  const assessmentEditNumber = decodeString(value, path);
  if (
    !/^[1-9][0-9]*$/u.test(assessmentEditNumber) ||
    BigInt(assessmentEditNumber) > 9_223_372_036_854_775_807n
  ) {
    throw new DecodeError(path, "a positive Assessment Edit Number");
  }
  return assessmentEditNumber;
}

function decodeInstructions(value: unknown, path: string): string {
  const instructions = decodeString(value, path);
  if (instructions.includes("\0") || Array.from(instructions).length > 50_000) {
    throw new DecodeError(path, "plain-text instructions no longer than 50000 Unicode scalars");
  }
  return instructions;
}

function decodePolicyLimit(value: unknown, path: string): number | null {
  return decodeNullable(value, path, (entry, entryPath) => {
    const limit = decodePositiveInteger(entry, entryPath);
    if (limit > 2_147_483_647) throw new DecodeError(entryPath, "a bounded positive integer");
    return limit;
  });
}

function decodeDelivery(value: unknown, path: string): InstructorStudentViewDelivery {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "available_at",
    "due_at",
    "closes_at",
    "assessment_attempt_time_limit_seconds",
    "attempt_limit",
    "late_work_rule",
  ]);
  return {
    available_at: decodeNullable(
      field(record, "available_at", path),
      `${path}.available_at`,
      decodeTimestamp,
    ),
    due_at: decodeNullable(field(record, "due_at", path), `${path}.due_at`, decodeTimestamp),
    closes_at: decodeNullable(
      field(record, "closes_at", path),
      `${path}.closes_at`,
      decodeTimestamp,
    ),
    assessment_attempt_time_limit_seconds: decodePolicyLimit(
      field(record, "assessment_attempt_time_limit_seconds", path),
      `${path}.assessment_attempt_time_limit_seconds`,
    ),
    attempt_limit: decodePolicyLimit(field(record, "attempt_limit", path), `${path}.attempt_limit`),
    late_work_rule: decodeStringEnum(
      field(record, "late_work_rule", path),
      `${path}.late_work_rule`,
      LATE_WORK_RULES,
    ),
  };
}

function decodeInstructorStudentViewQuestion(
  value: unknown,
  path: string,
): InstructorStudentViewQuestion {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["position", "questionRevisionTuple"]);
  return {
    position: decodePositiveInteger(field(record, "position", path), `${path}.position`),
    questionRevisionTuple: decodeQuestionRevisionTuple(
      field(record, "questionRevisionTuple", path),
      `${path}.questionRevisionTuple`,
      true,
    ),
  };
}

function decodeEntry(value: unknown, path: string): InstructorStudentViewEntry {
  const record = decodeRecord(value, path);
  const entryKind = decodeString(field(record, "kind", path), `${path}.kind`);
  const authoredPosition = decodeNonnegativeInteger(
    field(record, "authoredPosition", path),
    `${path}.authoredPosition`,
  );
  if (entryKind === "presented") {
    requireOnlyFields(record, path, ["kind", "authoredPosition", "questions"]);
    const questions = decodeBoundedArray(
      field(record, "questions", path),
      `${path}.questions`,
      MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY,
      decodeInstructorStudentViewQuestion,
    );
    if (questions.length === 0) {
      throw new DecodeError(`${path}.questions`, "one or more presented Questions");
    }
    return { kind: "presented", authoredPosition, questions };
  }
  if (entryKind === "notShown") {
    requireOnlyFields(record, path, ["kind", "authoredPosition", "reason"]);
    return {
      kind: "notShown",
      authoredPosition,
      reason: decodeStringEnum(field(record, "reason", path), `${path}.reason`, [
        "assessmentEntryUnavailable",
      ] as const),
    };
  }
  throw new DecodeError(`${path}.kind`, "presented or notShown");
}

/** Decode one closed manifest and enforce both authored and flattened order. */
export function decodeInstructorStudentView(
  value: unknown,
  path = "response",
): InstructorStudentView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "assessmentEditNumber",
    "status",
    "title",
    "instructions",
    "displayTimeZone",
    "delivery",
    "entries",
  ]);
  const entries = decodeBoundedArray(
    field(record, "entries", path),
    `${path}.entries`,
    MAX_ASSESSMENT_ORDERED_ENTRIES,
    decodeEntry,
  );
  const questionPositions: number[] = [];
  entries.forEach((entry, index) => {
    if (entry.authoredPosition !== index) {
      throw new DecodeError(
        `${path}.entries[${index}].authoredPosition`,
        `the ordered position ${index}`,
      );
    }
    if (entry.kind === "presented") {
      questionPositions.push(...entry.questions.map((question) => question.position));
    }
  });
  const orderedPositions = [...questionPositions].sort((left, right) => left - right);
  orderedPositions.forEach((position, index) => {
    if (position !== index + 1) {
      throw new DecodeError(
        `${path}.entries`,
        "unique flattened Question positions numbered consecutively from 1",
      );
    }
  });
  return {
    assessmentEditNumber: decodeAssessmentEditNumber(
      field(record, "assessmentEditNumber", path),
      `${path}.assessmentEditNumber`,
    ),
    status: decodeStringEnum(field(record, "status", path), `${path}.status`, ASSESSMENT_STATUSES),
    title: decodeAssessmentTitle(field(record, "title", path), `${path}.title`),
    instructions: decodeInstructions(field(record, "instructions", path), `${path}.instructions`),
    displayTimeZone: decodeAccountTimeZone(
      field(record, "displayTimeZone", path),
      `${path}.displayTimeZone`,
    ),
    delivery: decodeDelivery(field(record, "delivery", path), `${path}.delivery`),
    entries,
  };
}
