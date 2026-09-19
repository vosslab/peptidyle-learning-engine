// Strict browser decoding and local command validation for reusable Blueprint Courses.

import { MAX_ASSESSMENT_ORDERED_ENTRIES } from "../../../generated/api/MAX_ASSESSMENT_ORDERED_ENTRIES";
import { MAX_ASSESSMENT_INSTRUCTIONS_UNICODE_SCALARS } from "../../../generated/api/MAX_ASSESSMENT_INSTRUCTIONS_UNICODE_SCALARS";
import { MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY } from "../../../generated/api/MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY";
import { ASSESSMENT_TYPE_VALUES, type AssessmentType } from "../../../generated/api/AssessmentType";
import { MAX_BLUEPRINT_COURSE_TITLE_UNICODE_SCALARS } from "../../../generated/api/MAX_BLUEPRINT_COURSE_TITLE_UNICODE_SCALARS";
import type { BlueprintCourseSummaryView } from "../../../generated/api/BlueprintCourseSummaryView";
import type { BlueprintCourseView } from "../../../generated/api/BlueprintCourseView";
import type { BlueprintCourseId } from "../../../generated/api/BlueprintCourseId";
import type { BlueprintAvailability } from "../../../generated/api/BlueprintAvailability";
import type { BlueprintRevisionTuple } from "../../../generated/api/BlueprintRevisionTuple";
import type { BlueprintRevisionView } from "../../../generated/api/BlueprintRevisionView";
import type { BlueprintKnownForkView } from "../../../generated/api/BlueprintKnownForkView";
import type { BlueprintModuleView } from "../../../generated/api/BlueprintModuleView";
import type { BlueprintCourseSaveResponse } from "../../../generated/api/BlueprintCourseSaveResponse";
import type { BlueprintEditNumber } from "../../../generated/api/BlueprintEditNumber";
import type { BlueprintMetadataState } from "../../../generated/api/BlueprintMetadataState";
import type { CreateBlueprintCourseInput } from "../../../generated/api/CreateBlueprintCourseInput";
import type { RenameBlueprintCourseInput } from "../../../generated/api/RenameBlueprintCourseInput";
import type { ReplaceBlueprintCourseContentInput } from "../../../generated/api/ReplaceBlueprintCourseContentInput";
import type { CursorPage } from "../contracts";
import {
  DecodeError,
  decodeArray,
  decodeBoolean,
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
import {
  decodeBoundedArray,
  decodeCursor,
  decodeQuestionRevisionTuple,
  field,
  requireOnlyFields,
} from "./shared";
import { validateCanonicalPublicId, validateCanonicalQuestionIdSyntax } from "../../question_id";
import { decodeCourseClassification } from "./course_classification";

const MAX_PAGE_SIZE = 100;
const POSITIVE_REVISION = /^[1-9][0-9]*$/u;

export function text(value: unknown, path: string): string {
  const decoded = decodeNonemptyString(value, path);
  if (
    decoded !== decoded.trim() ||
    Array.from(decoded).length > MAX_BLUEPRINT_COURSE_TITLE_UNICODE_SCALARS
  ) {
    throw new DecodeError(path, "trimmed Blueprint Course text within its bound");
  }
  return decoded;
}

function blueprintCourseId(value: unknown, path: string): BlueprintCourseId {
  const decoded = decodeString(value, path);
  if (validateCanonicalPublicId("blueprintCourse", decoded) === null) {
    throw new DecodeError(path, "a canonical Blueprint Course ID");
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

export function questionId(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  const canonicalQuestionId = validateCanonicalQuestionIdSyntax(decoded);
  if (canonicalQuestionId === null || canonicalQuestionId !== decoded)
    throw new DecodeError(path, "a canonical public Question ID");
  return canonicalQuestionId;
}

function questionPoolId(value: unknown, path: string): string {
  const decoded = questionId(value, path);
  return decoded;
}

export function pointValue(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  if (!/^(?:0|[1-9][0-9]*)(?:\.[0-9]{1,4})?$/u.test(decoded)) {
    throw new DecodeError(path, "a canonical nonnegative point decimal with at most four places");
  }
  return decoded;
}

export function defaults(value: unknown, path: string): unknown {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "assessment_attempt_time_limit_seconds",
    "assessment_attempt_limit",
    "late_work_rule",
    "activity_rules",
    "student_feedback_release_rule",
  ]);
  const policies = decodeRecord(field(record, "activity_rules", path), `${path}.activity_rules`);
  requireOnlyFields(policies, `${path}.activity_rules`, [
    "questionVariationRule",
    "assessmentQuestionOrderRule",
  ]);
  return {
    assessment_attempt_time_limit_seconds: decodeNullable(
      field(record, "assessment_attempt_time_limit_seconds", path),
      `${path}.assessment_attempt_time_limit_seconds`,
      decodePositiveInteger,
    ),
    assessment_attempt_limit: decodeNullable(
      field(record, "assessment_attempt_limit", path),
      `${path}.assessment_attempt_limit`,
      decodePositiveInteger,
    ),
    late_work_rule: decodeStringEnum(
      field(record, "late_work_rule", path),
      `${path}.late_work_rule`,
      ["accept", "mark_late", "reject"],
    ),
    activity_rules: {
      questionVariationRule: decodeStringEnum(
        field(policies, "questionVariationRule", `${path}.activity_rules`),
        `${path}.activity_rules.questionVariationRule`,
        ["reuseVariation", "newVariation"],
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

function assessmentEntry(value: unknown, path: string): { kind: "fixed" | "pool" } {
  const record = decodeRecord(value, path);
  const kind = decodeStringEnum(field(record, "kind", path), `${path}.kind`, ["fixed", "pool"]);
  if (kind === "fixed") {
    requireOnlyFields(record, path, [
      "kind",
      "published_question",
      "points_possible",
      "scoring_rule",
      "question_attempt_limit",
      "question_attempt_time_limit",
    ]);
    // ASVS 1.5.2 and 2.2.1: accept the exact Tuple, never an ID-only fallback.
    decodeQuestionRevisionTuple(
      field(record, "published_question", path),
      `${path}.published_question`,
    );
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
    return { kind };
  }
  requireOnlyFields(record, path, [
    "kind",
    "pool",
    "selection_count",
    "points_per_item",
    "scoring_rule",
    "selection_rule",
    "question_attempt_limit",
    "question_attempt_time_limit",
  ]);
  const selectionCount = decodePositiveInteger(
    field(record, "selection_count", path),
    `${path}.selection_count`,
  );
  if (selectionCount > 4_294_967_295)
    throw new DecodeError(`${path}.selection_count`, "a positive u32 selection count");
  authoringPool(field(record, "pool", path), `${path}.pool`, selectionCount);
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
  return { kind };
}

function authoringPool(value: unknown, path: string, selectionCount: number): void {
  // ASVS 1.5.2, 2.2.1: closed input alternatives; no obsolete Pool ID alias.
  const record = decodeRecord(value, path);
  const kind = decodeStringEnum(field(record, "kind", path), `${path}.kind`, [
    "import",
    "retained",
  ]);
  requireOnlyFields(
    record,
    path,
    kind === "import"
      ? ["kind", "question_pool_id", "question_pool_edit_number"]
      : [
          "kind",
          "question_pool_id",
          "question_pool_edit_number",
          "members",
          "interchangeabilityAttested",
        ],
  );
  questionPoolId(field(record, "question_pool_id", path), `${path}.question_pool_id`);
  decodePositiveInteger(
    field(record, "question_pool_edit_number", path),
    `${path}.question_pool_edit_number`,
  );
  if (kind === "import") return;
  const attested = decodeBoolean(
    field(record, "interchangeabilityAttested", path),
    `${path}.interchangeabilityAttested`,
  );
  const members = decodeNullable(
    field(record, "members", path),
    `${path}.members`,
    (memberValue, memberPath) =>
      decodeBoundedArray(
        memberValue,
        memberPath,
        MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY,
        (item, itemPath) => {
          const member = decodeQuestionRevisionTuple(item, itemPath, true);
          if (member.revisionNumber > 4_294_967_295)
            throw new DecodeError(
              `${itemPath}.revisionNumber`,
              "a positive u32 Question Revision Number",
            );
          return member;
        },
      ),
  );
  if (members === null) return;
  if (members.length === 0) throw new DecodeError(`${path}.members`, "at least one Pool member");
  // ASVS 2.2.3: related member count, identity and review must agree.
  if (new Set(members.map((member) => member.questionId)).size !== members.length)
    throw new DecodeError(`${path}.members`, "unique Question IDs");
  if (selectionCount > members.length)
    throw new DecodeError(path, "a selection count within the authored member count");
  if (!attested)
    throw new DecodeError(`${path}.interchangeabilityAttested`, "true for authored members");
}

export function selectionRule(value: unknown, path: string): void {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["selectedQuestionOrder"]);
  decodeStringEnum(field(record, "selectedQuestionOrder", path), `${path}.selectedQuestionOrder`, [
    "questionPoolOrder",
    "randomOrder",
  ]);
}

export function assessmentType(value: unknown, path: string): AssessmentType {
  return decodeStringEnum(value, path, ASSESSMENT_TYPE_VALUES);
}

function assessmentContent(value: unknown, path: string): unknown {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "assessment_type",
    "title",
    "instructions",
    "entries",
    "defaults",
  ]);
  assessmentType(field(record, "assessment_type", path), `${path}.assessment_type`);
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
  requireOnlyFields(record, path, ["short_name", "long_name", "modules", "classification"]);
  decodeCourseClassification(field(record, "classification", path), `${path}.classification`);
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

// ASVS 1.5.2, 2.2.1, and 2.2.2: accept only the exact retained-ID-or-New shape.
function replacementChoice(value: unknown, path: string, identityField: string): void {
  const record = decodeRecord(value, path);
  const kind = decodeStringEnum(field(record, "kind", path), `${path}.kind`, ["retained", "new"]);
  requireOnlyFields(record, path, kind === "new" ? ["kind"] : ["kind", identityField]);
  if (kind === "retained")
    decodeNonemptyString(field(record, identityField, path), `${path}.${identityField}`);
}

function replacementModule(value: unknown, path: string): unknown {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["choice", "label", "assessments"]);
  replacementChoice(field(record, "choice", path), `${path}.choice`, "blueprint_module_id");
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
        "blueprint_assessment_id",
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
  requireOnlyFields(record, path, [
    "question_revision_tuple",
    "question_library",
    "selection_availability",
  ]);
  decodeQuestionRevisionTuple(
    field(record, "question_revision_tuple", path),
    `${path}.question_revision_tuple`,
  );
  decodeQuestionSearchResult(field(record, "question_library", path), `${path}.question_library`);
  decodeStringEnum(
    field(record, "selection_availability", path),
    `${path}.selection_availability`,
    ["available", "retained"],
  );
}

function contentView(value: unknown, path: string): void {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "assessment_type",
    "title",
    "instructions",
    "entries",
    "defaults",
  ]);
  assessmentType(field(record, "assessment_type", path), `${path}.assessment_type`);
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
          "question_pool_id",
          "question_pool_edit_number",
          "selection_count",
          "points_per_item",
          "scoring_rule",
          "selection_rule",
          "question_attempt_limit",
          "question_attempt_time_limit",
        ]);
        questionPoolId(
          field(entry, "question_pool_id", entryPath),
          `${entryPath}.question_pool_id`,
        );
        decodePositiveInteger(
          field(entry, "question_pool_edit_number", entryPath),
          `${entryPath}.question_pool_edit_number`,
        );
        decodePositiveInteger(
          field(entry, "selection_count", entryPath),
          `${entryPath}.selection_count`,
        );
        pointValue(field(entry, "points_per_item", entryPath), `${entryPath}.points_per_item`);
        decodeStringEnum(field(entry, "scoring_rule", entryPath), `${entryPath}.scoring_rule`, [
          "normal",
          "fullCredit",
          "extraCredit",
          "excluded",
        ]);
        selectionRule(field(entry, "selection_rule", entryPath), `${entryPath}.selection_rule`);
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

export function blueprintEditNumber(value: unknown, path: string): BlueprintEditNumber {
  const decoded = decodeString(value, path);
  if (!POSITIVE_REVISION.test(decoded) || BigInt(decoded) > 9_223_372_036_854_775_807n) {
    throw new DecodeError(path, "a positive Blueprint Edit Number");
  }
  return decoded;
}

export function blueprintRevisionTuple(value: unknown, path: string): BlueprintRevisionTuple {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["blueprintCourseId", "revisionNumber"]);
  return {
    blueprintCourseId: blueprintCourseId(
      field(record, "blueprintCourseId", path),
      `${path}.blueprintCourseId`,
    ),
    revisionNumber: revision(field(record, "revisionNumber", path), `${path}.revisionNumber`),
  };
}

function summary(value: unknown, path: string): BlueprintCourseSummaryView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "total_adoptions",
    "total_students_ever_enrolled",
    "id",
    "short_name",
    "long_name",
    "availability",
    "blueprint_edit_number",
    "classification",
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
    classification: decodeCourseClassification(
      field(record, "classification", path),
      `${path}.classification`,
    ),
    total_students_ever_enrolled: totalStudents,
    id: blueprintCourseId(field(record, "id", path), `${path}.id`),
    short_name: text(field(record, "short_name", path), `${path}.short_name`),
    long_name: text(field(record, "long_name", path), `${path}.long_name`),
    availability: availability(field(record, "availability", path), `${path}.availability`),
    blueprint_edit_number: blueprintEditNumber(
      field(record, "blueprint_edit_number", path),
      `${path}.blueprint_edit_number`,
    ),
    current_revision: blueprintRevisionTuple(
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
      requireOnlyFields(module, modulePath, ["blueprint_module_id", "label", "assessments"]);
      decodeNonemptyString(
        field(module, "blueprint_module_id", modulePath),
        `${modulePath}.blueprint_module_id`,
      );
      text(field(module, "label", modulePath), `${modulePath}.label`);
      decodeBoundedArray(
        field(module, "assessments", modulePath),
        `${modulePath}.assessments`,
        MAX_ASSESSMENT_ORDERED_ENTRIES,
        (contentValue, contentPath) => {
          const content = decodeRecord(contentValue, contentPath);
          requireOnlyFields(content, contentPath, ["blueprint_assessment_id", "content"]);
          decodeNonemptyString(
            field(content, "blueprint_assessment_id", contentPath),
            `${contentPath}.blueprint_assessment_id`,
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
    "id",
    "short_name",
    "long_name",
    "availability",
    "blueprint_edit_number",
    "classification",
    "current_revision",
    "fork_source",
    "read_access",
    "modules",
  ]);
  return {
    id: blueprintCourseId(field(record, "id", path), `${path}.id`),
    classification: decodeCourseClassification(
      field(record, "classification", path),
      `${path}.classification`,
    ),
    short_name: text(field(record, "short_name", path), `${path}.short_name`),
    long_name: text(field(record, "long_name", path), `${path}.long_name`),
    availability: availability(field(record, "availability", path), `${path}.availability`),
    blueprint_edit_number: blueprintEditNumber(
      field(record, "blueprint_edit_number", path),
      `${path}.blueprint_edit_number`,
    ),
    current_revision: blueprintRevisionTuple(
      field(record, "current_revision", path),
      `${path}.current_revision`,
    ),
    read_access: decodeStringEnum(field(record, "read_access", path), `${path}.read_access`, [
      "blueprint_course_owner",
      "active_instructor",
    ]),
    fork_source: decodeNullable(
      field(record, "fork_source", path),
      `${path}.fork_source`,
      blueprintRevisionTuple,
    ),
    modules: modules(field(record, "modules", path), `${path}.modules`),
  };
}

export function decodeBlueprintRevisionView(
  value: unknown,
  path = "response",
): BlueprintRevisionView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["blueprintRevisionTuple", "modules"]);
  return {
    blueprintRevisionTuple: blueprintRevisionTuple(
      field(record, "blueprintRevisionTuple", path),
      `${path}.blueprintRevisionTuple`,
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
  requireOnlyFields(record, path, [
    "short_name",
    "long_name",
    "availability",
    "blueprint_edit_number",
    "classification",
  ]);
  return {
    classification: decodeCourseClassification(
      field(record, "classification", path),
      `${path}.classification`,
    ),
    short_name: text(field(record, "short_name", path), `${path}.short_name`),
    long_name: text(field(record, "long_name", path), `${path}.long_name`),
    availability: availability(field(record, "availability", path), `${path}.availability`),
    blueprint_edit_number: blueprintEditNumber(
      field(record, "blueprint_edit_number", path),
      `${path}.blueprint_edit_number`,
    ),
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

export function decodeBlueprintCourseId(value: unknown, path = "id"): BlueprintCourseId {
  return blueprintCourseId(value, path);
}

function knownBlueprintFork(value: unknown, path: string): BlueprintKnownForkView {
  const record = decodeRecord(value, path);
  // ASVS 2.2.1, 8.2.3: reject substitute identities and hidden-child metadata.
  requireOnlyFields(record, path, [
    "id",
    "shortName",
    "longName",
    "availability",
    "currentRevision",
    "sourceRevision",
    "ownerDisplayName",
  ]);
  const ownerDisplayName = decodeNonemptyString(
    field(record, "ownerDisplayName", path),
    `${path}.ownerDisplayName`,
  );
  if (
    ownerDisplayName !== ownerDisplayName.trim() ||
    /[\p{Cc}]/u.test(ownerDisplayName) ||
    Array.from(ownerDisplayName).length > 200
  ) {
    throw new DecodeError(`${path}.ownerDisplayName`, "one verified Instructor display name");
  }
  return {
    id: blueprintCourseId(field(record, "id", path), `${path}.id`),
    shortName: text(field(record, "shortName", path), `${path}.shortName`),
    longName: text(field(record, "longName", path), `${path}.longName`),
    availability: availability(field(record, "availability", path), `${path}.availability`),
    currentRevision: revision(field(record, "currentRevision", path), `${path}.currentRevision`),
    sourceRevision: revision(field(record, "sourceRevision", path), `${path}.sourceRevision`),
    ownerDisplayName,
  };
}

/** Decodes only visible direct forks, without counts or pagination state. */
export function decodeKnownBlueprintForks(
  value: unknown,
  path = "response",
): readonly BlueprintKnownForkView[] {
  return decodeArray(value, path, knownBlueprintFork);
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
