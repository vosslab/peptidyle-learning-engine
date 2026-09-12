// Strict browser decoding for the current Assignment Workspace routes.

import { MAX_ASSIGNMENT_INSTRUCTIONS_UNICODE_SCALARS } from "../../../generated/api/MAX_ASSIGNMENT_INSTRUCTIONS_UNICODE_SCALARS";
import { MAX_ASSIGNMENT_ORDERED_ENTRIES } from "../../../generated/api/MAX_ASSIGNMENT_ORDERED_ENTRIES";
import { MAX_ASSIGNMENT_QUESTION_POOL_ITEMS } from "../../../generated/api/MAX_ASSIGNMENT_QUESTION_POOL_ITEMS";
import { MAX_QUESTION_POOL_ITEMS_PER_ASSIGNMENT_ENTRY } from "../../../generated/api/MAX_QUESTION_POOL_ITEMS_PER_ASSIGNMENT_ENTRY";
import type { AssignmentActivityRules } from "../../../generated/api/AssignmentActivityRules";
import type { AssignmentEntry } from "../../../generated/api/AssignmentEntry";
import type { AssignmentEntryAvailability } from "../../../generated/api/AssignmentEntryAvailability";
import type { AssignmentEntryScoringRule } from "../../../generated/api/AssignmentEntryScoringRule";
import type { AssignmentPointValue } from "../../../generated/api/AssignmentPointValue";
import type { AccountTimeZone } from "../../../generated/api/AccountTimeZone";
import type { BlueprintCourseReference } from "../../../generated/api/BlueprintCourseReference";
import type { BlueprintRevision } from "../../../generated/api/BlueprintRevision";
import type { LateWorkRule } from "../../../generated/api/LateWorkRule";
import type { LocalDateAndTime } from "../../../generated/api/LocalDateAndTime";
import type { QuestionPoolItem } from "../../../generated/api/QuestionPoolItem";
import type { QuestionPoolItemAvailability } from "../../../generated/api/QuestionPoolItemAvailability";
import type { QuestionPoolSelectedQuestionOrder } from "../../../generated/api/QuestionPoolSelectedQuestionOrder";
import type {
  AssignmentPreview,
  AssignmentQuestionPickerEntry,
  AssignmentReleaseValidation,
  AssignmentUnreleaseImpact,
  AuthoredAssignmentQuestion,
  BlueprintAssignmentSource,
  CourseAssignmentSourceChoice,
  CourseAssignmentSummary,
  CreateLiveAssignmentInput,
  DueSoonAssignmentSummary,
  DueSoonAssignments,
  LiveAssignmentStatus,
  LiveAssignmentWorkspace,
  SaveLiveAssignmentInlineInput,
  SaveLiveAssignmentInput,
  UnreleasedLiveAssignment,
} from "../assignment_release";
import {
  DecodeError,
  decodeArray,
  decodeBoolean,
  decodeFiniteNumber,
  decodeNonnegativeInteger,
  decodePositiveInteger,
  decodeRecord,
  decodeString,
  decodeStringEnum,
} from "../decoder";
import { decodeStudentFeedbackReleaseRule } from "./assignment_policy";
import { decodeQuestionAttemptLimit, decodeQuestionAttemptTimeLimit } from "./question_model";
import {
  decodeAssignmentReference,
  decodeAssignmentTitle,
  decodeBoundedArray,
  decodeCourseInstanceReference,
  decodeCourseName,
  decodeIdentifier,
  decodeQuestionDescription,
  decodeQuestionRevisionReference,
  field,
  requireOnlyFields,
} from "./shared";

/** Decode bounded plain-text Assignment Instructions retained by current work and Attempts. */
export function decodeAssignmentInstructions(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  if (
    decoded.includes("\0") ||
    Array.from(decoded).length > MAX_ASSIGNMENT_INSTRUCTIONS_UNICODE_SCALARS
  ) {
    throw new DecodeError(path, "bounded plain-text Assignment Instructions");
  }
  return decoded;
}

function editNumber(value: unknown, path: string): string {
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
  return decodeStringEnum(value, path, ["accept", "mark_late", "reject"]);
}

function optionalPositiveInteger(value: unknown, path: string): number | null {
  return value === null ? null : decodePositiveInteger(value, path);
}

function completionRule(
  value: unknown,
  path: string,
): AssignmentActivityRules["assignmentCompletionRule"] {
  const record = decodeRecord(value, path);
  const kind = decodeString(field(record, "kind", path), `${path}.kind`);
  if (kind === "answerAll" || kind === "allCorrect") {
    requireOnlyFields(record, path, ["kind"]);
    return { kind };
  }
  if (kind === "scoreAtLeast") {
    requireOnlyFields(record, path, ["kind", "fraction"]);
    const fraction = decodeFiniteNumber(field(record, "fraction", path), `${path}.fraction`);
    if (fraction < 0 || fraction > 1)
      throw new DecodeError(`${path}.fraction`, "a fraction from 0 through 1");
    return { kind, fraction };
  }
  throw new DecodeError(`${path}.kind`, "a known Assignment Completion Rule");
}

function continuationRule(
  value: unknown,
  path: string,
): AssignmentActivityRules["assignmentAttemptContinuationRule"] {
  const record = decodeRecord(value, path);
  const kind = decodeString(field(record, "kind", path), `${path}.kind`);
  if (kind === "unlimited" || kind === "closed") {
    requireOnlyFields(record, path, ["kind"]);
    return { kind };
  }
  if (kind === "capped") {
    requireOnlyFields(record, path, ["kind", "maxAdditionalAssignmentAttempts"]);
    return {
      kind,
      maxAdditionalAssignmentAttempts: decodeNonnegativeInteger(
        field(record, "maxAdditionalAssignmentAttempts", path),
        `${path}.maxAdditionalAssignmentAttempts`,
      ),
    };
  }
  throw new DecodeError(`${path}.kind`, "a known Assignment Attempt Continuation Rule");
}

/** Decode the persisted Assignment Activity Rules shared by current work and Attempt evidence. */
export function decodeAssignmentActivityRules(
  value: unknown,
  path: string,
): AssignmentActivityRules {
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
  return {
    assignmentCompletionRule: completionRule(
      field(record, "assignmentCompletionRule", path),
      `${path}.assignmentCompletionRule`,
    ),
    assignmentAttemptGradeRule: decodeStringEnum(
      field(record, "assignmentAttemptGradeRule", path),
      `${path}.assignmentAttemptGradeRule`,
      ["first", "latest", "highest", "instructorSelected"],
    ),
    assignmentAttemptContinuationRule: continuationRule(
      field(record, "assignmentAttemptContinuationRule", path),
      `${path}.assignmentAttemptContinuationRule`,
    ),
    questionPoolReuseRule: decodeStringEnum(
      field(record, "questionPoolReuseRule", path),
      `${path}.questionPoolReuseRule`,
      ["reuseSelection", "selectAgain"],
    ),
    questionVariationRule: decodeStringEnum(
      field(record, "questionVariationRule", path),
      `${path}.questionVariationRule`,
      ["reuseVariation", "newVariation"],
    ),
    assignmentAttemptResumeRule: decodeStringEnum(
      field(record, "assignmentAttemptResumeRule", path),
      `${path}.assignmentAttemptResumeRule`,
      ["resumable", "singleSession"],
    ),
    assignmentQuestionDisplayRule: decodeStringEnum(
      field(record, "assignmentQuestionDisplayRule", path),
      `${path}.assignmentQuestionDisplayRule`,
      ["allQuestions", "oneQuestionAtATime"],
    ),
    assignmentNavigationRule: decodeStringEnum(
      field(record, "assignmentNavigationRule", path),
      `${path}.assignmentNavigationRule`,
      ["freeNavigation", "forwardOnly"],
    ),
    assignmentQuestionOrderRule: decodeStringEnum(
      field(record, "assignmentQuestionOrderRule", path),
      `${path}.assignmentQuestionOrderRule`,
      ["authoredOrder", "shuffled"],
    ),
  };
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

function blueprintCourseReference(value: unknown, path: string): BlueprintCourseReference {
  const decoded = decodeString(value, path);
  if (!/^BP-[1-9][0-9]{0,9}$/u.test(decoded) || Number(decoded.slice(3)) > 2_147_483_647) {
    throw new DecodeError(path, "a canonical Blueprint Course public reference");
  }
  return decoded;
}

function blueprintRevision(value: unknown, path: string): BlueprintRevision {
  const decoded = decodeString(value, path);
  if (!/^[1-9][0-9]*$/u.test(decoded) || BigInt(decoded) > 9_223_372_036_854_775_807n) {
    throw new DecodeError(path, "a positive Blueprint Revision");
  }
  return decoded;
}

function blueprintAssignmentSource(value: unknown, path: string): BlueprintAssignmentSource {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["blueprint_revision", "blueprint_assignment_reference"]);
  const revision = decodeRecord(
    field(record, "blueprint_revision", path),
    `${path}.blueprint_revision`,
  );
  requireOnlyFields(revision, `${path}.blueprint_revision`, ["reference", "revision"]);
  return {
    blueprint_revision: {
      reference: blueprintCourseReference(
        field(revision, "reference", `${path}.blueprint_revision`),
        `${path}.blueprint_revision.reference`,
      ),
      revision: blueprintRevision(
        field(revision, "revision", `${path}.blueprint_revision`),
        `${path}.blueprint_revision.revision`,
      ),
    },
    blueprint_assignment_reference: decodeIdentifier(
      field(record, "blueprint_assignment_reference", path),
      `${path}.blueprint_assignment_reference`,
    ),
  };
}

function pointValue(value: unknown, path: string): AssignmentPointValue {
  const decoded = decodeString(value, path);
  if (
    !/^(?:0|[1-9][0-9]{0,9})(?:\.[0-9]{1,4})?$/u.test(decoded) ||
    BigInt(decoded.split(".")[0] ?? "0") > 1_000_000_000n
  ) {
    throw new DecodeError(path, "a supported nonnegative point decimal with at most four places");
  }
  return decoded;
}

function poolItem(value: unknown, path: string): QuestionPoolItem {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["id", "reference", "availability"]);
  return {
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    reference: decodeQuestionRevisionReference(
      field(record, "reference", path),
      `${path}.reference`,
      true,
    ),
    availability: decodeStringEnum(field(record, "availability", path), `${path}.availability`, [
      "available",
      "retired",
    ] as const satisfies ReadonlyArray<QuestionPoolItemAvailability>),
  };
}

function poolSelectionRule(
  value: unknown,
  path: string,
): { readonly selectedQuestionOrder: QuestionPoolSelectedQuestionOrder } {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["selectedQuestionOrder"]);
  return {
    selectedQuestionOrder: decodeStringEnum(
      field(record, "selectedQuestionOrder", path),
      `${path}.selectedQuestionOrder`,
      ["questionPoolOrder", "randomOrder"],
    ),
  };
}

function assignmentEntry(value: unknown, path: string): AssignmentEntry {
  const record = decodeRecord(value, path);
  const kind = decodeString(field(record, "kind", path), `${path}.kind`);
  if (kind === "fixedQuestion") {
    requireOnlyFields(record, path, [
      "kind",
      "id",
      "reference",
      "pointsPossible",
      "availability",
      "scoringRule",
      "questionAttemptLimit",
      "questionAttemptTimeLimit",
    ]);
    return {
      kind,
      id: decodeIdentifier(field(record, "id", path), `${path}.id`),
      reference: decodeQuestionRevisionReference(
        field(record, "reference", path),
        `${path}.reference`,
        true,
      ),
      pointsPossible: pointValue(field(record, "pointsPossible", path), `${path}.pointsPossible`),
      availability: decodeStringEnum(field(record, "availability", path), `${path}.availability`, [
        "available",
        "retired",
      ] as const satisfies ReadonlyArray<AssignmentEntryAvailability>),
      scoringRule: decodeStringEnum(field(record, "scoringRule", path), `${path}.scoringRule`, [
        "normal",
        "fullCredit",
        "extraCredit",
        "excluded",
      ] as const satisfies ReadonlyArray<AssignmentEntryScoringRule>),
      questionAttemptLimit: decodeQuestionAttemptLimit(
        field(record, "questionAttemptLimit", path),
        `${path}.questionAttemptLimit`,
        true,
      ),
      questionAttemptTimeLimit: decodeQuestionAttemptTimeLimit(
        field(record, "questionAttemptTimeLimit", path),
        `${path}.questionAttemptTimeLimit`,
        true,
      ),
    };
  }
  if (kind !== "questionPool")
    throw new DecodeError(`${path}.kind`, "a known Assignment Entry kind");
  requireOnlyFields(record, path, [
    "kind",
    "id",
    "availability",
    "scoringRule",
    "selectionCount",
    "pointsPerItem",
    "selectionRule",
    "questionAttemptLimit",
    "questionAttemptTimeLimit",
    "items",
  ]);
  const items = decodeBoundedArray(
    field(record, "items", path),
    `${path}.items`,
    MAX_QUESTION_POOL_ITEMS_PER_ASSIGNMENT_ENTRY,
    poolItem,
  );
  const selectionCount = decodePositiveInteger(
    field(record, "selectionCount", path),
    `${path}.selectionCount`,
  );
  if (selectionCount > items.length)
    throw new DecodeError(`${path}.selectionCount`, "no greater than the Question Pool Item count");
  return {
    kind,
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    availability: decodeStringEnum(field(record, "availability", path), `${path}.availability`, [
      "available",
      "retired",
    ] as const satisfies ReadonlyArray<AssignmentEntryAvailability>),
    scoringRule: decodeStringEnum(field(record, "scoringRule", path), `${path}.scoringRule`, [
      "normal",
      "fullCredit",
      "extraCredit",
      "excluded",
    ] as const satisfies ReadonlyArray<AssignmentEntryScoringRule>),
    selectionCount,
    pointsPerItem: pointValue(field(record, "pointsPerItem", path), `${path}.pointsPerItem`),
    selectionRule: poolSelectionRule(field(record, "selectionRule", path), `${path}.selectionRule`),
    questionAttemptLimit: decodeQuestionAttemptLimit(
      field(record, "questionAttemptLimit", path),
      `${path}.questionAttemptLimit`,
      true,
    ),
    questionAttemptTimeLimit: decodeQuestionAttemptTimeLimit(
      field(record, "questionAttemptTimeLimit", path),
      `${path}.questionAttemptTimeLimit`,
      true,
    ),
    items,
  };
}

function entries(value: unknown, path: string): ReadonlyArray<AssignmentEntry> {
  const decoded = decodeBoundedArray(value, path, MAX_ASSIGNMENT_ORDERED_ENTRIES, assignmentEntry);
  const poolItems = decoded.reduce(
    (count, entry) => count + (entry.kind === "questionPool" ? entry.items.length : 0),
    0,
  );
  if (poolItems > MAX_ASSIGNMENT_QUESTION_POOL_ITEMS)
    throw new DecodeError(path, "the supported total number of Question Pool Items");
  return decoded;
}

function pickerEntry(value: unknown, path: string): AssignmentQuestionPickerEntry {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["reference", "description"]);
  return {
    reference: decodeQuestionRevisionReference(
      field(record, "reference", path),
      `${path}.reference`,
      true,
    ),
    description: decodeQuestionDescription(
      field(record, "description", path),
      `${path}.description`,
    ),
  };
}

function courseAssignmentSourceChoice(value: unknown, path: string): CourseAssignmentSourceChoice {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["source", "label"]);
  return {
    source: blueprintAssignmentSource(field(record, "source", path), `${path}.source`),
    // The source choice label is the immutable Blueprint Assignment title,
    // not arbitrary display text.  Keep its browser boundary equal to the
    // title bound PostgreSQL enforces for that authored content.
    label: decodeAssignmentTitle(field(record, "label", path), `${path}.label`),
  };
}

function authoredQuestion(value: unknown, path: string): AuthoredAssignmentQuestion {
  return pickerEntry(value, path);
}

function status(value: unknown, path: string): LiveAssignmentStatus {
  return decodeStringEnum(value, path, ["unreleased", "released", "closed", "archived"]);
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
  const dueAtMillis = decodeFiniteNumber(field(record, "dueAtMillis", path), `${path}.dueAtMillis`);
  if (!Number.isSafeInteger(dueAtMillis))
    throw new DecodeError(`${path}.dueAtMillis`, "a safe Unix millisecond instant");
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
    dueAtMillis,
  };
}

export function decodeDueSoonAssignments(value: unknown, path = "response"): DueSoonAssignments {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["items", "nextCursor", "displayTimeZone"]);
  if (field(record, "nextCursor", path) !== null)
    throw new DecodeError(`${path}.nextCursor`, "null for the bounded Due Soon list");
  return {
    items: decodeArray(field(record, "items", path), `${path}.items`, dueSoonAssignmentSummary),
    nextCursor: null,
    displayTimeZone: displayTimeZone(
      field(record, "displayTimeZone", path),
      `${path}.displayTimeZone`,
    ),
  };
}

export function decodeCreateLiveAssignmentInput(
  value: unknown,
  path = "request",
): CreateLiveAssignmentInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["blueprintAssignmentReference", "title", "instructions"]);
  return {
    blueprintAssignmentReference: decodeIdentifier(
      field(record, "blueprintAssignmentReference", path),
      `${path}.blueprintAssignmentReference`,
    ),
    title: decodeAssignmentTitle(field(record, "title", path), `${path}.title`),
    instructions: decodeAssignmentInstructions(
      field(record, "instructions", path),
      `${path}.instructions`,
    ),
  };
}

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
  // ASVS 1.5.2 and 2.2.1: browser requests use one closed current aggregate.
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "title",
    "instructions",
    "dueAt",
    "availableAt",
    "closesAt",
    "lateWorkRule",
    "assignmentAttemptTimeLimitSeconds",
    "attemptLimit",
    "activityRules",
    "studentFeedbackReleaseRule",
    "entries",
  ]);
  return {
    title: decodeAssignmentTitle(field(record, "title", path), `${path}.title`),
    instructions: decodeAssignmentInstructions(
      field(record, "instructions", path),
      `${path}.instructions`,
    ),
    dueAt: localDateAndTime(field(record, "dueAt", path), `${path}.dueAt`),
    availableAt: localDateAndTime(field(record, "availableAt", path), `${path}.availableAt`),
    closesAt: localDateAndTime(field(record, "closesAt", path), `${path}.closesAt`),
    lateWorkRule: lateWorkRule(field(record, "lateWorkRule", path), `${path}.lateWorkRule`),
    assignmentAttemptTimeLimitSeconds: optionalPositiveInteger(
      field(record, "assignmentAttemptTimeLimitSeconds", path),
      `${path}.assignmentAttemptTimeLimitSeconds`,
    ),
    attemptLimit: optionalPositiveInteger(
      field(record, "attemptLimit", path),
      `${path}.attemptLimit`,
    ),
    activityRules: decodeAssignmentActivityRules(
      field(record, "activityRules", path),
      `${path}.activityRules`,
    ),
    studentFeedbackReleaseRule: decodeStudentFeedbackReleaseRule(
      field(record, "studentFeedbackReleaseRule", path),
      `${path}.studentFeedbackReleaseRule`,
    ),
    entries: entries(field(record, "entries", path), `${path}.entries`),
  };
}

export function decodeLiveAssignmentWorkspace(
  value: unknown,
  path = "response",
): LiveAssignmentWorkspace {
  // ASVS 1.5.2: untrusted transport JSON is allowlisted before UI use.
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "reference",
    "editNumber",
    "status",
    "source",
    "title",
    "instructions",
    "dueAt",
    "availableAt",
    "closesAt",
    "lateWorkRule",
    "assignmentAttemptTimeLimitSeconds",
    "attemptLimit",
    "activityRules",
    "studentFeedbackReleaseRule",
    "displayTimeZone",
    "entries",
    "questions",
  ]);
  return {
    reference: decodeAssignmentReference(field(record, "reference", path), `${path}.reference`),
    editNumber: editNumber(field(record, "editNumber", path), `${path}.editNumber`),
    status: status(field(record, "status", path), `${path}.status`),
    source: blueprintAssignmentSource(field(record, "source", path), `${path}.source`),
    title: decodeAssignmentTitle(field(record, "title", path), `${path}.title`),
    instructions: decodeAssignmentInstructions(
      field(record, "instructions", path),
      `${path}.instructions`,
    ),
    dueAt: localDateAndTime(field(record, "dueAt", path), `${path}.dueAt`),
    availableAt: localDateAndTime(field(record, "availableAt", path), `${path}.availableAt`),
    closesAt: localDateAndTime(field(record, "closesAt", path), `${path}.closesAt`),
    lateWorkRule: lateWorkRule(field(record, "lateWorkRule", path), `${path}.lateWorkRule`),
    assignmentAttemptTimeLimitSeconds: optionalPositiveInteger(
      field(record, "assignmentAttemptTimeLimitSeconds", path),
      `${path}.assignmentAttemptTimeLimitSeconds`,
    ),
    attemptLimit: optionalPositiveInteger(
      field(record, "attemptLimit", path),
      `${path}.attemptLimit`,
    ),
    activityRules: decodeAssignmentActivityRules(
      field(record, "activityRules", path),
      `${path}.activityRules`,
    ),
    studentFeedbackReleaseRule: decodeStudentFeedbackReleaseRule(
      field(record, "studentFeedbackReleaseRule", path),
      `${path}.studentFeedbackReleaseRule`,
    ),
    displayTimeZone: displayTimeZone(
      field(record, "displayTimeZone", path),
      `${path}.displayTimeZone`,
    ),
    entries: entries(field(record, "entries", path), `${path}.entries`),
    questions: decodeBoundedArray(
      field(record, "questions", path),
      `${path}.questions`,
      MAX_ASSIGNMENT_ORDERED_ENTRIES,
      authoredQuestion,
    ),
  };
}

export function decodeAssignmentQuestionPicker(
  value: unknown,
  path = "response",
): ReadonlyArray<AssignmentQuestionPickerEntry> {
  return decodeArray(value, path, pickerEntry);
}

export function decodeCourseAssignmentSourceChoices(
  value: unknown,
  path = "response",
): ReadonlyArray<CourseAssignmentSourceChoice> {
  return decodeArray(value, path, courseAssignmentSourceChoice);
}
export function decodeCourseAssignments(
  value: unknown,
  path = "response",
): ReadonlyArray<CourseAssignmentSummary> {
  return decodeArray(value, path, courseAssignmentSummary);
}
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
  return {
    canRelease: decodeBoolean(field(record, "canRelease", path), `${path}.canRelease`),
    issues: decodeArray(field(record, "issues", path), `${path}.issues`, (item, itemPath) =>
      decodeStringEnum(item, itemPath, [
        "noPublishedQuestions",
        "questionUnavailable",
        "timeLimitRequired",
      ]),
    ),
  };
}

/** Decode the released-only aggregate confirmation projection. */
export function decodeAssignmentUnreleaseImpact(
  value: unknown,
  path = "response",
): AssignmentUnreleaseImpact {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "confirmationTitle",
    "editNumber",
    "attemptCount",
    "submissionCount",
    "gradeCount",
  ]);
  return {
    confirmationTitle: decodeAssignmentTitle(
      field(record, "confirmationTitle", path),
      `${path}.confirmationTitle`,
    ),
    editNumber: editNumber(field(record, "editNumber", path), `${path}.editNumber`),
    attemptCount: decodeNonnegativeInteger(
      field(record, "attemptCount", path),
      `${path}.attemptCount`,
    ),
    submissionCount: decodeNonnegativeInteger(
      field(record, "submissionCount", path),
      `${path}.submissionCount`,
    ),
    gradeCount: decodeNonnegativeInteger(field(record, "gradeCount", path), `${path}.gradeCount`),
  };
}

/** Decode an accepted Unrelease receipt without exposing Student Work detail. */
export function decodeUnreleasedLiveAssignment(
  value: unknown,
  path = "response",
): UnreleasedLiveAssignment {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["assignment", "deleted"]);
  return {
    assignment: decodeLiveAssignmentWorkspace(
      field(record, "assignment", path),
      `${path}.assignment`,
    ),
    deleted: decodeAssignmentUnreleaseImpact(field(record, "deleted", path), `${path}.deleted`),
  };
}

export function decodeAssignmentPreview(value: unknown, path = "response"): AssignmentPreview {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["title", "instructions", "questions"]);
  return {
    title: decodeAssignmentTitle(field(record, "title", path), `${path}.title`),
    instructions: decodeAssignmentInstructions(
      field(record, "instructions", path),
      `${path}.instructions`,
    ),
    questions: decodeBoundedArray(
      field(record, "questions", path),
      `${path}.questions`,
      MAX_ASSIGNMENT_ORDERED_ENTRIES,
      authoredQuestion,
    ),
  };
}
