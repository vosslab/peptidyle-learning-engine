// Strict browser decoding and local command validation for reusable Blueprint Courses.

import { MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY } from "../../../generated/api/MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY";
import { MAX_ASSESSMENT_INSTRUCTIONS_UNICODE_SCALARS } from "../../../generated/api/MAX_ASSESSMENT_INSTRUCTIONS_UNICODE_SCALARS";
import { MAX_ASSESSMENT_ORDERED_ENTRIES } from "../../../generated/api/MAX_ASSESSMENT_ORDERED_ENTRIES";
import { MAX_ASSESSMENT_QUESTION_POOL_ITEMS } from "../../../generated/api/MAX_ASSESSMENT_QUESTION_POOL_ITEMS";
import { MAX_BLUEPRINT_COURSE_TITLE_UNICODE_SCALARS } from "../../../generated/api/MAX_BLUEPRINT_COURSE_TITLE_UNICODE_SCALARS";
import type { BlueprintCourseSummaryView } from "../../../generated/api/BlueprintCourseSummaryView";
import type { BlueprintCourseView } from "../../../generated/api/BlueprintCourseView";
import type { BlueprintCourseReference } from "../../../generated/api/BlueprintCourseReference";
import type { BlueprintAvailability } from "../../../generated/api/BlueprintAvailability";
import type { BlueprintRevisionReference } from "../../../generated/api/BlueprintRevisionReference";
import type { BlueprintRevisionView } from "../../../generated/api/BlueprintRevisionView";
import type { BlueprintModuleView } from "../../../generated/api/BlueprintModuleView";
import type { BlueprintCourseSaveResponse } from "../../../generated/api/BlueprintCourseSaveResponse";
import type { BlueprintMetadataState } from "../../../generated/api/BlueprintMetadataState";
import type { CreateBlueprintCourseInput } from "../../../generated/api/CreateBlueprintCourseInput";
import type { RenameBlueprintCourseInput } from "../../../generated/api/RenameBlueprintCourseInput";
import type { ReplaceBlueprintCourseContentInput } from "../../../generated/api/ReplaceBlueprintCourseContentInput";
import type { CursorPage } from "../contracts";
import {
  DecodeError,
  decodeFiniteNumber,
  decodeNonemptyString,
  decodeNullable,
  decodePositiveInteger,
  decodeRecord,
  decodeSafeInteger,
  decodeString,
  decodeStringEnum,
} from "../decoder";
import { decodeStudentFeedbackReleaseRule } from "./assessment_policy";
import { decodeQuestionSearchResult } from "./question_library";
import { decodeQuestionAttemptLimit, decodeQuestionAttemptTimeLimit } from "./question_model";
import { decodeBoundedArray, decodeCursor, field, requireOnlyFields } from "./shared";
import { normalizeQuestionIdSyntax } from "../../question_id";

const MAX_PAGE_SIZE = 100;
const POSITIVE_REVISION = /^[1-9][0-9]*$/u;
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/u;

function text(value: unknown, path: string): string {
  const decoded = decodeNonemptyString(value, path);
  if (
    decoded !== decoded.trim() ||
    Array.from(decoded).length > MAX_BLUEPRINT_COURSE_TITLE_UNICODE_SCALARS
  ) {
    throw new DecodeError(path, "trimmed Blueprint Course text within its bound");
  }
  return decoded;
}

function blueprintReference(value: unknown, path: string): BlueprintCourseReference {
  const decoded = decodeString(value, path);
  if (!/^BP[0-9ABCDEFGHJKMNPQRSTVWXYZ]{6}$/u.test(decoded)) {
    throw new DecodeError(path, "a canonical opaque Blueprint Course reference");
  }
  return decoded;
}

function revision(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  if (!POSITIVE_REVISION.test(decoded) || BigInt(decoded) > 9_223_372_036_854_775_807n) {
    throw new DecodeError(path, "a canonical positive PostgreSQL bigint revision");
  }
  return decoded;
}

function questionId(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  const canonicalQuestionId = normalizeQuestionIdSyntax(decoded);
  if (canonicalQuestionId === null || canonicalQuestionId !== decoded)
    throw new DecodeError(path, "a canonical public Question ID");
  return canonicalQuestionId;
}

function pointValue(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  if (!/^(?:0|[1-9][0-9]*)(?:\.[0-9]{1,4})?$/u.test(decoded)) {
    throw new DecodeError(path, "a canonical nonnegative point decimal with at most four places");
  }
  return decoded;
}

function assessmentCompletionRule(value: unknown, path: string): unknown {
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
  throw new DecodeError(`${path}.kind`, "a known Assessment Completion Rule");
}

function assessmentAttemptContinuationRule(value: unknown, path: string): unknown {
  const record = decodeRecord(value, path);
  const kind = decodeString(field(record, "kind", path), `${path}.kind`);
  if (kind === "unlimited" || kind === "closed") {
    requireOnlyFields(record, path, ["kind"]);
    return { kind };
  }
  if (kind === "capped") {
    requireOnlyFields(record, path, ["kind", "maxAdditionalAssessmentAttempts"]);
    const maxAdditionalAssessmentAttempts = decodeSafeInteger(
      field(record, "maxAdditionalAssessmentAttempts", path),
      `${path}.maxAdditionalAssessmentAttempts`,
    );
    if (maxAdditionalAssessmentAttempts < 0)
      throw new DecodeError(
        `${path}.maxAdditionalAssessmentAttempts`,
        "a nonnegative safe integer",
      );
    return { kind, maxAdditionalAssessmentAttempts };
  }
  throw new DecodeError(`${path}.kind`, "a known Assessment Attempt Continuation Rule");
}

function defaults(value: unknown, path: string): unknown {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "assessment_attempt_time_limit_seconds",
    "attempt_limit",
    "late_work_rule",
    "activity_rules",
    "student_feedback_release_rule",
  ]);
  const policies = decodeRecord(field(record, "activity_rules", path), `${path}.activity_rules`);
  requireOnlyFields(policies, `${path}.activity_rules`, [
    "assessmentCompletionRule",
    "assessmentAttemptGradeRule",
    "assessmentAttemptContinuationRule",
    "questionPoolReuseRule",
    "questionVariationRule",
    "assessmentAttemptResumeRule",
    "assessmentQuestionDisplayRule",
    "assessmentNavigationRule",
    "assessmentQuestionOrderRule",
  ]);
  return {
    assessment_attempt_time_limit_seconds: decodeNullable(
      field(record, "assessment_attempt_time_limit_seconds", path),
      `${path}.assessment_attempt_time_limit_seconds`,
      decodePositiveInteger,
    ),
    attempt_limit: decodeNullable(
      field(record, "attempt_limit", path),
      `${path}.attempt_limit`,
      decodePositiveInteger,
    ),
    late_work_rule: decodeStringEnum(
      field(record, "late_work_rule", path),
      `${path}.late_work_rule`,
      ["accept", "mark_late", "reject"],
    ),
    activity_rules: {
      assessmentCompletionRule: assessmentCompletionRule(
        field(policies, "assessmentCompletionRule", `${path}.activity_rules`),
        `${path}.activity_rules.assessmentCompletionRule`,
      ),
      assessmentAttemptGradeRule: decodeStringEnum(
        field(policies, "assessmentAttemptGradeRule", `${path}.activity_rules`),
        `${path}.activity_rules.assessmentAttemptGradeRule`,
        ["first", "latest", "highest", "instructorSelected"],
      ),
      assessmentAttemptContinuationRule: assessmentAttemptContinuationRule(
        field(policies, "assessmentAttemptContinuationRule", `${path}.activity_rules`),
        `${path}.activity_rules.assessmentAttemptContinuationRule`,
      ),
      questionPoolReuseRule: decodeStringEnum(
        field(policies, "questionPoolReuseRule", `${path}.activity_rules`),
        `${path}.activity_rules.questionPoolReuseRule`,
        ["reuseSelection", "selectAgain"],
      ),
      questionVariationRule: decodeStringEnum(
        field(policies, "questionVariationRule", `${path}.activity_rules`),
        `${path}.activity_rules.questionVariationRule`,
        ["reuseVariation", "newVariation"],
      ),
      assessmentAttemptResumeRule: decodeStringEnum(
        field(policies, "assessmentAttemptResumeRule", `${path}.activity_rules`),
        `${path}.activity_rules.assessmentAttemptResumeRule`,
        ["resumable", "singleSession"],
      ),
      assessmentQuestionDisplayRule: decodeStringEnum(
        field(policies, "assessmentQuestionDisplayRule", `${path}.activity_rules`),
        `${path}.activity_rules.assessmentQuestionDisplayRule`,
        ["allQuestions", "oneQuestionAtATime"],
      ),
      assessmentNavigationRule: decodeStringEnum(
        field(policies, "assessmentNavigationRule", `${path}.activity_rules`),
        `${path}.activity_rules.assessmentNavigationRule`,
        ["freeNavigation", "forwardOnly"],
      ),
      assessmentQuestionOrderRule: decodeStringEnum(
        field(policies, "assessmentQuestionOrderRule", `${path}.activity_rules`),
        `${path}.activity_rules.assessmentQuestionOrderRule`,
        ["authoredOrder", "shuffled"],
      ),
    },
    student_feedback_release_rule: decodeStudentFeedbackReleaseRule(
      field(record, "student_feedback_release_rule", path),
      `${path}.student_feedback_release_rule`,
    ),
  };
}

function assessmentEntry(
  value: unknown,
  path: string,
): { kind: "fixed" | "pool"; questionPoolItems: string[] } {
  const record = decodeRecord(value, path);
  const kind = decodeStringEnum(field(record, "kind", path), `${path}.kind`, ["fixed", "pool"]);
  if (kind === "fixed") {
    requireOnlyFields(record, path, [
      "kind",
      "question_id",
      "points_possible",
      "scoring_rule",
      "question_attempt_limit",
      "question_attempt_time_limit",
    ]);
    questionId(field(record, "question_id", path), `${path}.question_id`);
    pointValue(field(record, "points_possible", path), `${path}.points_possible`);
    decodeStringEnum(field(record, "scoring_rule", path), `${path}.scoring_rule`, [
      "normal",
      "fullCredit",
      "extraCredit",
      "excluded",
    ]);
    decodeQuestionAttemptLimit(
      field(record, "question_attempt_limit", path),
      `${path}.question_attempt_limit`,
      true,
    );
    decodeQuestionAttemptTimeLimit(
      field(record, "question_attempt_time_limit", path),
      `${path}.question_attempt_time_limit`,
      true,
    );
    return { kind, questionPoolItems: [] };
  }
  requireOnlyFields(record, path, [
    "kind",
    "items",
    "selection_count",
    "points_per_item",
    "scoring_rule",
    "selection_rule",
    "question_attempt_limit",
    "question_attempt_time_limit",
  ]);
  const questionPoolItems = decodeBoundedArray(
    field(record, "items", path),
    `${path}.items`,
    MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY,
    questionId,
  );
  const selectionCount = decodePositiveInteger(
    field(record, "selection_count", path),
    `${path}.selection_count`,
  );
  if (
    questionPoolItems.length === 0 ||
    selectionCount > questionPoolItems.length ||
    new Set(questionPoolItems).size !== questionPoolItems.length
  ) {
    throw new DecodeError(
      path,
      "a nonempty Question Pool with distinct Question Pool Items and a valid selection count",
    );
  }
  pointValue(field(record, "points_per_item", path), `${path}.points_per_item`);
  selectionRule(field(record, "selection_rule", path), `${path}.selection_rule`);
  decodeQuestionAttemptLimit(
    field(record, "question_attempt_limit", path),
    `${path}.question_attempt_limit`,
    true,
  );
  decodeQuestionAttemptTimeLimit(
    field(record, "question_attempt_time_limit", path),
    `${path}.question_attempt_time_limit`,
    true,
  );
  return { kind, questionPoolItems };
}

function selectionRule(value: unknown, path: string): void {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["selected_question_order"]);
  decodeStringEnum(
    field(record, "selected_question_order", path),
    `${path}.selected_question_order`,
    ["questionPoolOrder", "randomOrder"],
  );
}

function assessmentContent(value: unknown, path: string): unknown {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["title", "instructions", "entries", "defaults"]);
  const instructions = decodeString(field(record, "instructions", path), `${path}.instructions`);
  if (Array.from(instructions).length > MAX_ASSESSMENT_INSTRUCTIONS_UNICODE_SCALARS)
    throw new DecodeError(`${path}.instructions`, "instructions within the shared bound");
  const entries = decodeBoundedArray(
    field(record, "entries", path),
    `${path}.entries`,
    MAX_ASSESSMENT_ORDERED_ENTRIES,
    assessmentEntry,
  );
  if (entries.length === 0) throw new DecodeError(`${path}.entries`, "at least one ordered entry");
  const questionPoolItemCount = entries.reduce(
    (total, entry) => total + entry.questionPoolItems.length,
    0,
  );
  if (questionPoolItemCount > MAX_ASSESSMENT_QUESTION_POOL_ITEMS)
    throw new DecodeError(
      `${path}.entries`,
      "Question Pool Items within the Assessment total bound",
    );
  defaults(field(record, "defaults", path), `${path}.defaults`);
  return value;
}

function createModule(value: unknown, path: string): unknown {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["label", "assessments"]);
  const assessments = decodeBoundedArray(
    field(record, "assessments", path),
    `${path}.assessments`,
    MAX_ASSESSMENT_ORDERED_ENTRIES,
    assessmentContent,
  );
  if (assessments.length === 0)
    throw new DecodeError(`${path}.assessments`, "at least one content");
  text(field(record, "label", path), `${path}.label`);
  return value;
}

export function decodeCreateBlueprintCourseInput(
  value: unknown,
  path = "request",
): CreateBlueprintCourseInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["short_name", "long_name", "modules"]);
  const modules = decodeBoundedArray(
    field(record, "modules", path),
    `${path}.modules`,
    MAX_ASSESSMENT_ORDERED_ENTRIES,
    createModule,
  );
  if (modules.length === 0) throw new DecodeError(`${path}.modules`, "at least one ordered module");
  text(field(record, "short_name", path), `${path}.short_name`);
  text(field(record, "long_name", path), `${path}.long_name`);
  return value as CreateBlueprintCourseInput;
}

// ASVS 1.5.2, 2.2.1, and 2.2.2: accept only the exact retained-reference-or-New shape.
function replacementChoice(value: unknown, path: string, referenceField: string): void {
  const record = decodeRecord(value, path);
  const kind = decodeStringEnum(field(record, "kind", path), `${path}.kind`, ["retained", "new"]);
  requireOnlyFields(record, path, kind === "new" ? ["kind"] : ["kind", referenceField]);
  if (kind === "retained")
    decodeNonemptyString(field(record, referenceField, path), `${path}.${referenceField}`);
}

function replacementModule(value: unknown, path: string): unknown {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["choice", "label", "assessments"]);
  replacementChoice(field(record, "choice", path), `${path}.choice`, "blueprint_module_reference");
  text(field(record, "label", path), `${path}.label`);
  const assessments = decodeBoundedArray(
    field(record, "assessments", path),
    `${path}.assessments`,
    MAX_ASSESSMENT_ORDERED_ENTRIES,
    (contentValue, contentPath) => {
      const content = decodeRecord(contentValue, contentPath);
      requireOnlyFields(content, contentPath, ["choice", "content"]);
      replacementChoice(
        field(content, "choice", contentPath),
        `${contentPath}.choice`,
        "blueprint_assessment_reference",
      );
      assessmentContent(field(content, "content", contentPath), `${contentPath}.content`);
      return contentValue;
    },
  );
  if (assessments.length === 0)
    throw new DecodeError(`${path}.assessments`, "at least one content");
  return value;
}

export function decodeReplaceBlueprintCourseContentInput(
  value: unknown,
  path = "request",
): ReplaceBlueprintCourseContentInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["modules"]);
  const modules = decodeBoundedArray(
    field(record, "modules", path),
    `${path}.modules`,
    MAX_ASSESSMENT_ORDERED_ENTRIES,
    replacementModule,
  );
  if (modules.length === 0) throw new DecodeError(`${path}.modules`, "at least one ordered module");
  return value as ReplaceBlueprintCourseContentInput;
}

function questionView(value: unknown, path: string): void {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["question_library", "selection_availability"]);
  decodeQuestionSearchResult(field(record, "question_library", path), `${path}.question_library`);
  decodeStringEnum(
    field(record, "selection_availability", path),
    `${path}.selection_availability`,
    ["available", "retained"],
  );
}

function contentView(value: unknown, path: string): void {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["title", "instructions", "entries", "defaults"]);
  text(field(record, "title", path), `${path}.title`);
  decodeString(field(record, "instructions", path), `${path}.instructions`);
  defaults(field(record, "defaults", path), `${path}.defaults`);
  decodeBoundedArray(
    field(record, "entries", path),
    `${path}.entries`,
    MAX_ASSESSMENT_ORDERED_ENTRIES,
    (entryValue, entryPath) => {
      const entry = decodeRecord(entryValue, entryPath);
      const kind = decodeStringEnum(field(entry, "kind", entryPath), `${entryPath}.kind`, [
        "fixed",
        "pool",
      ]);
      if (kind === "fixed") {
        requireOnlyFields(entry, entryPath, [
          "kind",
          "question",
          "points_possible",
          "scoring_rule",
          "question_attempt_limit",
          "question_attempt_time_limit",
        ]);
        questionView(field(entry, "question", entryPath), `${entryPath}.question`);
        decodeQuestionAttemptLimit(
          field(entry, "question_attempt_limit", entryPath),
          `${entryPath}.question_attempt_limit`,
          true,
        );
        decodeQuestionAttemptTimeLimit(
          field(entry, "question_attempt_time_limit", entryPath),
          `${entryPath}.question_attempt_time_limit`,
          true,
        );
      } else {
        requireOnlyFields(entry, entryPath, [
          "kind",
          "items",
          "selection_count",
          "points_per_item",
          "scoring_rule",
          "selection_rule",
          "question_attempt_limit",
          "question_attempt_time_limit",
        ]);
        decodeBoundedArray(
          field(entry, "items", entryPath),
          `${entryPath}.items`,
          MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY,
          questionView,
        );
        decodeQuestionAttemptLimit(
          field(entry, "question_attempt_limit", entryPath),
          `${entryPath}.question_attempt_limit`,
          true,
        );
        decodeQuestionAttemptTimeLimit(
          field(entry, "question_attempt_time_limit", entryPath),
          `${entryPath}.question_attempt_time_limit`,
          true,
        );
      }
      return entryValue;
    },
  );
}

function availability(value: unknown, path: string): BlueprintAvailability {
  // Blueprint Availability is a closed, generated browser contract. Do not
  // accept the obsolete `available` spelling: Private and Public have distinct
  // visibility and adoption behavior.
  return decodeStringEnum(value, path, ["private", "public", "archived"]);
}

function metadataEtag(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  if (!UUID.test(decoded)) throw new DecodeError(path, "a canonical opaque metadata UUID");
  return decoded;
}

function revisionReference(value: unknown, path: string): BlueprintRevisionReference {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["reference", "revision"]);
  return {
    reference: blueprintReference(field(record, "reference", path), `${path}.reference`),
    revision: revision(field(record, "revision", path), `${path}.revision`),
  };
}

function summary(value: unknown, path: string): BlueprintCourseSummaryView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "total_adoptions",
    "total_students_ever_enrolled",
    "reference",
    "short_name",
    "long_name",
    "availability",
    "metadata_etag",
    "current_revision",
    "read_access",
  ]);
  const totalAdoptions = decodeSafeInteger(
    field(record, "total_adoptions", path),
    `${path}.total_adoptions`,
  );
  const totalStudents = decodeSafeInteger(
    field(record, "total_students_ever_enrolled", path),
    `${path}.total_students_ever_enrolled`,
  );
  if (totalAdoptions < 0 || totalStudents < 0)
    throw new DecodeError(path, "nonnegative Blueprint popularity totals");
  return {
    total_adoptions: totalAdoptions,
    total_students_ever_enrolled: totalStudents,
    reference: blueprintReference(field(record, "reference", path), `${path}.reference`),
    short_name: text(field(record, "short_name", path), `${path}.short_name`),
    long_name: text(field(record, "long_name", path), `${path}.long_name`),
    availability: availability(field(record, "availability", path), `${path}.availability`),
    metadata_etag: metadataEtag(field(record, "metadata_etag", path), `${path}.metadata_etag`),
    current_revision: revisionReference(
      field(record, "current_revision", path),
      `${path}.current_revision`,
    ),
    read_access: decodeStringEnum(field(record, "read_access", path), `${path}.read_access`, [
      "blueprint_course_owner",
      "active_instructor",
    ]),
  };
}

function modules(value: unknown, path: string): Array<BlueprintModuleView> {
  const decoded = decodeBoundedArray(
    value,
    path,
    MAX_ASSESSMENT_ORDERED_ENTRIES,
    (moduleValue, modulePath) => {
      const module = decodeRecord(moduleValue, modulePath);
      requireOnlyFields(module, modulePath, ["blueprint_module_reference", "label", "assessments"]);
      decodeNonemptyString(
        field(module, "blueprint_module_reference", modulePath),
        `${modulePath}.blueprint_module_reference`,
      );
      text(field(module, "label", modulePath), `${modulePath}.label`);
      decodeBoundedArray(
        field(module, "assessments", modulePath),
        `${modulePath}.assessments`,
        MAX_ASSESSMENT_ORDERED_ENTRIES,
        (contentValue, contentPath) => {
          const content = decodeRecord(contentValue, contentPath);
          requireOnlyFields(content, contentPath, ["blueprint_assessment_reference", "content"]);
          decodeNonemptyString(
            field(content, "blueprint_assessment_reference", contentPath),
            `${contentPath}.blueprint_assessment_reference`,
          );
          contentView(field(content, "content", contentPath), `${contentPath}.content`);
          return contentValue;
        },
      );
      return moduleValue;
    },
  );
  return decoded as Array<BlueprintModuleView>;
}

export function decodeBlueprintCourseView(value: unknown, path = "response"): BlueprintCourseView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "reference",
    "short_name",
    "long_name",
    "availability",
    "metadata_etag",
    "current_revision",
    "read_access",
    "modules",
  ]);
  return {
    reference: blueprintReference(field(record, "reference", path), `${path}.reference`),
    short_name: text(field(record, "short_name", path), `${path}.short_name`),
    long_name: text(field(record, "long_name", path), `${path}.long_name`),
    availability: availability(field(record, "availability", path), `${path}.availability`),
    metadata_etag: metadataEtag(field(record, "metadata_etag", path), `${path}.metadata_etag`),
    current_revision: revisionReference(
      field(record, "current_revision", path),
      `${path}.current_revision`,
    ),
    read_access: decodeStringEnum(field(record, "read_access", path), `${path}.read_access`, [
      "blueprint_course_owner",
      "active_instructor",
    ]),
    modules: modules(field(record, "modules", path), `${path}.modules`),
  };
}

export function decodeBlueprintRevisionView(
  value: unknown,
  path = "response",
): BlueprintRevisionView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["blueprintRevision", "modules"]);
  return {
    blueprintRevision: revisionReference(
      field(record, "blueprintRevision", path),
      `${path}.blueprintRevision`,
    ),
    modules: modules(field(record, "modules", path), `${path}.modules`),
  };
}

export function decodeBlueprintCourseSaveResponse(
  value: unknown,
  path = "response",
): BlueprintCourseSaveResponse {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["blueprintCourse", "changed"]);
  const changed = field(record, "changed", path);
  if (typeof changed !== "boolean") throw new DecodeError(`${path}.changed`, "a boolean");
  return {
    blueprintCourse: decodeBlueprintCourseView(
      field(record, "blueprintCourse", path),
      `${path}.blueprintCourse`,
    ),
    changed,
  };
}

export function decodeBlueprintMetadataState(
  value: unknown,
  path = "response",
): BlueprintMetadataState {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["short_name", "long_name", "availability", "metadata_etag"]);
  return {
    short_name: text(field(record, "short_name", path), `${path}.short_name`),
    long_name: text(field(record, "long_name", path), `${path}.long_name`),
    availability: availability(field(record, "availability", path), `${path}.availability`),
    metadata_etag: metadataEtag(field(record, "metadata_etag", path), `${path}.metadata_etag`),
  };
}

export function decodeRenameBlueprintCourseInput(
  value: unknown,
  path = "request",
): RenameBlueprintCourseInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["short_name", "long_name"]);
  text(field(record, "short_name", path), `${path}.short_name`);
  text(field(record, "long_name", path), `${path}.long_name`);
  return value as RenameBlueprintCourseInput;
}

export function decodeBlueprintCourseReference(
  value: unknown,
  path = "reference",
): BlueprintCourseReference {
  return blueprintReference(value, path);
}

export function decodeBlueprintRevision(value: unknown, path = "revision"): string {
  return revision(value, path);
}

export function decodeBlueprintCoursePage(
  value: unknown,
  path = "response",
): CursorPage<BlueprintCourseSummaryView> {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["items", "nextCursor"]);
  return {
    items: decodeBoundedArray(
      field(record, "items", path),
      `${path}.items`,
      MAX_PAGE_SIZE,
      summary,
    ),
    nextCursor: decodeNullable(
      field(record, "nextCursor", path),
      `${path}.nextCursor`,
      decodeCursor,
    ),
  };
}
