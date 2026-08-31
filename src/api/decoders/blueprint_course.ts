// Strict browser decoding and local command validation for reusable Blueprint Courses.

import { MAX_ASSIGNMENT_CANDIDATES_PER_QUESTION_POOL } from "../../../generated/api/MAX_ASSIGNMENT_CANDIDATES_PER_QUESTION_POOL";
import { MAX_ASSIGNMENT_INSTRUCTIONS_UNICODE_SCALARS } from "../../../generated/api/MAX_ASSIGNMENT_INSTRUCTIONS_UNICODE_SCALARS";
import { MAX_ASSIGNMENT_ORDERED_ENTRIES } from "../../../generated/api/MAX_ASSIGNMENT_ORDERED_ENTRIES";
import { MAX_ASSIGNMENT_TOTAL_QUESTION_POOL_CANDIDATES } from "../../../generated/api/MAX_ASSIGNMENT_TOTAL_QUESTION_POOL_CANDIDATES";
import { MAX_QUESTION_CURATION_TITLE_UNICODE_SCALARS } from "../../../generated/api/MAX_QUESTION_CURATION_TITLE_UNICODE_SCALARS";
import type { BlueprintCourseSummaryView } from "../../../generated/api/BlueprintCourseSummaryView";
import type { BlueprintCourseView } from "../../../generated/api/BlueprintCourseView";
import type { BlueprintCourseReference } from "../../../generated/api/BlueprintCourseReference";
import type { CreateBlueprintCourseDefinitionInput } from "../../../generated/api/CreateBlueprintCourseDefinitionInput";
import type { ReplaceBlueprintCourseDefinitionInput } from "../../../generated/api/ReplaceBlueprintCourseDefinitionInput";
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
import { decodeStudentFeedbackReleaseRule } from "./assignment_policy";
import { decodeQuestionSummary } from "./question_library";
import { decodeBoundedArray, decodeCursor, field, requireOnlyFields } from "./shared";

const MAX_PAGE_SIZE = 100;
const QUESTION_ID = /^[0-9A-HJKMNP-TV-Z]{3}-[0-9A-HJKMNP-TV-Z]{4}$/u;
const POSITIVE_REVISION = /^[1-9][0-9]*$/u;
const LOCAL_TIME = /^([01][0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9]\.[0-9]{3}$/u;

function text(value: unknown, path: string): string {
  const decoded = decodeNonemptyString(value, path);
  if (
    decoded !== decoded.trim() ||
    Array.from(decoded).length > MAX_QUESTION_CURATION_TITLE_UNICODE_SCALARS
  ) {
    throw new DecodeError(path, "trimmed Blueprint Course text within its shared bound");
  }
  return decoded;
}

function blueprintReference(value: unknown, path: string): BlueprintCourseReference {
  const decoded = decodeString(value, path);
  if (!/^BP-[1-9][0-9]{0,9}$/u.test(decoded) || Number(decoded.slice(3)) > 2_147_483_647) {
    throw new DecodeError(path, "a canonical Blueprint Course public reference");
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
  if (!QUESTION_ID.test(decoded)) throw new DecodeError(path, "a canonical public Question ID");
  return decoded;
}

function pointValue(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  if (!/^(?:0|[1-9][0-9]*)(?:\.[0-9]{1,4})?$/u.test(decoded)) {
    throw new DecodeError(path, "a canonical nonnegative point decimal with at most four places");
  }
  return decoded;
}

function localTime(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  if (!LOCAL_TIME.test(decoded)) throw new DecodeError(path, "exact HH:MM:SS.sss local time");
  return decoded;
}

function scheduleMoment(value: unknown, path: string): unknown {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["day_offset", "local_time"]);
  return {
    day_offset: decodeSafeInteger(field(record, "day_offset", path), `${path}.day_offset`),
    local_time: localTime(field(record, "local_time", path), `${path}.local_time`),
  };
}

function schedule(value: unknown, path: string): unknown {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["available_at", "due_at", "closes_at"]);
  return {
    available_at: decodeNullable(
      field(record, "available_at", path),
      `${path}.available_at`,
      scheduleMoment,
    ),
    due_at: decodeNullable(field(record, "due_at", path), `${path}.due_at`, scheduleMoment),
    closes_at: decodeNullable(
      field(record, "closes_at", path),
      `${path}.closes_at`,
      scheduleMoment,
    ),
  };
}

function assignmentCompletionRule(value: unknown, path: string): unknown {
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

function assignmentAttemptContinuationRule(value: unknown, path: string): unknown {
  const record = decodeRecord(value, path);
  const kind = decodeString(field(record, "kind", path), `${path}.kind`);
  if (kind === "unlimited" || kind === "closed") {
    requireOnlyFields(record, path, ["kind"]);
    return { kind };
  }
  if (kind === "capped") {
    requireOnlyFields(record, path, ["kind", "maxAdditionalRuns"]);
    const maxAdditionalRuns = decodeSafeInteger(
      field(record, "maxAdditionalRuns", path),
      `${path}.maxAdditionalRuns`,
    );
    if (maxAdditionalRuns < 0)
      throw new DecodeError(`${path}.maxAdditionalRuns`, "a nonnegative safe integer");
    return { kind, maxAdditionalRuns };
  }
  throw new DecodeError(`${path}.kind`, "a known Assignment Attempt Continuation Rule");
}

function defaults(value: unknown, path: string): unknown {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "assignment_attempt_time_limit_seconds",
    "attempt_limit",
    "late_work_rule",
    "assignment_deadline_rule",
    "activity_rules",
    "student_feedback_release_rule",
  ]);
  const policies = decodeRecord(field(record, "activity_rules", path), `${path}.activity_rules`);
  requireOnlyFields(policies, `${path}.activity_rules`, [
    "assignmentCompletionRule",
    "assignmentAttemptGradeRule",
    "assignmentAttemptContinuationRule",
    "questionVariationRule",
    "assignmentAttemptResumeRule",
    "assignmentQuestionDisplayRule",
    "assignmentNavigationRule",
    "assignmentQuestionOrderRule",
  ]);
  return {
    assignment_attempt_time_limit_seconds: decodeNullable(
      field(record, "assignment_attempt_time_limit_seconds", path),
      `${path}.assignment_attempt_time_limit_seconds`,
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
      ["accept", "markLate", "reject"],
    ),
    assignment_deadline_rule: decodeStringEnum(
      field(record, "assignment_deadline_rule", path),
      `${path}.assignment_deadline_rule`,
      ["autoSubmit"],
    ),
    activity_rules: {
      assignmentCompletionRule: assignmentCompletionRule(
        field(policies, "assignmentCompletionRule", `${path}.activity_rules`),
        `${path}.activity_rules.assignmentCompletionRule`,
      ),
      assignmentAttemptGradeRule: decodeStringEnum(
        field(policies, "assignmentAttemptGradeRule", `${path}.activity_rules`),
        `${path}.activity_rules.assignmentAttemptGradeRule`,
        ["first", "latest", "highest", "instructorSelected"],
      ),
      assignmentAttemptContinuationRule: assignmentAttemptContinuationRule(
        field(policies, "assignmentAttemptContinuationRule", `${path}.activity_rules`),
        `${path}.activity_rules.assignmentAttemptContinuationRule`,
      ),
      questionVariationRule: decodeStringEnum(
        field(policies, "questionVariationRule", `${path}.activity_rules`),
        `${path}.activity_rules.questionVariationRule`,
        ["reuseQuestionsWithNewSeeds", "selectedQuestionVariants", "redrawQuestionPools"],
      ),
      assignmentAttemptResumeRule: decodeStringEnum(
        field(policies, "assignmentAttemptResumeRule", `${path}.activity_rules`),
        `${path}.activity_rules.assignmentAttemptResumeRule`,
        ["resumable", "singleSession"],
      ),
      assignmentQuestionDisplayRule: decodeStringEnum(
        field(policies, "assignmentQuestionDisplayRule", `${path}.activity_rules`),
        `${path}.activity_rules.assignmentQuestionDisplayRule`,
        ["allQuestions", "oneQuestionAtATime"],
      ),
      assignmentNavigationRule: decodeStringEnum(
        field(policies, "assignmentNavigationRule", `${path}.activity_rules`),
        `${path}.activity_rules.assignmentNavigationRule`,
        ["freeNavigation", "forwardOnly"],
      ),
      assignmentQuestionOrderRule: decodeStringEnum(
        field(policies, "assignmentQuestionOrderRule", `${path}.activity_rules`),
        `${path}.activity_rules.assignmentQuestionOrderRule`,
        ["authoredOrder", "shuffled"],
      ),
    },
    student_feedback_release_rule: decodeStudentFeedbackReleaseRule(
      field(record, "student_feedback_release_rule", path),
      `${path}.student_feedback_release_rule`,
    ),
  };
}

function assignmentEntry(
  value: unknown,
  path: string,
): { kind: "fixed" | "pool"; candidates: string[] } {
  const record = decodeRecord(value, path);
  const kind = decodeStringEnum(field(record, "kind", path), `${path}.kind`, ["fixed", "pool"]);
  if (kind === "fixed") {
    requireOnlyFields(record, path, ["kind", "question_id", "points_possible", "scoring_rule"]);
    questionId(field(record, "question_id", path), `${path}.question_id`);
    pointValue(field(record, "points_possible", path), `${path}.points_possible`);
    decodeStringEnum(field(record, "scoring_rule", path), `${path}.scoring_rule`, [
      "normal",
      "fullCredit",
      "extraCredit",
      "excluded",
    ]);
    return { kind, candidates: [] };
  }
  requireOnlyFields(record, path, [
    "kind",
    "candidates",
    "draw_count",
    "points_per_item",
    "scoring_rule",
    "selection_rule",
  ]);
  const candidates = decodeBoundedArray(
    field(record, "candidates", path),
    `${path}.candidates`,
    MAX_ASSIGNMENT_CANDIDATES_PER_QUESTION_POOL,
    questionId,
  );
  const drawCount = decodePositiveInteger(field(record, "draw_count", path), `${path}.draw_count`);
  if (
    candidates.length === 0 ||
    drawCount > candidates.length ||
    new Set(candidates).size !== candidates.length
  ) {
    throw new DecodeError(path, "a nonempty pool with distinct candidates and a valid draw count");
  }
  pointValue(field(record, "points_per_item", path), `${path}.points_per_item`);
  selectionRule(field(record, "selection_rule", path), `${path}.selection_rule`);
  return { kind, candidates };
}

function selectionRule(value: unknown, path: string): void {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["algorithm", "ordering"]);
  decodeStringEnum(field(record, "algorithm", path), `${path}.algorithm`, ["v1"]);
  decodeStringEnum(field(record, "ordering", path), `${path}.ordering`, [
    "candidateOrder",
    "randomized",
  ]);
}

function assignmentDefinition(value: unknown, path: string): unknown {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["title", "instructions", "entries", "defaults", "schedule"]);
  const instructions = decodeString(field(record, "instructions", path), `${path}.instructions`);
  if (Array.from(instructions).length > MAX_ASSIGNMENT_INSTRUCTIONS_UNICODE_SCALARS)
    throw new DecodeError(`${path}.instructions`, "instructions within the shared bound");
  const entries = decodeBoundedArray(
    field(record, "entries", path),
    `${path}.entries`,
    MAX_ASSIGNMENT_ORDERED_ENTRIES,
    assignmentEntry,
  );
  if (entries.length === 0) throw new DecodeError(`${path}.entries`, "at least one ordered entry");
  const candidateCount = entries.reduce((total, entry) => total + entry.candidates.length, 0);
  if (candidateCount > MAX_ASSIGNMENT_TOTAL_QUESTION_POOL_CANDIDATES)
    throw new DecodeError(`${path}.entries`, "pool candidates within the assignment total bound");
  defaults(field(record, "defaults", path), `${path}.defaults`);
  schedule(field(record, "schedule", path), `${path}.schedule`);
  return value;
}

function createModule(value: unknown, path: string): unknown {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["label", "definitions"]);
  const definitions = decodeBoundedArray(
    field(record, "definitions", path),
    `${path}.definitions`,
    MAX_ASSIGNMENT_ORDERED_ENTRIES,
    assignmentDefinition,
  );
  if (definitions.length === 0)
    throw new DecodeError(`${path}.definitions`, "at least one definition");
  text(field(record, "label", path), `${path}.label`);
  return value;
}

export function decodeCreateBlueprintCourseDefinitionInput(
  value: unknown,
  path = "request",
): CreateBlueprintCourseDefinitionInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["title", "modules"]);
  const modules = decodeBoundedArray(
    field(record, "modules", path),
    `${path}.modules`,
    MAX_ASSIGNMENT_ORDERED_ENTRIES,
    createModule,
  );
  if (modules.length === 0) throw new DecodeError(`${path}.modules`, "at least one ordered module");
  text(field(record, "title", path), `${path}.title`);
  return value as CreateBlueprintCourseDefinitionInput;
}

function replacementHandle(value: unknown, path: string, idField: string): void {
  const record = decodeRecord(value, path);
  const kind = decodeStringEnum(field(record, "kind", path), `${path}.kind`, ["retained", "new"]);
  requireOnlyFields(record, path, kind === "new" ? ["kind"] : ["kind", idField]);
  if (kind === "retained") decodeNonemptyString(field(record, idField, path), `${path}.${idField}`);
}

function replacementModule(value: unknown, path: string): unknown {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["handle", "label", "definitions"]);
  replacementHandle(field(record, "handle", path), `${path}.handle`, "module_id");
  text(field(record, "label", path), `${path}.label`);
  const definitions = decodeBoundedArray(
    field(record, "definitions", path),
    `${path}.definitions`,
    MAX_ASSIGNMENT_ORDERED_ENTRIES,
    (definitionValue, definitionPath) => {
      const definition = decodeRecord(definitionValue, definitionPath);
      requireOnlyFields(definition, definitionPath, ["handle", "definition"]);
      replacementHandle(
        field(definition, "handle", definitionPath),
        `${definitionPath}.handle`,
        "assignment_id",
      );
      assignmentDefinition(
        field(definition, "definition", definitionPath),
        `${definitionPath}.definition`,
      );
      return definitionValue;
    },
  );
  if (definitions.length === 0)
    throw new DecodeError(`${path}.definitions`, "at least one definition");
  return value;
}

export function decodeReplaceBlueprintCourseDefinitionInput(
  value: unknown,
  path = "request",
): ReplaceBlueprintCourseDefinitionInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["title", "modules"]);
  const modules = decodeBoundedArray(
    field(record, "modules", path),
    `${path}.modules`,
    MAX_ASSIGNMENT_ORDERED_ENTRIES,
    replacementModule,
  );
  if (modules.length === 0) throw new DecodeError(`${path}.modules`, "at least one ordered module");
  text(field(record, "title", path), `${path}.title`);
  return value as ReplaceBlueprintCourseDefinitionInput;
}

function questionView(value: unknown, path: string): void {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["question_library", "selection_availability"]);
  const questionLibrary = decodeRecord(
    field(record, "question_library", path),
    `${path}.question_library`,
  );
  requireOnlyFields(questionLibrary, `${path}.question_library`, ["summary", "evidence"]);
  decodeQuestionSummary(
    field(questionLibrary, "summary", `${path}.question_library`),
    `${path}.question_library.summary`,
    true,
  );
  const evidence = decodeRecord(
    field(questionLibrary, "evidence", `${path}.question_library`),
    `${path}.question_library.evidence`,
  );
  const state = decodeStringEnum(
    field(evidence, "state", `${path}.question_library.evidence`),
    `${path}.question_library.evidence.state`,
    ["insufficientEvidence", "available"],
  );
  if (state === "insufficientEvidence") {
    requireOnlyFields(evidence, `${path}.question_library.evidence`, ["state"]);
  }
  decodeStringEnum(
    field(record, "selection_availability", path),
    `${path}.selection_availability`,
    ["available", "retained"],
  );
}

function definitionView(value: unknown, path: string): void {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["title", "instructions", "entries", "defaults", "schedule"]);
  text(field(record, "title", path), `${path}.title`);
  decodeString(field(record, "instructions", path), `${path}.instructions`);
  defaults(field(record, "defaults", path), `${path}.defaults`);
  schedule(field(record, "schedule", path), `${path}.schedule`);
  decodeBoundedArray(
    field(record, "entries", path),
    `${path}.entries`,
    MAX_ASSIGNMENT_ORDERED_ENTRIES,
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
        ]);
        questionView(field(entry, "question", entryPath), `${entryPath}.question`);
      } else {
        requireOnlyFields(entry, entryPath, [
          "kind",
          "candidates",
          "draw_count",
          "points_per_item",
          "scoring_rule",
          "selection_rule",
        ]);
        decodeBoundedArray(
          field(entry, "candidates", entryPath),
          `${entryPath}.candidates`,
          MAX_ASSIGNMENT_CANDIDATES_PER_QUESTION_POOL,
          questionView,
        );
      }
      return entryValue;
    },
  );
}

function summary(value: unknown, path: string): BlueprintCourseSummaryView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["reference", "title", "revision", "access"]);
  return {
    reference: blueprintReference(field(record, "reference", path), `${path}.reference`),
    title: text(field(record, "title", path), `${path}.title`),
    revision: revision(field(record, "revision", path), `${path}.revision`),
    access: decodeStringEnum(field(record, "access", path), `${path}.access`, [
      "owner",
      "approved_instructor",
    ]),
  };
}

export function decodeBlueprintCourseView(value: unknown, path = "response"): BlueprintCourseView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["reference", "title", "revision", "access", "modules"]);
  const modules = decodeBoundedArray(
    field(record, "modules", path),
    `${path}.modules`,
    MAX_ASSIGNMENT_ORDERED_ENTRIES,
    (moduleValue, modulePath) => {
      const module = decodeRecord(moduleValue, modulePath);
      requireOnlyFields(module, modulePath, ["module_id", "label", "definitions"]);
      decodeNonemptyString(field(module, "module_id", modulePath), `${modulePath}.module_id`);
      text(field(module, "label", modulePath), `${modulePath}.label`);
      decodeBoundedArray(
        field(module, "definitions", modulePath),
        `${modulePath}.definitions`,
        MAX_ASSIGNMENT_ORDERED_ENTRIES,
        (definitionValue, definitionPath) => {
          const definition = decodeRecord(definitionValue, definitionPath);
          requireOnlyFields(definition, definitionPath, ["assignment_id", "definition"]);
          decodeNonemptyString(
            field(definition, "assignment_id", definitionPath),
            `${definitionPath}.assignment_id`,
          );
          definitionView(
            field(definition, "definition", definitionPath),
            `${definitionPath}.definition`,
          );
          return definitionValue;
        },
      );
      return moduleValue;
    },
  );
  return {
    reference: blueprintReference(field(record, "reference", path), `${path}.reference`),
    title: text(field(record, "title", path), `${path}.title`),
    revision: revision(field(record, "revision", path), `${path}.revision`),
    access: decodeStringEnum(field(record, "access", path), `${path}.access`, [
      "owner",
      "approved_instructor",
    ]),
    modules: modules as BlueprintCourseView["modules"],
  };
}

export function decodeBlueprintCourseReference(
  value: unknown,
  path = "reference",
): BlueprintCourseReference {
  return blueprintReference(value, path);
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
