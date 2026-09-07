// Strict browser decoding for the bounded M10 Assignment Workspace routes.

import { MAX_ASSIGNMENT_INSTRUCTIONS_UNICODE_SCALARS } from "../../../generated/api/MAX_ASSIGNMENT_INSTRUCTIONS_UNICODE_SCALARS";
import type { AssignmentEditNumber } from "../../../generated/api/AssignmentEditNumber";
import type { CourseLocalDateAndTime } from "../../../generated/api/CourseLocalDateAndTime";
import type { LateWorkRule } from "../../../generated/api/LateWorkRule";
import type {
  AssignmentPreview,
  AssignmentQuestionPickerEntry,
  AssignmentReleaseValidation,
  AuthoredAssignmentQuestion,
  CreateLiveAssignmentInput,
  LiveAssignmentStatus,
  LiveAssignmentWorkspace,
  ReleasedLiveAssignment,
  SaveLiveAssignmentInput,
} from "../assignment_release";
import {
  DecodeError,
  decodeArray,
  decodeBoolean,
  decodePositiveInteger,
  decodeRecord,
  decodeString,
} from "../decoder";
import {
  decodeAssignmentReference,
  decodeAssignmentTitle,
  decodeQuestionDescription,
  decodeQuestionId,
  field,
  requireOnlyFields,
} from "./shared";

const MAX_QUESTIONS = 25;

function instructions(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  if (
    decoded.includes("\0") ||
    Array.from(decoded).length > MAX_ASSIGNMENT_INSTRUCTIONS_UNICODE_SCALARS
  ) {
    throw new DecodeError(path, "bounded plain-text Assignment Instructions");
  }
  return decoded;
}

function editNumber(value: unknown, path: string): AssignmentEditNumber {
  const decoded = decodeString(value, path);
  if (!/^[1-9][0-9]*$/u.test(decoded) || BigInt(decoded) > 9_223_372_036_854_775_807n) {
    throw new DecodeError(path, "a positive Assignment Edit Number");
  }
  return decoded;
}

function courseLocalDateAndTime(value: unknown, path: string): CourseLocalDateAndTime | null {
  if (value === null) return null;
  const decoded = decodeString(value, path);
  if (!/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}$/u.test(decoded)) {
    throw new DecodeError(path, "a canonical course-local YYYY-MM-DDTHH:MM:SS.sss value or null");
  }
  return decoded;
}

function lateWorkRule(value: unknown, path: string): LateWorkRule {
  if (value === "accept" || value === "mark_late" || value === "reject") return value;
  throw new DecodeError(path, "a canonical Late Work Rule");
}

function pickerEntry(value: unknown, path: string): AssignmentQuestionPickerEntry {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["questionId", "description"]);
  return {
    questionId: decodeQuestionId(field(record, "questionId", path), `${path}.questionId`),
    description: decodeQuestionDescription(
      field(record, "description", path),
      `${path}.description`,
    ),
  };
}

function authoredQuestion(value: unknown, path: string): AuthoredAssignmentQuestion {
  return pickerEntry(value, path);
}

function status(value: unknown, path: string): LiveAssignmentStatus {
  const decoded = decodeString(value, path);
  if (
    decoded !== "unreleased" &&
    decoded !== "released" &&
    decoded !== "closed" &&
    decoded !== "archived"
  ) {
    throw new DecodeError(path, "a current Assignment Status");
  }
  return decoded;
}

/** Validates authored input before it leaves the browser's bounded workspace. */
export function decodeCreateLiveAssignmentInput(
  value: unknown,
  path = "request",
): CreateLiveAssignmentInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["title", "instructions"]);
  return {
    title: decodeAssignmentTitle(field(record, "title", path), `${path}.title`),
    instructions: instructions(field(record, "instructions", path), `${path}.instructions`),
  };
}

export function decodeSaveLiveAssignmentInput(
  value: unknown,
  path = "request",
): SaveLiveAssignmentInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "title",
    "instructions",
    "questionIds",
    "dueAt",
    "lateWorkRule",
  ]);
  const questionIds = decodeArray(
    field(record, "questionIds", path),
    `${path}.questionIds`,
    (item, itemPath) => decodeQuestionId(item, itemPath),
  );
  if (questionIds.length > MAX_QUESTIONS || new Set(questionIds).size !== questionIds.length) {
    throw new DecodeError(`${path}.questionIds`, "at most twenty-five unique Question IDs");
  }
  return {
    title: decodeAssignmentTitle(field(record, "title", path), `${path}.title`),
    instructions: instructions(field(record, "instructions", path), `${path}.instructions`),
    questionIds,
    dueAt: courseLocalDateAndTime(field(record, "dueAt", path), `${path}.dueAt`),
    lateWorkRule: lateWorkRule(field(record, "lateWorkRule", path), `${path}.lateWorkRule`),
  };
}

export function decodeLiveAssignmentWorkspace(
  value: unknown,
  path = "response",
): LiveAssignmentWorkspace {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "reference",
    "editNumber",
    "status",
    "title",
    "instructions",
    "dueAt",
    "lateWorkRule",
    "questions",
  ]);
  return {
    reference: decodeAssignmentReference(field(record, "reference", path), `${path}.reference`),
    editNumber: editNumber(field(record, "editNumber", path), `${path}.editNumber`),
    status: status(field(record, "status", path), `${path}.status`),
    title: decodeAssignmentTitle(field(record, "title", path), `${path}.title`),
    instructions: instructions(field(record, "instructions", path), `${path}.instructions`),
    dueAt: courseLocalDateAndTime(field(record, "dueAt", path), `${path}.dueAt`),
    lateWorkRule: lateWorkRule(field(record, "lateWorkRule", path), `${path}.lateWorkRule`),
    questions: decodeArray(field(record, "questions", path), `${path}.questions`, authoredQuestion),
  };
}

export function decodeAssignmentQuestionPicker(
  value: unknown,
  path = "response",
): ReadonlyArray<AssignmentQuestionPickerEntry> {
  return decodeArray(value, path, pickerEntry);
}

export function decodeAssignmentReleaseValidation(
  value: unknown,
  path = "response",
): AssignmentReleaseValidation {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["canRelease", "issues"]);
  const issues = decodeArray(field(record, "issues", path), `${path}.issues`, (item, itemPath) => {
    if (item !== "noPublishedQuestions" && item !== "questionUnavailable") {
      throw new DecodeError(itemPath, "a current Assignment Release issue");
    }
    return item;
  });
  return {
    canRelease: decodeBoolean(field(record, "canRelease", path), `${path}.canRelease`),
    issues,
  };
}

export function decodeAssignmentPreview(value: unknown, path = "response"): AssignmentPreview {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["title", "instructions", "questions"]);
  return {
    title: decodeAssignmentTitle(field(record, "title", path), `${path}.title`),
    instructions: instructions(field(record, "instructions", path), `${path}.instructions`),
    questions: decodeArray(field(record, "questions", path), `${path}.questions`, authoredQuestion),
  };
}

export function decodeReleasedLiveAssignment(
  value: unknown,
  path = "response",
): ReleasedLiveAssignment {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["reference", "revisionNumber"]);
  return {
    reference: decodeAssignmentReference(field(record, "reference", path), `${path}.reference`),
    revisionNumber: decodePositiveInteger(
      field(record, "revisionNumber", path),
      `${path}.revisionNumber`,
    ),
  };
}
