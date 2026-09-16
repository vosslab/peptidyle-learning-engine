// Strict browser decoding for the current Assessment Workspace routes.

import { MAX_ASSESSMENT_INSTRUCTIONS_UNICODE_SCALARS } from "../../../generated/api/MAX_ASSESSMENT_INSTRUCTIONS_UNICODE_SCALARS";
import { MAX_ASSESSMENT_ORDERED_ENTRIES } from "../../../generated/api/MAX_ASSESSMENT_ORDERED_ENTRIES";
import type { AssessmentActivityRules } from "../../../generated/api/AssessmentActivityRules";
import { ASSESSMENT_TYPE_VALUES, type AssessmentType } from "../../../generated/api/AssessmentType";
import type { AssessmentEntry } from "../../../generated/api/AssessmentEntry";
import type { AssessmentOrigin } from "../../../generated/api/AssessmentOrigin";
import type { AssessmentEntryAvailability } from "../../../generated/api/AssessmentEntryAvailability";
import type { AssessmentEntryScoringRule } from "../../../generated/api/AssessmentEntryScoringRule";
import type { AssessmentPointValue } from "../../../generated/api/AssessmentPointValue";
import type { AccountTimeZone } from "../../../generated/api/AccountTimeZone";
import type { BlueprintAssessmentSource } from "../../../generated/api/BlueprintAssessmentSource";
import type { BlueprintCourseReference } from "../../../generated/api/BlueprintCourseReference";
import type { BlueprintRevision } from "../../../generated/api/BlueprintRevision";
import type { LateWorkRule } from "../../../generated/api/LateWorkRule";
import type { LocalDateAndTime } from "../../../generated/api/LocalDateAndTime";
import type { QuestionPoolRevisionReference } from "../../../generated/api/QuestionPoolRevisionReference";
import type { QuestionPoolSelectedQuestionOrder } from "../../../generated/api/QuestionPoolSelectedQuestionOrder";
import type {
  AssessmentBlueprintUpdateContent,
  AssessmentBlueprintUpdateEntry,
  AssessmentBlueprintUpdateReview,
  ApplyAssessmentBlueprintUpdateInput,
  AssessmentQuestionPickerEntry,
  AssessmentReleaseValidation,
  AssessmentUnreleaseImpact,
  AuthoredAssessmentQuestion,
  CourseAssessmentSummary,
  CreateLiveAssessmentInput,
  DueSoonAssessmentSummary,
  DueSoonAssessments,
  LiveAssessmentStatus,
  LiveAssessmentWorkspace,
  SaveLiveAssessmentInlineInput,
  SaveBaseAssessmentPolicyInput,
  SaveLiveAssessmentInput,
  UnreleasedLiveAssessment,
} from "../assessment_release";
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
import { decodeStudentFeedbackReleaseRule } from "./assessment_policy";
import { decodeQuestionAttemptLimit, decodeQuestionAttemptTimeLimit } from "./question_model";
import {
  decodeAssessmentReference,
  decodeAssessmentTitle,
  decodeBoundedArray,
  decodeCourseInstanceReference,
  decodeCourseName,
  decodeIdentifier,
  decodeQuestionDescription,
  decodeQuestionId,
  decodePositiveQuestionRevisionNumber,
  decodeQuestionRevisionReference,
  field,
  requireOnlyFields,
} from "./shared";

/** Decode bounded plain-text Assessment Instructions retained by current work and Attempts. */
export function decodeAssessmentInstructions(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  if (
    decoded.includes("\0") ||
    Array.from(decoded).length > MAX_ASSESSMENT_INSTRUCTIONS_UNICODE_SCALARS
  ) {
    throw new DecodeError(path, "bounded plain-text Assessment Instructions");
  }
  return decoded;
}

export function decodeApplyAssessmentBlueprintUpdateInput(
  value: unknown,
  path = "input",
): ApplyAssessmentBlueprintUpdateInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["expectedSourceRevision", "expectedEditNumber"]);
  return {
    expectedSourceRevision: blueprintRevision(
      field(record, "expectedSourceRevision", path),
      `${path}.expectedSourceRevision`,
    ),
    expectedEditNumber: editNumber(
      field(record, "expectedEditNumber", path),
      `${path}.expectedEditNumber`,
    ),
  };
}

function blueprintUpdateEntry(value: unknown, path: string): AssessmentBlueprintUpdateEntry {
  const record = decodeRecord(value, path);
  const kind = decodeStringEnum(field(record, "kind", path), `${path}.kind`, [
    "fixedQuestion",
    "questionPool",
  ]);
  const sharedFields = ["kind", "scoringRule", "questionAttemptLimit", "questionAttemptTimeLimit"];
  requireOnlyFields(
    record,
    path,
    kind === "fixedQuestion"
      ? [...sharedFields, "reference", "pointsPossible"]
      : [
          ...sharedFields,
          "questionPoolRevision",
          "selectionCount",
          "pointsPerItem",
          "selectionRule",
        ],
  );
  const settings = {
    scoringRule: decodeStringEnum(field(record, "scoringRule", path), `${path}.scoringRule`, [
      "normal",
      "fullCredit",
      "extraCredit",
      "excluded",
    ]),
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
  if (kind === "fixedQuestion")
    return {
      kind,
      ...settings,
      reference: decodeQuestionRevisionReference(
        field(record, "reference", path),
        `${path}.reference`,
        true,
      ),
      pointsPossible: pointValue(field(record, "pointsPossible", path), `${path}.pointsPossible`),
    };
  const selectionCount = decodePositiveInteger(
    field(record, "selectionCount", path),
    `${path}.selectionCount`,
  );
  if (selectionCount > 4_294_967_295)
    throw new DecodeError(`${path}.selectionCount`, "a positive u32");
  return {
    kind,
    ...settings,
    selectionCount,
    questionPoolRevision: questionPoolRevisionReference(
      field(record, "questionPoolRevision", path),
      `${path}.questionPoolRevision`,
    ),
    pointsPerItem: pointValue(field(record, "pointsPerItem", path), `${path}.pointsPerItem`),
    selectionRule: poolSelectionRule(field(record, "selectionRule", path), `${path}.selectionRule`),
  };
}

function blueprintUpdateContent(value: unknown, path: string): AssessmentBlueprintUpdateContent {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "assessmentType",
    "title",
    "instructions",
    "defaults",
    "entries",
  ]);
  const defaultsPath = `${path}.defaults`;
  const defaults = decodeRecord(field(record, "defaults", path), defaultsPath);
  requireOnlyFields(defaults, defaultsPath, [
    "assessment_attempt_time_limit_seconds",
    "assessment_attempt_limit",
    "late_work_rule",
    "activity_rules",
    "student_feedback_release_rule",
  ]);
  return {
    assessmentType: assessmentType(field(record, "assessmentType", path), `${path}.assessmentType`),
    title: decodeAssessmentTitle(field(record, "title", path), `${path}.title`),
    instructions: decodeAssessmentInstructions(
      field(record, "instructions", path),
      `${path}.instructions`,
    ),
    entries: decodeBoundedArray(
      field(record, "entries", path),
      `${path}.entries`,
      MAX_ASSESSMENT_ORDERED_ENTRIES,
      blueprintUpdateEntry,
    ),
    defaults: {
      assessment_attempt_time_limit_seconds: optionalPositiveInteger(
        field(defaults, "assessment_attempt_time_limit_seconds", defaultsPath),
        `${defaultsPath}.assessment_attempt_time_limit_seconds`,
      ),
      assessment_attempt_limit: optionalPositiveInteger(
        field(defaults, "assessment_attempt_limit", defaultsPath),
        `${defaultsPath}.assessment_attempt_limit`,
      ),
      late_work_rule: lateWorkRule(
        field(defaults, "late_work_rule", defaultsPath),
        `${defaultsPath}.late_work_rule`,
      ),
      activity_rules: decodeAssessmentActivityRules(
        field(defaults, "activity_rules", defaultsPath),
        `${defaultsPath}.activity_rules`,
      ),
      student_feedback_release_rule: decodeStudentFeedbackReleaseRule(
        field(defaults, "student_feedback_release_rule", defaultsPath),
        `${defaultsPath}.student_feedback_release_rule`,
      ),
    },
  };
}

/** ASVS 1.5.2, 2.2.1: accept only the closed, internally consistent review. */
export function decodeAssessmentBlueprintUpdateReview(
  value: unknown,
  path = "response",
): AssessmentBlueprintUpdateReview {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "assessment",
    "sourceRevision",
    "proposed",
    "cannotApplyReason",
  ]);
  const rawProposed = field(record, "proposed", path);
  const rawReason = field(record, "cannotApplyReason", path);
  const assessment = decodeLiveAssessmentWorkspace(
    field(record, "assessment", path),
    `${path}.assessment`,
  );
  const proposed =
    rawProposed === null ? null : blueprintUpdateContent(rawProposed, `${path}.proposed`);
  const cannotApplyReason =
    rawReason === null
      ? null
      : decodeStringEnum(rawReason, `${path}.cannotApplyReason`, [
          "retainedSourceMissing",
          "assessmentTypeMismatch",
        ]);
  if (
    (proposed === null) !== (cannotApplyReason === "retainedSourceMissing") ||
    (proposed !== null &&
      (proposed.assessmentType !== assessment.assessmentType) !==
        (cannotApplyReason === "assessmentTypeMismatch")) ||
    assessment.origin.kind !== "adopted"
  ) {
    throw new DecodeError(path, "a consistent retained Assessment Blueprint update review");
  }
  return {
    assessment,
    proposed,
    cannotApplyReason,
    sourceRevision: blueprintRevision(
      field(record, "sourceRevision", path),
      `${path}.sourceRevision`,
    ),
  };
}

function editNumber(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  if (!/^[1-9][0-9]*$/u.test(decoded) || BigInt(decoded) > 9_223_372_036_854_775_807n) {
    throw new DecodeError(path, "a positive Assessment Edit Number");
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

/** Decode the persisted Assessment Activity Rules shared by current work and Attempt evidence. */
export function decodeAssessmentActivityRules(
  value: unknown,
  path: string,
): AssessmentActivityRules {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "questionVariationRule",
    "assessmentQuestionOrderRule",
  ]);
  return {
    questionVariationRule: decodeStringEnum(
      field(record, "questionVariationRule", path),
      `${path}.questionVariationRule`,
      ["reuseVariation", "newVariation"],
    ),
    assessmentQuestionOrderRule: decodeStringEnum(
      field(record, "assessmentQuestionOrderRule", path),
      `${path}.assessmentQuestionOrderRule`,
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

export function blueprintCourseReference(value: unknown, path: string): BlueprintCourseReference {
  const decoded = decodeString(value, path);
  if (!/^BP[0-9ABCDEFGHJKMNPQRSTVWXYZ]{6}$/u.test(decoded)) {
    throw new DecodeError(path, "a canonical opaque Blueprint Course reference");
  }
  return decoded;
}

export function blueprintRevision(value: unknown, path: string): BlueprintRevision {
  const decoded = decodeString(value, path);
  if (!/^[1-9][0-9]*$/u.test(decoded) || BigInt(decoded) > 9_223_372_036_854_775_807n) {
    throw new DecodeError(path, "a positive Blueprint Revision");
  }
  return decoded;
}

function blueprintAssessmentSource(value: unknown, path: string): BlueprintAssessmentSource {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["blueprint_revision", "blueprint_assessment_reference"]);
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
    blueprint_assessment_reference: decodeIdentifier(
      field(record, "blueprint_assessment_reference", path),
      `${path}.blueprint_assessment_reference`,
    ),
  };
}

function pointValue(value: unknown, path: string): AssessmentPointValue {
  const decoded = decodeString(value, path);
  if (
    !/^(?:0|[1-9][0-9]{0,9})(?:\.[0-9]{1,4})?$/u.test(decoded) ||
    BigInt(decoded.split(".")[0] ?? "0") > 1_000_000_000n
  ) {
    throw new DecodeError(path, "a supported nonnegative point decimal with at most four places");
  }
  return decoded;
}

function questionPoolRevisionReference(
  value: unknown,
  path: string,
): QuestionPoolRevisionReference {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["questionPoolId", "revisionNumber"]);
  return {
    questionPoolId: decodeQuestionId(
      field(record, "questionPoolId", path),
      `${path}.questionPoolId`,
    ),
    revisionNumber: decodePositiveQuestionRevisionNumber(
      field(record, "revisionNumber", path),
      `${path}.revisionNumber`,
    ),
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

function assessmentEntry(value: unknown, path: string): AssessmentEntry {
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
      ] as const satisfies ReadonlyArray<AssessmentEntryAvailability>),
      scoringRule: decodeStringEnum(field(record, "scoringRule", path), `${path}.scoringRule`, [
        "normal",
        "fullCredit",
        "extraCredit",
        "excluded",
      ] as const satisfies ReadonlyArray<AssessmentEntryScoringRule>),
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
    throw new DecodeError(`${path}.kind`, "a known Assessment Entry kind");
  requireOnlyFields(record, path, [
    "kind",
    "id",
    "questionPoolRevision",
    "availability",
    "scoringRule",
    "selectionCount",
    "pointsPerItem",
    "selectionRule",
    "questionAttemptLimit",
    "questionAttemptTimeLimit",
  ]);
  const selectionCount = decodePositiveInteger(
    field(record, "selectionCount", path),
    `${path}.selectionCount`,
  );
  return {
    kind,
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    questionPoolRevision: questionPoolRevisionReference(
      field(record, "questionPoolRevision", path),
      `${path}.questionPoolRevision`,
    ),
    availability: decodeStringEnum(field(record, "availability", path), `${path}.availability`, [
      "available",
      "retired",
    ] as const satisfies ReadonlyArray<AssessmentEntryAvailability>),
    scoringRule: decodeStringEnum(field(record, "scoringRule", path), `${path}.scoringRule`, [
      "normal",
      "fullCredit",
      "extraCredit",
      "excluded",
    ] as const satisfies ReadonlyArray<AssessmentEntryScoringRule>),
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
  };
}

function entries(value: unknown, path: string): ReadonlyArray<AssessmentEntry> {
  return decodeBoundedArray(value, path, MAX_ASSESSMENT_ORDERED_ENTRIES, assessmentEntry);
}

function pickerEntry(value: unknown, path: string): AssessmentQuestionPickerEntry {
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

function authoredQuestion(value: unknown, path: string): AuthoredAssessmentQuestion {
  return pickerEntry(value, path);
}

function status(value: unknown, path: string): LiveAssessmentStatus {
  return decodeStringEnum(value, path, ["unreleased", "released", "closed", "archived"]);
}

function assessmentType(value: unknown, path: string): AssessmentType {
  return decodeStringEnum(value, path, ASSESSMENT_TYPE_VALUES);
}

function courseAssessmentSummary(value: unknown, path: string): CourseAssessmentSummary {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "reference",
    "assessmentType",
    "title",
    "dueAt",
    "displayTimeZone",
    "status",
    "editNumber",
  ]);
  return {
    reference: decodeAssessmentReference(field(record, "reference", path), `${path}.reference`),
    assessmentType: assessmentType(field(record, "assessmentType", path), `${path}.assessmentType`),
    title: decodeAssessmentTitle(field(record, "title", path), `${path}.title`),
    dueAt: localDateAndTime(field(record, "dueAt", path), `${path}.dueAt`),
    displayTimeZone: displayTimeZone(
      field(record, "displayTimeZone", path),
      `${path}.displayTimeZone`,
    ),
    status: status(field(record, "status", path), `${path}.status`),
    editNumber: editNumber(field(record, "editNumber", path), `${path}.editNumber`),
  };
}

function dueSoonAssessmentSummary(value: unknown, path: string): DueSoonAssessmentSummary {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "courseReference",
    "courseLongName",
    "assessmentReference",
    "assessmentType",
    "assessmentTitle",
    "assessmentStatus",
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
    assessmentReference: decodeAssessmentReference(
      field(record, "assessmentReference", path),
      `${path}.assessmentReference`,
    ),
    assessmentType: assessmentType(field(record, "assessmentType", path), `${path}.assessmentType`),
    assessmentTitle: decodeAssessmentTitle(
      field(record, "assessmentTitle", path),
      `${path}.assessmentTitle`,
    ),
    assessmentStatus: status(field(record, "assessmentStatus", path), `${path}.assessmentStatus`),
    dueAtMillis,
  };
}

export function decodeDueSoonAssessments(value: unknown, path = "response"): DueSoonAssessments {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["items", "nextCursor", "displayTimeZone"]);
  if (field(record, "nextCursor", path) !== null)
    throw new DecodeError(`${path}.nextCursor`, "null for the bounded Due Soon list");
  return {
    items: decodeArray(field(record, "items", path), `${path}.items`, dueSoonAssessmentSummary),
    nextCursor: null,
    displayTimeZone: displayTimeZone(
      field(record, "displayTimeZone", path),
      `${path}.displayTimeZone`,
    ),
  };
}

export function decodeCreateLiveAssessmentInput(
  value: unknown,
  path = "request",
): CreateLiveAssessmentInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["assessmentType", "title", "instructions"]);
  return {
    assessmentType: assessmentType(field(record, "assessmentType", path), `${path}.assessmentType`),
    title: decodeAssessmentTitle(field(record, "title", path), `${path}.title`),
    instructions: decodeAssessmentInstructions(
      field(record, "instructions", path),
      `${path}.instructions`,
    ),
  };
}

function assessmentOrigin(value: unknown, path: string): AssessmentOrigin {
  const record = decodeRecord(value, path);
  const kind = decodeString(field(record, "kind", path), `${path}.kind`);
  if (kind === "direct") {
    requireOnlyFields(record, path, ["kind"]);
    return { kind };
  }
  if (kind === "adopted") {
    requireOnlyFields(record, path, ["kind", "source"]);
    return {
      kind,
      source: blueprintAssessmentSource(field(record, "source", path), `${path}.source`),
    };
  }
  throw new DecodeError(`${path}.kind`, "a known Assessment origin");
}

export function decodeSaveLiveAssessmentInlineInput(
  value: unknown,
  path = "request",
): SaveLiveAssessmentInlineInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["title", "dueAt"]);
  return {
    title: decodeAssessmentTitle(field(record, "title", path), `${path}.title`),
    dueAt: localDateAndTime(field(record, "dueAt", path), `${path}.dueAt`),
  };
}

export function decodeSaveLiveAssessmentInput(
  value: unknown,
  path = "request",
): SaveLiveAssessmentInput {
  // ASVS 1.5.2 and 2.2.1: browser requests use one closed current aggregate.
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "title",
    "instructions",
    "dueAt",
    "availableAt",
    "closesAt",
    "lateWorkRule",
    "assessmentAttemptTimeLimitSeconds",
    "attemptLimit",
    "activityRules",
    "studentFeedbackReleaseRule",
    "entries",
  ]);
  return {
    title: decodeAssessmentTitle(field(record, "title", path), `${path}.title`),
    instructions: decodeAssessmentInstructions(
      field(record, "instructions", path),
      `${path}.instructions`,
    ),
    dueAt: localDateAndTime(field(record, "dueAt", path), `${path}.dueAt`),
    availableAt: localDateAndTime(field(record, "availableAt", path), `${path}.availableAt`),
    closesAt: localDateAndTime(field(record, "closesAt", path), `${path}.closesAt`),
    lateWorkRule: lateWorkRule(field(record, "lateWorkRule", path), `${path}.lateWorkRule`),
    assessmentAttemptTimeLimitSeconds: optionalPositiveInteger(
      field(record, "assessmentAttemptTimeLimitSeconds", path),
      `${path}.assessmentAttemptTimeLimitSeconds`,
    ),
    attemptLimit: optionalPositiveInteger(
      field(record, "attemptLimit", path),
      `${path}.attemptLimit`,
    ),
    activityRules: decodeAssessmentActivityRules(
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

export function decodeSaveBaseAssessmentPolicyInput(
  value: unknown,
  path = "request",
): SaveBaseAssessmentPolicyInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "instructions",
    "dueAt",
    "availableAt",
    "closesAt",
    "lateWorkRule",
    "assessmentAttemptTimeLimitSeconds",
    "attemptLimit",
    "activityRules",
    "studentFeedbackReleaseRule",
  ]);
  return {
    instructions: decodeAssessmentInstructions(
      field(record, "instructions", path),
      `${path}.instructions`,
    ),
    dueAt: localDateAndTime(field(record, "dueAt", path), `${path}.dueAt`),
    availableAt: localDateAndTime(field(record, "availableAt", path), `${path}.availableAt`),
    closesAt: localDateAndTime(field(record, "closesAt", path), `${path}.closesAt`),
    lateWorkRule: lateWorkRule(field(record, "lateWorkRule", path), `${path}.lateWorkRule`),
    assessmentAttemptTimeLimitSeconds: optionalPositiveInteger(
      field(record, "assessmentAttemptTimeLimitSeconds", path),
      `${path}.assessmentAttemptTimeLimitSeconds`,
    ),
    attemptLimit: optionalPositiveInteger(
      field(record, "attemptLimit", path),
      `${path}.attemptLimit`,
    ),
    activityRules: decodeAssessmentActivityRules(
      field(record, "activityRules", path),
      `${path}.activityRules`,
    ),
    studentFeedbackReleaseRule: decodeStudentFeedbackReleaseRule(
      field(record, "studentFeedbackReleaseRule", path),
      `${path}.studentFeedbackReleaseRule`,
    ),
  };
}

export function decodeLiveAssessmentWorkspace(
  value: unknown,
  path = "response",
): LiveAssessmentWorkspace {
  // ASVS 1.5.2: untrusted transport JSON is allowlisted before UI use.
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "reference",
    "editNumber",
    "status",
    "origin",
    "assessmentType",
    "title",
    "instructions",
    "dueAt",
    "availableAt",
    "closesAt",
    "lateWorkRule",
    "assessmentAttemptTimeLimitSeconds",
    "attemptLimit",
    "activityRules",
    "studentFeedbackReleaseRule",
    "displayTimeZone",
    "entries",
    "questions",
  ]);
  return {
    reference: decodeAssessmentReference(field(record, "reference", path), `${path}.reference`),
    editNumber: editNumber(field(record, "editNumber", path), `${path}.editNumber`),
    status: status(field(record, "status", path), `${path}.status`),
    origin: assessmentOrigin(field(record, "origin", path), `${path}.origin`),
    assessmentType: assessmentType(field(record, "assessmentType", path), `${path}.assessmentType`),
    title: decodeAssessmentTitle(field(record, "title", path), `${path}.title`),
    instructions: decodeAssessmentInstructions(
      field(record, "instructions", path),
      `${path}.instructions`,
    ),
    dueAt: localDateAndTime(field(record, "dueAt", path), `${path}.dueAt`),
    availableAt: localDateAndTime(field(record, "availableAt", path), `${path}.availableAt`),
    closesAt: localDateAndTime(field(record, "closesAt", path), `${path}.closesAt`),
    lateWorkRule: lateWorkRule(field(record, "lateWorkRule", path), `${path}.lateWorkRule`),
    assessmentAttemptTimeLimitSeconds: optionalPositiveInteger(
      field(record, "assessmentAttemptTimeLimitSeconds", path),
      `${path}.assessmentAttemptTimeLimitSeconds`,
    ),
    attemptLimit: optionalPositiveInteger(
      field(record, "attemptLimit", path),
      `${path}.attemptLimit`,
    ),
    activityRules: decodeAssessmentActivityRules(
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
      MAX_ASSESSMENT_ORDERED_ENTRIES,
      authoredQuestion,
    ),
  };
}

export function decodeAssessmentQuestionPicker(
  value: unknown,
  path = "response",
): ReadonlyArray<AssessmentQuestionPickerEntry> {
  return decodeArray(value, path, pickerEntry);
}

export function decodeCourseAssessments(
  value: unknown,
  path = "response",
): ReadonlyArray<CourseAssessmentSummary> {
  return decodeArray(value, path, courseAssessmentSummary);
}
export function decodeCourseAssessmentSummary(
  value: unknown,
  path = "response",
): CourseAssessmentSummary {
  return courseAssessmentSummary(value, path);
}

export function decodeAssessmentReleaseValidation(
  value: unknown,
  path = "response",
): AssessmentReleaseValidation {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["canRelease", "issues"]);
  return {
    canRelease: decodeBoolean(field(record, "canRelease", path), `${path}.canRelease`),
    issues: decodeArray(field(record, "issues", path), `${path}.issues`, (item, itemPath) =>
      decodeStringEnum(item, itemPath, [
        "noPublishedQuestions",
        "questionUnavailable",
        "dueDateRequired",
        "dueDateLessThan24HoursAhead",
        "dueDateAfterCourseActiveUntil",
        "availabilityAfterDueDate",
        "dueDateAfterClose",
        "timeLimitRequired",
      ]),
    ),
  };
}

/** Decode the released-only aggregate confirmation projection. */
export function decodeAssessmentUnreleaseImpact(
  value: unknown,
  path = "response",
): AssessmentUnreleaseImpact {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "confirmationTitle",
    "editNumber",
    "attemptCount",
    "submissionCount",
    "gradeCount",
  ]);
  return {
    confirmationTitle: decodeAssessmentTitle(
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
export function decodeUnreleasedLiveAssessment(
  value: unknown,
  path = "response",
): UnreleasedLiveAssessment {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["assessment", "deleted"]);
  return {
    assessment: decodeLiveAssessmentWorkspace(
      field(record, "assessment", path),
      `${path}.assessment`,
    ),
    deleted: decodeAssessmentUnreleaseImpact(field(record, "deleted", path), `${path}.deleted`),
  };
}
