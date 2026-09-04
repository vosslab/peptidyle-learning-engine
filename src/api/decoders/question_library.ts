// Question Library, course, and assignment browser-visible API DTOs.

import type { AssignmentEntryAvailability } from "../../../generated/api/AssignmentEntryAvailability";
import type { QuestionPoolItemAvailability } from "../../../generated/api/QuestionPoolItemAvailability";
import type { FixedQuestionAssignmentEntrySummary as FixedQuestionAssignmentEntry } from "../../../generated/api/FixedQuestionAssignmentEntrySummary";
import type { AssignmentEntrySummary } from "../../../generated/api/AssignmentEntrySummary";
import type { AssignmentEntryScoringRule } from "../../../generated/api/AssignmentEntryScoringRule";
import type { QuestionPoolItemSummary as QuestionPoolItem } from "../../../generated/api/QuestionPoolItemSummary";
import type { QuestionPoolAssignmentEntrySummary as QuestionPoolAssignmentEntry } from "../../../generated/api/QuestionPoolAssignmentEntrySummary";
import type { AssignmentSummary } from "../../../generated/api/AssignmentSummary";
import type { QuestionStatistics } from "../../../generated/api/QuestionStatistics";
import type { QuestionSearchResult } from "../../../generated/api/QuestionSearchResult";
import type { CourseQuestionUse } from "../../../generated/api/CourseQuestionUse";
import type { QuestionDetails } from "../../../generated/api/QuestionDetails";
import type { QuestionSummary } from "../../../generated/api/QuestionSummary";
import type { QuestionDetailsPromptView } from "../../../generated/api/QuestionDetailsPromptView";
import type { QuestionSearchPage } from "../../../generated/api/QuestionSearchPage";
import type { QuestionUseDetails } from "../../../generated/api/QuestionUseDetails";
import type { QuestionUseSummary } from "../../../generated/api/QuestionUseSummary";
import type { AssignmentCompletionRule } from "../../../generated/api/AssignmentCompletionRule";
import type { AssignmentAttemptContinuationRule } from "../../../generated/api/AssignmentAttemptContinuationRule";
import type { CourseSummary } from "../../../generated/api/CourseSummary";
import { decodeQuestionAuthorship } from "../question_authorship";
import type { AssignmentPointValue } from "../../../generated/api/AssignmentPointValue";
import type { AssignmentActivityRules } from "../../../generated/api/AssignmentActivityRules";
import type { QuestionPoolSelectedQuestionOrder } from "../../../generated/api/QuestionPoolSelectedQuestionOrder";
import type { QuestionPoolSelectionRule } from "../../../generated/api/QuestionPoolSelectionRule";
import type {
  AssignmentContentInput,
  AssignmentEditorEntryInput,
  CourseCreateInput,
  CourseRouteView,
} from "../contracts";
import {
  DecodeError,
  decodeArray,
  decodeBoolean,
  decodeFiniteNumber,
  decodeNonemptyString,
  decodeNonnegativeInteger,
  decodeNullable,
  decodePositiveInteger,
  decodeRecord,
  decodeString,
  decodeStringEnum,
} from "../decoder";
import { MAX_QUESTION_POOL_ITEMS_PER_ASSIGNMENT_ENTRY } from "../../../generated/api/MAX_QUESTION_POOL_ITEMS_PER_ASSIGNMENT_ENTRY";
import { MAX_ASSIGNMENT_ORDERED_ENTRIES } from "../../../generated/api/MAX_ASSIGNMENT_ORDERED_ENTRIES";
import { MAX_ASSIGNMENT_QUESTION_POOL_ITEMS } from "../../../generated/api/MAX_ASSIGNMENT_QUESTION_POOL_ITEMS";
import { MAX_QUESTION_SEARCH_OWN_COURSE_USAGES } from "../../../generated/api/MAX_QUESTION_SEARCH_OWN_COURSE_USAGES";
import {
  MAX_QUESTION_SEARCH_PAGE_ITEMS,
  decodeAssignmentReference,
  decodeAssignmentTitle,
  decodeCourseTitle,
  decodeQuestionBackendCapabilities,
  decodeBoundedArray,
  decodeQuestionRevisionReference,
  decodeQuestionRevisionAvailability,
  decodeCursor,
  decodeQuestionTitle,
  decodeCourseInstanceReference,
  decodeIdentifier,
  decodeQuestionMetadata,
  decodeQuestionId,
  decodeTimestamp,
  field,
  kind,
  requireOnlyFields,
} from "./shared";
import { decodeCourseTerm } from "./course_term";
import {
  decodeQuestionAttemptLimit,
  decodeQuestionAttemptTimeLimit,
  decodeQuestionContentBlock,
} from "./question_model";
import { decodeStudentFeedbackReleaseRule } from "./assignment_policy";
import { decodeCourseAppearanceView } from "./course_appearance";
import { decodeQuestionSearchFacets } from "./question_type_facets";

// Reuse the Question Library course import surface while course-term owns its decoding rules.
export { decodeCourseTerm, decodeCourseTermValidationFailure } from "./course_term";
export { decodeStudentFeedbackReleaseRule } from "./assignment_policy";
export {
  decodeCourseAppearanceView,
  decodeCourseAppearanceUpdate,
  decodeCourseBannerUploadReceipt,
} from "./course_appearance";

export function decodeQuestionSummary(
  value: unknown,
  path = "response",
  strict = false,
): QuestionSummary {
  const record = decodeRecord(value, path);
  if (strict) {
    requireOnlyFields(record, path, [
      "questionId",
      "latestQuestionRevision",
      "backend",
      "questionType",
      "capabilities",
      "metadata",
      "availability",
      "publishedAt",
      "authorship",
    ]);
  }
  const decoded = {
    questionId: decodeQuestionId(field(record, "questionId", path), `${path}.questionId`),
    latestQuestionRevision: decodeQuestionRevisionReference(
      field(record, "latestQuestionRevision", path),
      `${path}.latestQuestionRevision`,
      strict,
    ),
    backend: decodeStringEnum(field(record, "backend", path), `${path}.backend`, [
      "ple",
      "webwork",
      "imathas",
    ]),
    questionType: decodeStringEnum(field(record, "questionType", path), `${path}.questionType`, [
      "multipleChoice",
      "multipleAnswer",
      "fillInBlank",
      "multipleFillInBlank",
      "numeric",
      "matching",
      "ordering",
      "hotspot",
    ]),
    capabilities: decodeQuestionBackendCapabilities(
      field(record, "capabilities", path),
      `${path}.capabilities`,
    ),
    metadata: decodeQuestionMetadata(field(record, "metadata", path), `${path}.metadata`, strict),
    authorship: decodeQuestionAuthorship(field(record, "authorship", path), `${path}.authorship`),
    availability: decodeQuestionRevisionAvailability(
      field(record, "availability", path),
      `${path}.availability`,
      strict,
    ),
    publishedAt: decodeTimestamp(field(record, "publishedAt", path), `${path}.publishedAt`),
  } satisfies QuestionSummary;
  if (decoded.latestQuestionRevision.questionId !== decoded.questionId) {
    throw new DecodeError(
      `${path}.latestQuestionRevision.questionId`,
      "the Question Summary questionId",
    );
  }
  return decoded;
}

/**
 * Verifies the exact browser-safe Question Summary command result for a PLE publication.
 *
 * Decoding establishes the DTO's shape; callers of a publication command must
 * additionally bind that DTO to the published state.
 */
export function isAvailablePleQuestionSummary(summary: QuestionSummary): boolean {
  return summary.backend === "ple" && summary.availability.availability === "available";
}

function decodeQuestionStatistics(value: unknown, path: string): QuestionStatistics {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["state"]);
  const state = decodeStringEnum(field(record, "state", path), `${path}.state`, ["unavailable"]);
  return { state };
}

export function decodeQuestionSearchResult(value: unknown, path: string): QuestionSearchResult {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["summary", "evidence"]);
  return {
    summary: decodeQuestionSummary(field(record, "summary", path), `${path}.summary`, true),
    evidence: decodeQuestionStatistics(field(record, "evidence", path), `${path}.evidence`),
  };
}

function decodeQuestionUseSummary(value: unknown, path: string): QuestionUseSummary {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "globalCourseCount",
    "globalAssignmentCount",
    "ownCourseCount",
    "ownAssignmentCount",
  ]);
  const globalCourseCount = decodeNonnegativeInteger(
    field(record, "globalCourseCount", path),
    `${path}.globalCourseCount`,
  );
  const globalAssignmentCount = decodeNonnegativeInteger(
    field(record, "globalAssignmentCount", path),
    `${path}.globalAssignmentCount`,
  );
  const ownCourseCount = decodeNonnegativeInteger(
    field(record, "ownCourseCount", path),
    `${path}.ownCourseCount`,
  );
  const ownAssignmentCount = decodeNonnegativeInteger(
    field(record, "ownAssignmentCount", path),
    `${path}.ownAssignmentCount`,
  );
  if (ownCourseCount > globalCourseCount || ownAssignmentCount > globalAssignmentCount) {
    throw new DecodeError(path, "usage counts within their installation-wide totals");
  }
  if (globalAssignmentCount < globalCourseCount || ownAssignmentCount < ownCourseCount) {
    throw new DecodeError(path, "assignment counts at least as large as their course counts");
  }
  return {
    globalCourseCount,
    globalAssignmentCount,
    ownCourseCount,
    ownAssignmentCount,
  };
}

function decodeCourseQuestionUse(value: unknown, path: string): CourseQuestionUse {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["course", "title", "assignmentCount"]);
  return {
    course: decodeCourseInstanceReference(field(record, "course", path), `${path}.course`),
    title: decodeCourseTitle(field(record, "title", path), `${path}.title`),
    assignmentCount: decodePositiveInteger(
      field(record, "assignmentCount", path),
      `${path}.assignmentCount`,
    ),
  };
}

function decodeQuestionUseDetails(value: unknown, path: string): QuestionUseDetails {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["summary", "ownCourses", "ownCoursesTruncated"]);
  const summary = decodeQuestionUseSummary(field(record, "summary", path), `${path}.summary`);
  const ownCourses = decodeBoundedArray(
    field(record, "ownCourses", path),
    `${path}.ownCourses`,
    MAX_QUESTION_SEARCH_OWN_COURSE_USAGES,
    decodeCourseQuestionUse,
  );
  const seenCourses = new Set<string>();
  for (const courseUsage of ownCourses) {
    if (seenCourses.has(courseUsage.course)) {
      throw new DecodeError(`${path}.ownCourses`, "unique course references");
    }
    seenCourses.add(courseUsage.course);
  }
  const ownCoursesTruncated = decodeBoolean(
    field(record, "ownCoursesTruncated", path),
    `${path}.ownCoursesTruncated`,
  );
  if (!ownCoursesTruncated && ownCourses.length !== summary.ownCourseCount) {
    throw new DecodeError(
      `${path}.ownCourses`,
      "a complete list matching ownCourseCount when not truncated",
    );
  }
  if (
    ownCoursesTruncated &&
    (ownCourses.length !== MAX_QUESTION_SEARCH_OWN_COURSE_USAGES ||
      summary.ownCourseCount <= MAX_QUESTION_SEARCH_OWN_COURSE_USAGES)
  ) {
    throw new DecodeError(
      `${path}.ownCoursesTruncated`,
      `true only for ${MAX_QUESTION_SEARCH_OWN_COURSE_USAGES} listed rows with additional own courses`,
    );
  }
  return {
    summary,
    ownCourses,
    ownCoursesTruncated,
  };
}

function decodeQuestionDetailsPromptView(value: unknown, path: string): QuestionDetailsPromptView {
  const record = decodeRecord(value, path);
  const promptKind = kind(record, path);
  if (promptKind !== "static" && promptKind !== "generatedExample") {
    throw new DecodeError(`${path}.kind`, "a known Question Details Prompt View");
  }
  requireOnlyFields(record, path, ["kind", "blocks"]);
  return {
    kind: promptKind,
    blocks: decodeBoundedArray(
      field(record, "blocks", path),
      `${path}.blocks`,
      MAX_QUESTION_SEARCH_PAGE_ITEMS,
      (block, blockPath) => decodeQuestionContentBlock(block, blockPath, true),
    ),
  };
}
/** Strict, bounded metadata-only Question Search Results View. */
export function decodeQuestionSearchPage(value: unknown, path = "response"): QuestionSearchPage {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["items", "nextCursor", "facets"]);
  return {
    items: decodeBoundedArray(
      field(record, "items", path),
      `${path}.items`,
      MAX_QUESTION_SEARCH_PAGE_ITEMS,
      decodeQuestionSearchResult,
    ),
    nextCursor: decodeNullable(
      field(record, "nextCursor", path),
      `${path}.nextCursor`,
      decodeCursor,
    ),
    facets: decodeQuestionSearchFacets(field(record, "facets", path), `${path}.facets`),
  };
}

/** Strict safe immutable Question Details View; source and grading fields are rejected. */
export function decodeQuestionDetails(value: unknown, path = "response"): QuestionDetails {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["summary", "prompt", "evidence", "usage"]);
  return {
    summary: decodeQuestionSummary(field(record, "summary", path), `${path}.summary`, true),
    prompt: decodeQuestionDetailsPromptView(field(record, "prompt", path), `${path}.prompt`),
    evidence: decodeQuestionStatistics(field(record, "evidence", path), `${path}.evidence`),
    usage: decodeQuestionUseDetails(field(record, "usage", path), `${path}.usage`),
  };
}

function decodeAssignmentCompletionRule(
  value: unknown,
  path: string,
  strict = false,
): AssignmentCompletionRule {
  const record = decodeRecord(value, path);
  const requirement = kind(record, path);
  switch (requirement) {
    case "answerAll":
    case "allCorrect":
      if (strict) requireOnlyFields(record, path, ["kind"]);
      return { kind: requirement };
    case "scoreAtLeast": {
      if (strict) requireOnlyFields(record, path, ["kind", "fraction"]);
      const decoded = {
        kind: requirement,
        fraction: decodeFiniteNumber(field(record, "fraction", path), `${path}.fraction`),
      } satisfies AssignmentCompletionRule;
      return decoded;
    }
    default:
      throw new DecodeError(`${path}.kind`, "a known Assignment Completion Rule");
  }
}

function decodeAssignmentAttemptContinuationRule(
  value: unknown,
  path: string,
  strict = false,
): AssignmentAttemptContinuationRule {
  const record = decodeRecord(value, path);
  const practice = kind(record, path);
  switch (practice) {
    case "unlimited":
    case "closed":
      if (strict) requireOnlyFields(record, path, ["kind"]);
      return { kind: practice };
    case "capped": {
      if (strict) requireOnlyFields(record, path, ["kind", "maxAdditionalAssignmentAttempts"]);
      const decoded = {
        kind: practice,
        maxAdditionalAssignmentAttempts: decodeNonnegativeInteger(
          field(record, "maxAdditionalAssignmentAttempts", path),
          `${path}.maxAdditionalAssignmentAttempts`,
        ),
      } satisfies AssignmentAttemptContinuationRule;
      return decoded;
    }
    default:
      throw new DecodeError(`${path}.kind`, "a known Assignment Attempt Continuation Rule");
  }
}

function decodeAssignmentActivityRules(
  value: unknown,
  path: string,
  strict = false,
): AssignmentActivityRules {
  const record = decodeRecord(value, path);
  if (strict) {
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
  }
  const decoded = {
    assignmentCompletionRule: decodeAssignmentCompletionRule(
      field(record, "assignmentCompletionRule", path),
      `${path}.assignmentCompletionRule`,
      strict,
    ),
    assignmentAttemptGradeRule: decodeStringEnum(
      field(record, "assignmentAttemptGradeRule", path),
      `${path}.assignmentAttemptGradeRule`,
      ["first", "latest", "highest", "instructorSelected"],
    ),
    assignmentAttemptContinuationRule: decodeAssignmentAttemptContinuationRule(
      field(record, "assignmentAttemptContinuationRule", path),
      `${path}.assignmentAttemptContinuationRule`,
      strict,
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
  } satisfies AssignmentActivityRules;
  return decoded;
}

export function decodeCourseSummary(value: unknown, path = "response"): CourseSummary {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["id", "reference", "title", "term", "role"]);
  const decoded = {
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    reference: decodeCourseInstanceReference(field(record, "reference", path), `${path}.reference`),
    title: decodeCourseTitle(field(record, "title", path), `${path}.title`),
    term: decodeCourseTerm(field(record, "term", path), `${path}.term`),
    role: decodeStringEnum(field(record, "role", path), `${path}.role`, ["student", "instructor"]),
  } satisfies CourseSummary;
  return decoded;
}

/** Strict request decoder for the public course-creation transport boundary. */
export function decodeCourseCreateInput(value: unknown, path = "request"): CourseCreateInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["title", "term"]);
  const title = decodeCourseTitle(field(record, "title", path), `${path}.title`);
  const decoded = {
    title,
    term: decodeCourseTerm(field(record, "term", path), `${path}.term`),
  } satisfies CourseCreateInput;
  return decoded;
}

export function decodeCourseRouteView(value: unknown, path: string): CourseRouteView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["summary", "appearance"]);
  return {
    summary: decodeCourseSummary(field(record, "summary", path), `${path}.summary`),
    appearance: decodeCourseAppearanceView(field(record, "appearance", path), `${path}.appearance`),
  };
}

function decodeAssignmentPointValue(value: unknown, path: string): AssignmentPointValue {
  const decoded = decodeString(value, path);
  if (!/^(?:0|[1-9][0-9]{0,9})(?:\.[0-9]{1,4})?$/u.test(decoded)) {
    throw new DecodeError(path, "a canonical nonnegative decimal with at most four places");
  }
  const [whole = "0"] = decoded.split(".", 1);
  if (BigInt(whole) > 1_000_000_000n) {
    throw new DecodeError(path, "an assignment point value in the supported range");
  }
  return decoded;
}

function decodeFixedQuestionAssignmentEntry(
  value: unknown,
  path: string,
): FixedQuestionAssignmentEntry {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "kind",
    "id",
    "questionId",
    "questionTitle",
    "backend",
    "capabilities",
    "pointsPossible",
    "availability",
    "scoringRule",
    "questionAttemptLimit",
    "questionAttemptTimeLimit",
  ]);
  return {
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    questionId: decodeQuestionId(field(record, "questionId", path), `${path}.questionId`),
    questionTitle: decodeQuestionTitle(
      field(record, "questionTitle", path),
      `${path}.questionTitle`,
    ),
    backend: decodeStringEnum(field(record, "backend", path), `${path}.backend`, [
      "ple",
      "webwork",
      "imathas",
    ]),
    capabilities: decodeQuestionBackendCapabilities(
      field(record, "capabilities", path),
      `${path}.capabilities`,
    ),
    pointsPossible: decodeAssignmentPointValue(
      field(record, "pointsPossible", path),
      `${path}.pointsPossible`,
    ),
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
    ),
    questionAttemptTimeLimit: decodeQuestionAttemptTimeLimit(
      field(record, "questionAttemptTimeLimit", path),
      `${path}.questionAttemptTimeLimit`,
    ),
  };
}

/** Request-only entry shape: the server owns display metadata and all internal identities. */
function decodeAssignmentContentEntry(value: unknown, path: string): AssignmentEditorEntryInput {
  const record = decodeRecord(value, path);
  const entryKind = decodeStringEnum(field(record, "kind", path), `${path}.kind`, [
    "fixedQuestion",
    "questionPool",
  ] as const);
  if (entryKind === "fixedQuestion") {
    requireOnlyFields(record, path, [
      "kind",
      "questionId",
      "pointsPossible",
      "availability",
      "scoringRule",
      "questionAttemptLimit",
      "questionAttemptTimeLimit",
    ]);
    return {
      kind: "fixedQuestion",
      questionId: decodeQuestionId(field(record, "questionId", path), `${path}.questionId`),
      pointsPossible: decodeAssignmentPointValue(
        field(record, "pointsPossible", path),
        `${path}.pointsPossible`,
      ),
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
      ),
      questionAttemptTimeLimit: decodeQuestionAttemptTimeLimit(
        field(record, "questionAttemptTimeLimit", path),
        `${path}.questionAttemptTimeLimit`,
      ),
    };
  }
  requireOnlyFields(record, path, [
    "kind",
    "questionIds",
    "availability",
    "scoringRule",
    "selectionCount",
    "pointsPerItem",
    "selectionRule",
    "questionAttemptLimit",
    "questionAttemptTimeLimit",
  ]);
  const questionIds = decodeBoundedArray(
    field(record, "questionIds", path),
    `${path}.questionIds`,
    MAX_QUESTION_POOL_ITEMS_PER_ASSIGNMENT_ENTRY,
    decodeQuestionId,
  );
  if (new Set(questionIds).size !== questionIds.length)
    throw new DecodeError(`${path}.questionIds`, "unique Question IDs");
  const selectionCount = decodePositiveInteger(
    field(record, "selectionCount", path),
    `${path}.selectionCount`,
  );
  if (selectionCount > questionIds.length)
    throw new DecodeError(
      `${path}.selectionCount`,
      "a value no greater than the Question Pool Item count",
    );
  return {
    kind: "questionPool",
    questionIds,
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
    pointsPerItem: decodeAssignmentPointValue(
      field(record, "pointsPerItem", path),
      `${path}.pointsPerItem`,
    ),
    selectionRule: decodeQuestionPoolSelectionRule(
      field(record, "selectionRule", path),
      `${path}.selectionRule`,
    ),
    questionAttemptLimit: decodeQuestionAttemptLimit(
      field(record, "questionAttemptLimit", path),
      `${path}.questionAttemptLimit`,
    ),
    questionAttemptTimeLimit: decodeQuestionAttemptTimeLimit(
      field(record, "questionAttemptTimeLimit", path),
      `${path}.questionAttemptTimeLimit`,
    ),
  };
}

function decodeAssignmentContentEntries(
  value: unknown,
  path: string,
): ReadonlyArray<AssignmentEditorEntryInput> {
  const entries = decodeBoundedArray(
    value,
    path,
    MAX_ASSIGNMENT_ORDERED_ENTRIES,
    decodeAssignmentContentEntry,
  );
  const questionPoolItemCount = entries.reduce(
    (total, entry) => total + (entry.kind === "questionPool" ? entry.questionIds.length : 0),
    0,
  );
  if (questionPoolItemCount > MAX_ASSIGNMENT_QUESTION_POOL_ITEMS)
    throw new DecodeError(
      path,
      `no more than ${MAX_ASSIGNMENT_QUESTION_POOL_ITEMS} Question Pool Item Question IDs`,
    );
  return entries;
}

/** Strict request decoder for the Questions-owned title and ordered content slice. */
export function decodeAssignmentContentInput(
  value: unknown,
  path = "response",
): AssignmentContentInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["title", "entries"]);
  return {
    title: decodeAssignmentTitle(field(record, "title", path), `${path}.title`),
    entries: decodeAssignmentContentEntries(field(record, "entries", path), `${path}.entries`),
  };
}

function decodeQuestionPoolItem(value: unknown, path: string): QuestionPoolItem {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "id",
    "questionId",
    "questionTitle",
    "backend",
    "capabilities",
    "availability",
  ]);
  return {
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    questionId: decodeQuestionId(field(record, "questionId", path), `${path}.questionId`),
    questionTitle: decodeQuestionTitle(
      field(record, "questionTitle", path),
      `${path}.questionTitle`,
    ),
    backend: decodeStringEnum(field(record, "backend", path), `${path}.backend`, [
      "ple",
      "webwork",
      "imathas",
    ]),
    capabilities: decodeQuestionBackendCapabilities(
      field(record, "capabilities", path),
      `${path}.capabilities`,
    ),
    availability: decodeStringEnum(field(record, "availability", path), `${path}.availability`, [
      "available",
      "retired",
    ] as const satisfies ReadonlyArray<QuestionPoolItemAvailability>),
  };
}

function decodeQuestionPoolSelectionRule(value: unknown, path: string): QuestionPoolSelectionRule {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["selectedQuestionOrder"]);
  return {
    selectedQuestionOrder: decodeStringEnum(
      field(record, "selectedQuestionOrder", path),
      `${path}.selectedQuestionOrder`,
      [
        "questionPoolOrder",
        "randomOrder",
      ] as const satisfies ReadonlyArray<QuestionPoolSelectedQuestionOrder>,
    ),
  };
}

function decodeQuestionPoolAssignmentEntry(
  value: unknown,
  path: string,
): QuestionPoolAssignmentEntry {
  const record = decodeRecord(value, path);
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
  return {
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
    selectionCount: decodePositiveInteger(
      field(record, "selectionCount", path),
      `${path}.selectionCount`,
    ),
    pointsPerItem: decodeAssignmentPointValue(
      field(record, "pointsPerItem", path),
      `${path}.pointsPerItem`,
    ),
    selectionRule: decodeQuestionPoolSelectionRule(
      field(record, "selectionRule", path),
      `${path}.selectionRule`,
    ),
    questionAttemptLimit: decodeQuestionAttemptLimit(
      field(record, "questionAttemptLimit", path),
      `${path}.questionAttemptLimit`,
    ),
    questionAttemptTimeLimit: decodeQuestionAttemptTimeLimit(
      field(record, "questionAttemptTimeLimit", path),
      `${path}.questionAttemptTimeLimit`,
    ),
    items: decodeArray(field(record, "items", path), `${path}.items`, decodeQuestionPoolItem),
  };
}

export function decodeAssignmentEntry(value: unknown, path: string): AssignmentEntrySummary {
  const record = decodeRecord(value, path);
  const kind = decodeStringEnum(field(record, "kind", path), `${path}.kind`, [
    "fixedQuestion",
    "questionPool",
  ] as const);
  if (kind === "fixedQuestion") {
    return { kind, ...decodeFixedQuestionAssignmentEntry(value, path) };
  }
  return { kind, ...decodeQuestionPoolAssignmentEntry(value, path) };
}

export function decodeAssignmentSummary(
  value: unknown,
  path = "response",
  strict = false,
): AssignmentSummary {
  const record = decodeRecord(value, path);
  if (strict) {
    requireOnlyFields(record, path, [
      "id",
      "reference",
      "courseId",
      "title",
      "entries",
      "studentFeedbackReleaseRule",
      "policies",
    ]);
  }
  const decoded = {
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    reference: decodeAssignmentReference(field(record, "reference", path), `${path}.reference`),
    courseId: decodeIdentifier(field(record, "courseId", path), `${path}.courseId`),
    title: decodeNonemptyString(field(record, "title", path), `${path}.title`),
    entries: decodeArray(field(record, "entries", path), `${path}.entries`, decodeAssignmentEntry),
    studentFeedbackReleaseRule: decodeStudentFeedbackReleaseRule(
      field(record, "studentFeedbackReleaseRule", path),
      `${path}.studentFeedbackReleaseRule`,
    ),
    policies: decodeAssignmentActivityRules(field(record, "policies", path), `${path}.policies`),
  } satisfies AssignmentSummary;
  return decoded;
}

/** Decode the student transport, which deliberately excludes authority inputs. */
export {
  decodeAssignmentAuthoredContentValidationFailure,
  decodeInstructorAssignmentAuthoredContentLocal,
  decodeStudentAssignmentDetail,
  decodeStudentAssignmentLandingSummary,
} from "./assignment_teaching_delivery";
export * from "./question_model";
