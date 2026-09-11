// Strict browser decoding for the bounded Assignment Workspace routes.

import { MAX_ASSIGNMENT_INSTRUCTIONS_UNICODE_SCALARS } from "../../../generated/api/MAX_ASSIGNMENT_INSTRUCTIONS_UNICODE_SCALARS";
import type { AssignmentEditNumber } from "../../../generated/api/AssignmentEditNumber";
import type { LocalDateAndTime } from "../../../generated/api/LocalDateAndTime";
import type { LateWorkRule } from "../../../generated/api/LateWorkRule";
import type { AccountTimeZone } from "../../../generated/api/AccountTimeZone";
import type { AssignmentActivityRules } from "../../../generated/api/AssignmentActivityRules";
import type { StudentFeedbackReleaseRule } from "../../../generated/api/StudentFeedbackReleaseRule";
import type {
  AssignmentPreview,
  AssignmentQuestionPickerEntry,
  AssignmentReleaseValidation,
  AuthoredAssignmentQuestion,
  CourseAssignmentSummary,
  CreateLiveAssignmentInput,
  DueSoonAssignmentSummary,
  DueSoonAssignments,
  LiveAssignmentStatus,
  LiveAssignmentWorkspace,
  ReleasedLiveAssignment,
  SaveLiveAssignmentInlineInput,
  SaveLiveAssignmentInput,
} from "../assignment_release";
import {
  DecodeError,
  decodeArray,
  decodeBoolean,
  decodeFiniteNumber,
  decodePositiveInteger,
  decodeRecord,
  decodeString,
} from "../decoder";
import {
  decodeAssignmentReference,
  decodeAssignmentTitle,
  decodeCourseInstanceReference,
  decodeCourseName,
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

function localDateAndTime(value: unknown, path: string): LocalDateAndTime | null {
  if (value === null) return null;
  const decoded = decodeString(value, path);
  if (!/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}$/u.test(decoded)) {
    throw new DecodeError(path, "a canonical local YYYY-MM-DDTHH:MM:SS.sss value or null");
  }
  return decoded;
}

function lateWorkRule(value: unknown, path: string): LateWorkRule {
  if (value === "accept" || value === "mark_late" || value === "reject") return value;
  throw new DecodeError(path, "a canonical Late Work Rule");
}

function optionalPositiveInteger(value: unknown, path: string): number | null {
  if (value === null) return null;
  return decodePositiveInteger(value, path);
}

function activityRules(value: unknown, path: string): AssignmentActivityRules {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "assignmentCompletionRule",
    "assignmentAttemptGradeRule",
    "assignmentAttemptContinuationRule",
    "questionPoolReuseRule",
    "questionVariationRule",
    "assignmentAttemptResumeRule",
    "assignmentQuestionDisplayRule",
    "assignmentNavigationRule",
    "assignmentQuestionOrderRule",
  ]);
  return record as AssignmentActivityRules;
}

function feedbackRules(value: unknown, path: string): StudentFeedbackReleaseRule {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "score",
    "per_item_correctness",
    "submitted_response",
    "question_feedback",
    "question_answer",
    "question_answer_explanation",
    "class_statistics",
  ]);
  for (const name of Object.keys(record)) {
    const timing = field(record, name, path);
    if (
      !["during_attempt", "after_submit", "after_due", "after_close", "never"].includes(
        String(timing),
      )
    ) {
      throw new DecodeError(`${path}.${name}`, "a Student Feedback Release timing");
    }
  }
  return record as StudentFeedbackReleaseRule;
}

function displayTimeZone(value: unknown, path: string): AccountTimeZone {
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

function courseAssignmentSummary(value: unknown, path: string): CourseAssignmentSummary {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "reference",
    "title",
    "dueAt",
    "displayTimeZone",
    "status",
    "editNumber",
  ]);
  return {
    reference: decodeAssignmentReference(field(record, "reference", path), `${path}.reference`),
    title: decodeAssignmentTitle(field(record, "title", path), `${path}.title`),
    dueAt: localDateAndTime(field(record, "dueAt", path), `${path}.dueAt`),
    displayTimeZone: displayTimeZone(
      field(record, "displayTimeZone", path),
      `${path}.displayTimeZone`,
    ),
    status: status(field(record, "status", path), `${path}.status`),
    editNumber: editNumber(field(record, "editNumber", path), `${path}.editNumber`),
  };
}

function dueAtMillis(value: unknown, path: string): number {
  const decoded = decodeFiniteNumber(value, path);
  if (!Number.isSafeInteger(decoded))
    throw new DecodeError(path, "a safe Unix millisecond instant");
  return decoded;
}

function dueSoonAssignmentSummary(value: unknown, path: string): DueSoonAssignmentSummary {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "courseReference",
    "courseLongName",
    "assignmentReference",
    "assignmentTitle",
    "assignmentStatus",
    "dueAtMillis",
  ]);
  return {
    courseReference: decodeCourseInstanceReference(
      field(record, "courseReference", path),
      `${path}.courseReference`,
    ),
    courseLongName: decodeCourseName(
      field(record, "courseLongName", path),
      `${path}.courseLongName`,
    ),
    assignmentReference: decodeAssignmentReference(
      field(record, "assignmentReference", path),
      `${path}.assignmentReference`,
    ),
    assignmentTitle: decodeAssignmentTitle(
      field(record, "assignmentTitle", path),
      `${path}.assignmentTitle`,
    ),
    assignmentStatus: status(field(record, "assignmentStatus", path), `${path}.assignmentStatus`),
    dueAtMillis: dueAtMillis(field(record, "dueAtMillis", path), `${path}.dueAtMillis`),
  };
}

/** Decodes the bounded cross-Course Due Soon response without accepting hidden pagination. */
export function decodeDueSoonAssignments(value: unknown, path = "response"): DueSoonAssignments {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["items", "nextCursor", "displayTimeZone"]);
  if (field(record, "nextCursor", path) !== null) {
    throw new DecodeError(`${path}.nextCursor`, "null for the bounded Due Soon list");
  }
  return {
    items: decodeArray(field(record, "items", path), `${path}.items`, dueSoonAssignmentSummary),
    nextCursor: null,
    displayTimeZone: displayTimeZone(
      field(record, "displayTimeZone", path),
      `${path}.displayTimeZone`,
    ),
  };
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

/** Validates the intentionally narrow mutable Course Assignment list-row input. */
export function decodeSaveLiveAssignmentInlineInput(
  value: unknown,
  path = "request",
): SaveLiveAssignmentInlineInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["title", "dueAt"]);
  return {
    title: decodeAssignmentTitle(field(record, "title", path), `${path}.title`),
    dueAt: localDateAndTime(field(record, "dueAt", path), `${path}.dueAt`),
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
    "assignmentAttemptTimeLimitSeconds",
    "attemptLimit",
    "activityRules",
    "studentFeedbackReleaseRule",
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
    dueAt: localDateAndTime(field(record, "dueAt", path), `${path}.dueAt`),
    lateWorkRule: lateWorkRule(field(record, "lateWorkRule", path), `${path}.lateWorkRule`),
    assignmentAttemptTimeLimitSeconds: optionalPositiveInteger(
      field(record, "assignmentAttemptTimeLimitSeconds", path),
      `${path}.assignmentAttemptTimeLimitSeconds`,
    ),
    attemptLimit: optionalPositiveInteger(
      field(record, "attemptLimit", path),
      `${path}.attemptLimit`,
    ),
    activityRules: activityRules(field(record, "activityRules", path), `${path}.activityRules`),
    studentFeedbackReleaseRule: feedbackRules(
      field(record, "studentFeedbackReleaseRule", path),
      `${path}.studentFeedbackReleaseRule`,
    ),
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
    "assignmentAttemptTimeLimitSeconds",
    "attemptLimit",
    "activityRules",
    "studentFeedbackReleaseRule",
    "displayTimeZone",
    "questions",
  ]);
  return {
    reference: decodeAssignmentReference(field(record, "reference", path), `${path}.reference`),
    editNumber: editNumber(field(record, "editNumber", path), `${path}.editNumber`),
    status: status(field(record, "status", path), `${path}.status`),
    title: decodeAssignmentTitle(field(record, "title", path), `${path}.title`),
    instructions: instructions(field(record, "instructions", path), `${path}.instructions`),
    dueAt: localDateAndTime(field(record, "dueAt", path), `${path}.dueAt`),
    lateWorkRule: lateWorkRule(field(record, "lateWorkRule", path), `${path}.lateWorkRule`),
    assignmentAttemptTimeLimitSeconds: optionalPositiveInteger(
      field(record, "assignmentAttemptTimeLimitSeconds", path),
      `${path}.assignmentAttemptTimeLimitSeconds`,
    ),
    attemptLimit: optionalPositiveInteger(
      field(record, "attemptLimit", path),
      `${path}.attemptLimit`,
    ),
    activityRules: activityRules(field(record, "activityRules", path), `${path}.activityRules`),
    studentFeedbackReleaseRule: feedbackRules(
      field(record, "studentFeedbackReleaseRule", path),
      `${path}.studentFeedbackReleaseRule`,
    ),
    displayTimeZone: displayTimeZone(
      field(record, "displayTimeZone", path),
      `${path}.displayTimeZone`,
    ),
    questions: decodeArray(field(record, "questions", path), `${path}.questions`, authoredQuestion),
  };
}

export function decodeAssignmentQuestionPicker(
  value: unknown,
  path = "response",
): ReadonlyArray<AssignmentQuestionPickerEntry> {
  return decodeArray(value, path, pickerEntry);
}

export function decodeCourseAssignments(
  value: unknown,
  path = "response",
): ReadonlyArray<CourseAssignmentSummary> {
  return decodeArray(value, path, courseAssignmentSummary);
}

/** Decodes the same closed row shape returned by the inline row save. */
export function decodeCourseAssignmentSummary(
  value: unknown,
  path = "response",
): CourseAssignmentSummary {
  return courseAssignmentSummary(value, path);
}

export function decodeAssignmentReleaseValidation(
  value: unknown,
  path = "response",
): AssignmentReleaseValidation {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["canRelease", "issues"]);
  const issues = decodeArray(field(record, "issues", path), `${path}.issues`, (item, itemPath) => {
    if (
      item !== "noPublishedQuestions" &&
      item !== "questionUnavailable" &&
      item !== "timeLimitRequired"
    ) {
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
