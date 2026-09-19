// Question Library, course, and assessment browser-visible API DTOs.
import { decodeCourseClassification } from "./course_classification";
import { decodeBloomClassificationView } from "./bloom_classification";
import { decodeQuestionResponsePreview } from "./question_response_preview";

import type { AssessmentEntryAvailability } from "../../../generated/api/AssessmentEntryAvailability";
import type { FixedQuestionAssessmentEntrySummary as FixedQuestionAssessmentEntry } from "../../../generated/api/FixedQuestionAssessmentEntrySummary";
import type { AssessmentEntrySummary } from "../../../generated/api/AssessmentEntrySummary";
import type { AssessmentEntryScoringRule } from "../../../generated/api/AssessmentEntryScoringRule";
import type { QuestionPoolAssessmentEntrySummary as QuestionPoolAssessmentEntry } from "../../../generated/api/QuestionPoolAssessmentEntrySummary";

import type { AssessmentSummary } from "../../../generated/api/AssessmentSummary";
import type { QuestionSearchResult } from "../../../generated/api/QuestionSearchResult";
import type { CourseQuestionUse } from "../../../generated/api/CourseQuestionUse";
import type { QuestionDetails } from "../../../generated/api/QuestionDetails";
import type { QuestionSummary } from "../../../generated/api/QuestionSummary";
import type { QuestionDetailsPromptView } from "../../../generated/api/QuestionDetailsPromptView";
import type { QuestionSearchPage } from "../../../generated/api/QuestionSearchPage";
import type { QuestionUseDetails } from "../../../generated/api/QuestionUseDetails";
import type { QuestionUseSummary } from "../../../generated/api/QuestionUseSummary";
import type { CourseSummary } from "../../../generated/api/CourseSummary";
import { decodeQuestionAuthorship } from "../question_authorship";
import type { AssessmentPointValue } from "../../../generated/api/AssessmentPointValue";
import type { AssessmentActivityRules } from "../../../generated/api/AssessmentActivityRules";
import type { QuestionPoolSelectedQuestionOrder } from "../../../generated/api/QuestionPoolSelectedQuestionOrder";
import type { QuestionPoolSelectionRule } from "../../../generated/api/QuestionPoolSelectionRule";
import type { AssessmentContentInput, AssessmentEditorEntryInput } from "../contracts";
import {
  DecodeError,
  decodeArray,
  decodeBoolean,
  decodeNonemptyString,
  decodeNonnegativeInteger,
  decodeNullable,
  decodePositiveInteger,
  decodeRecord,
  decodeString,
  decodeStringEnum,
} from "../decoder";
import { MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY } from "../../../generated/api/MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY";
import { MAX_QUESTION_SEARCH_CURSOR_ENCODED_BYTES } from "../../../generated/api/MAX_QUESTION_SEARCH_CURSOR_ENCODED_BYTES";
import { MAX_ASSESSMENT_ORDERED_ENTRIES } from "../../../generated/api/MAX_ASSESSMENT_ORDERED_ENTRIES";
import { MAX_ASSESSMENT_QUESTION_POOL_ITEMS } from "../../../generated/api/MAX_ASSESSMENT_QUESTION_POOL_ITEMS";
import { MAX_QUESTION_SEARCH_OWN_COURSE_USAGES } from "../../../generated/api/MAX_QUESTION_SEARCH_OWN_COURSE_USAGES";
import {
  MAX_QUESTION_SEARCH_PAGE_ITEMS,
  decodeAssessmentId,
  decodeAssessmentTitle,
  decodeCourseName,
  decodeQuestionBackendCapabilities,
  decodeBoundedArray,
  decodeQuestionRevisionTuple,
  decodeQuestionAvailability,
  decodeQuestionTitle,
  decodeCourseInstanceId,
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
import { decodeStudentFeedbackReleaseRule } from "./assessment_policy";
import { decodeQuestionSearchFacets } from "./question_type_facets";
import { decodeQuestionStatistics } from "./question_statistics";

// Reuse the Question Library course import surface while course-term owns its decoding rules.
export { decodeQuestionStatistics };
export { decodeCourseTerm } from "./course_term";
export { decodeStudentFeedbackReleaseRule } from "./assessment_policy";
export {
  decodeCourseAppearanceView,
  decodeCourseThemeUpdate,
  decodeCourseBannerUpdate,
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
      "questionRevision",
      "backend",
      "questionFormat",
      "questionType",
      "capabilities",
      "metadata",
      "availability",
      "publishedAt",
      "authorship",
      "bloom",
    ]);
  }
  const decoded = {
    questionId: decodeQuestionId(field(record, "questionId", path), `${path}.questionId`),
    questionRevision: decodeQuestionRevisionTuple(
      field(record, "questionRevision", path),
      `${path}.questionRevision`,
      strict,
    ),
    backend: decodeStringEnum(field(record, "backend", path), `${path}.backend`, [
      "ple",
      "webwork",
      "imathas",
    ]),
    questionFormat: decodeStringEnum(
      field(record, "questionFormat", path),
      `${path}.questionFormat`,
      ["pleQuestionJson", "webworkPg", "webworkPgml", "imathas"],
    ),
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
    availability: decodeQuestionAvailability(
      field(record, "availability", path),
      `${path}.availability`,
      strict,
    ),
    publishedAt: decodeTimestamp(field(record, "publishedAt", path), `${path}.publishedAt`),
    // Bloom Classification is blank until the deferred AI daemon assigns it.
    bloom: decodeNullable(
      field(record, "bloom", path),
      `${path}.bloom`,
      decodeBloomClassificationView,
    ),
  } satisfies QuestionSummary;
  if (decoded.questionRevision.questionId !== decoded.questionId) {
    throw new DecodeError(`${path}.questionRevision.questionId`, "the Question Summary questionId");
  }
  if (
    (decoded.backend === "ple" && decoded.questionFormat !== "pleQuestionJson") ||
    (decoded.backend === "webwork" &&
      decoded.questionFormat !== "webworkPg" &&
      decoded.questionFormat !== "webworkPgml") ||
    (decoded.backend === "imathas" && decoded.questionFormat !== "imathas")
  ) {
    throw new DecodeError(`${path}.questionFormat`, "the Question Backend's published format");
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

export function decodeQuestionSearchResult(value: unknown, path: string): QuestionSearchResult {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["summary", "disciplineName", "disciplineIsRetired", "evidence"]);
  return {
    summary: decodeQuestionSummary(field(record, "summary", path), `${path}.summary`, true),
    disciplineName: decodeNonemptyString(
      field(record, "disciplineName", path),
      `${path}.disciplineName`,
    ),
    disciplineIsRetired: decodeBoolean(
      field(record, "disciplineIsRetired", path),
      `${path}.disciplineIsRetired`,
    ),
    evidence: decodeQuestionStatistics(field(record, "evidence", path), `${path}.evidence`),
  };
}

function decodeQuestionUseSummary(value: unknown, path: string): QuestionUseSummary {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "globalCourseCount",
    "globalAssessmentCount",
    "ownCourseCount",
    "ownAssessmentCount",
  ]);
  const globalCourseCount = decodeNonnegativeInteger(
    field(record, "globalCourseCount", path),
    `${path}.globalCourseCount`,
  );
  const globalAssessmentCount = decodeNonnegativeInteger(
    field(record, "globalAssessmentCount", path),
    `${path}.globalAssessmentCount`,
  );
  const ownCourseCount = decodeNonnegativeInteger(
    field(record, "ownCourseCount", path),
    `${path}.ownCourseCount`,
  );
  const ownAssessmentCount = decodeNonnegativeInteger(
    field(record, "ownAssessmentCount", path),
    `${path}.ownAssessmentCount`,
  );
  if (ownCourseCount > globalCourseCount || ownAssessmentCount > globalAssessmentCount) {
    throw new DecodeError(path, "usage counts within their installation-wide totals");
  }
  if (globalAssessmentCount < globalCourseCount || ownAssessmentCount < ownCourseCount) {
    throw new DecodeError(path, "assessment counts at least as large as their course counts");
  }
  return {
    globalCourseCount,
    globalAssessmentCount,
    ownCourseCount,
    ownAssessmentCount,
  };
}

function decodeCourseQuestionUse(value: unknown, path: string): CourseQuestionUse {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["course", "title", "assessmentCount"]);
  return {
    course: decodeCourseInstanceId(field(record, "course", path), `${path}.course`),
    title: decodeCourseName(field(record, "title", path), `${path}.title`),
    assessmentCount: decodePositiveInteger(
      field(record, "assessmentCount", path),
      `${path}.assessmentCount`,
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
      decodeQuestionSearchCursor,
    ),
    facets: decodeQuestionSearchFacets(field(record, "facets", path), `${path}.facets`),
  };
}

/** Decodes the title-and-ID keyset cursor specific to Question Library search. */
function decodeQuestionSearchCursor(value: unknown, path: string): string {
  const cursor = decodeNonemptyString(value, path);
  if (cursor.length > MAX_QUESTION_SEARCH_CURSOR_ENCODED_BYTES) {
    throw new DecodeError(
      path,
      `a Question Library cursor no longer than ${MAX_QUESTION_SEARCH_CURSOR_ENCODED_BYTES} characters`,
    );
  }
  return cursor;
}

/** Strict safe immutable Question Details View; source and grading fields are rejected. */
export function decodeQuestionDetails(value: unknown, path = "response"): QuestionDetails {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "summary",
    "disciplineName",
    "subjectName",
    "disciplineIsRetired",
    "prompt",
    "responsePreview",
    "evidence",
    "usage",
  ]);
  const summary = decodeQuestionSummary(field(record, "summary", path), `${path}.summary`, true);
  const responsePreview = decodeNullable(
    field(record, "responsePreview", path),
    `${path}.responsePreview`,
    decodeQuestionResponsePreview,
  );
  if ((summary.backend === "ple") !== (responsePreview !== null)) {
    throw new DecodeError(`${path}.responsePreview`, "native controls only for a PLE Question");
  }
  return {
    summary,
    disciplineName: decodeNonemptyString(
      field(record, "disciplineName", path),
      `${path}.disciplineName`,
    ),
    subjectName: decodeNonemptyString(field(record, "subjectName", path), `${path}.subjectName`),
    disciplineIsRetired: decodeBoolean(
      field(record, "disciplineIsRetired", path),
      `${path}.disciplineIsRetired`,
    ),
    prompt: decodeQuestionDetailsPromptView(field(record, "prompt", path), `${path}.prompt`),
    responsePreview,
    evidence: decodeQuestionStatistics(field(record, "evidence", path), `${path}.evidence`),
    usage: decodeQuestionUseDetails(field(record, "usage", path), `${path}.usage`),
  };
}

function decodeAssessmentActivityRules(
  value: unknown,
  path: string,
  strict = false,
): AssessmentActivityRules {
  const record = decodeRecord(value, path);
  if (strict) {
    requireOnlyFields(record, path, ["questionVariationRule", "assessmentQuestionOrderRule"]);
  }
  const decoded = {
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
  } satisfies AssessmentActivityRules;
  return decoded;
}

export function decodeCourseSummary(value: unknown, path = "response"): CourseSummary {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "id",
    "shortName",
    "longName",
    "term",
    "role",
    "classification",
  ]);
  const decoded = {
    classification: decodeCourseClassification(
      field(record, "classification", path),
      `${path}.classification`,
    ),
    id: decodeCourseInstanceId(field(record, "id", path), `${path}.id`),
    shortName: decodeCourseName(field(record, "shortName", path), `${path}.shortName`),
    longName: decodeCourseName(field(record, "longName", path), `${path}.longName`),
    term: decodeCourseTerm(field(record, "term", path), `${path}.term`),
    role: decodeStringEnum(field(record, "role", path), `${path}.role`, ["student", "instructor"]),
  } satisfies CourseSummary;
  return decoded;
}

function decodeAssessmentPointValue(value: unknown, path: string): AssessmentPointValue {
  const decoded = decodeString(value, path);
  if (!/^(?:0|[1-9][0-9]{0,9})(?:\.[0-9]{1,4})?$/u.test(decoded)) {
    throw new DecodeError(path, "a canonical nonnegative decimal with at most four places");
  }
  const [whole = "0"] = decoded.split(".", 1);
  if (BigInt(whole) > 1_000_000_000n) {
    throw new DecodeError(path, "an assessment point value in the supported range");
  }
  return decoded;
}

function decodeFixedQuestionAssessmentEntry(
  value: unknown,
  path: string,
): FixedQuestionAssessmentEntry {
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
    pointsPossible: decodeAssessmentPointValue(
      field(record, "pointsPossible", path),
      `${path}.pointsPossible`,
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
function decodeAssessmentContentEntry(value: unknown, path: string): AssessmentEditorEntryInput {
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
      pointsPossible: decodeAssessmentPointValue(
        field(record, "pointsPossible", path),
        `${path}.pointsPossible`,
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
    MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY,
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
    ] as const satisfies ReadonlyArray<AssessmentEntryAvailability>),
    scoringRule: decodeStringEnum(field(record, "scoringRule", path), `${path}.scoringRule`, [
      "normal",
      "fullCredit",
      "extraCredit",
      "excluded",
    ] as const satisfies ReadonlyArray<AssessmentEntryScoringRule>),
    selectionCount,
    pointsPerItem: decodeAssessmentPointValue(
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

function decodeAssessmentContentEntries(
  value: unknown,
  path: string,
): ReadonlyArray<AssessmentEditorEntryInput> {
  const entries = decodeBoundedArray(
    value,
    path,
    MAX_ASSESSMENT_ORDERED_ENTRIES,
    decodeAssessmentContentEntry,
  );
  const questionPoolItemCount = entries.reduce(
    (total, entry) => total + (entry.kind === "questionPool" ? entry.questionIds.length : 0),
    0,
  );
  if (questionPoolItemCount > MAX_ASSESSMENT_QUESTION_POOL_ITEMS)
    throw new DecodeError(
      path,
      `no more than ${MAX_ASSESSMENT_QUESTION_POOL_ITEMS} Question Pool Item Question IDs`,
    );
  return entries;
}

/** Strict request decoder for the Questions-owned title and ordered content slice. */
export function decodeAssessmentContentInput(
  value: unknown,
  path = "response",
): AssessmentContentInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["title", "entries"]);
  return {
    title: decodeAssessmentTitle(field(record, "title", path), `${path}.title`),
    entries: decodeAssessmentContentEntries(field(record, "entries", path), `${path}.entries`),
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

function decodeQuestionPoolAssessmentEntry(
  value: unknown,
  path: string,
): QuestionPoolAssessmentEntry {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "kind",
    "id",
    "questionPoolId",
    "questionPoolEditNumber",
    "availability",
    "scoringRule",
    "selectionCount",
    "pointsPerItem",
    "selectionRule",
    "questionAttemptLimit",
    "questionAttemptTimeLimit",
  ]);
  return {
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    questionPoolId: decodeQuestionId(
      field(record, "questionPoolId", path),
      `${path}.questionPoolId`,
    ),
    questionPoolEditNumber: decodePositiveInteger(
      field(record, "questionPoolEditNumber", path),
      `${path}.questionPoolEditNumber`,
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
    selectionCount: decodePositiveInteger(
      field(record, "selectionCount", path),
      `${path}.selectionCount`,
    ),
    pointsPerItem: decodeAssessmentPointValue(
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

export function decodeAssessmentEntry(value: unknown, path: string): AssessmentEntrySummary {
  const record = decodeRecord(value, path);
  const kind = decodeStringEnum(field(record, "kind", path), `${path}.kind`, [
    "fixedQuestion",
    "questionPool",
  ] as const);
  if (kind === "fixedQuestion") {
    return { kind, ...decodeFixedQuestionAssessmentEntry(value, path) };
  }
  return { kind, ...decodeQuestionPoolAssessmentEntry(value, path) };
}

export function decodeAssessmentSummary(
  value: unknown,
  path = "response",
  strict = false,
): AssessmentSummary {
  const record = decodeRecord(value, path);
  if (strict) {
    requireOnlyFields(record, path, [
      "id",
      "courseId",
      "title",
      "entries",
      "studentFeedbackReleaseRule",
      "policies",
    ]);
  }
  const decoded = {
    id: decodeAssessmentId(field(record, "id", path), `${path}.id`),
    courseId: decodeCourseInstanceId(field(record, "courseId", path), `${path}.courseId`),
    title: decodeNonemptyString(field(record, "title", path), `${path}.title`),
    entries: decodeArray(field(record, "entries", path), `${path}.entries`, decodeAssessmentEntry),
    studentFeedbackReleaseRule: decodeStudentFeedbackReleaseRule(
      field(record, "studentFeedbackReleaseRule", path),
      `${path}.studentFeedbackReleaseRule`,
    ),
    policies: decodeAssessmentActivityRules(
      field(record, "policies", path),
      `${path}.policies`,
      strict,
    ),
  } satisfies AssessmentSummary;
  return decoded;
}

/** Decode the student transport, which deliberately excludes authority inputs. */
export {
  decodeAssessmentAuthoredContentValidationFailure,
  decodeInstructorAssessmentAuthoredContentLocal,
  decodeStudentAssessmentDetail,
  decodeStudentAssessmentLandingSummary,
} from "./assessment_teaching_delivery";
export * from "./question_model";
