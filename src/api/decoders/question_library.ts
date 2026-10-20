// Question Library and course browser-visible API DTOs.
import { decodeCourseClassification } from "./course_classification";
import { decodeBloomClassificationView } from "./bloom_classification";
import { decodeQuestionResponsePreview } from "./question_response_preview";

import type { QuestionSearchResult } from "../../../generated/api/QuestionSearchResult";
import type { LibraryObjectKindResponse } from "../../../generated/api/LibraryObjectKindResponse";
import type { LibrarySearchResult } from "../../../generated/api/LibrarySearchResult";
import type { LibrarySearchClassification } from "../../../generated/api/LibrarySearchClassification";
import type { CourseQuestionUse } from "../../../generated/api/CourseQuestionUse";
import type { QuestionDetails } from "../../../generated/api/QuestionDetails";
import type { QuestionSummary } from "../../../generated/api/QuestionSummary";
import type { QuestionDetailsPromptView } from "../../../generated/api/QuestionDetailsPromptView";
import type { LibraryObjectSearchPage } from "../../../generated/api/LibraryObjectSearchPage";
import type { LibraryFacetChoicePage } from "../../../generated/api/LibraryFacetChoicePage";
import type { LibraryFacetChoice } from "../../../generated/api/LibraryFacetChoice";
import type { QuestionUseDetails } from "../../../generated/api/QuestionUseDetails";
import type { QuestionUseSummary } from "../../../generated/api/QuestionUseSummary";
import type { CourseSummary } from "../../../generated/api/CourseSummary";
import { decodeQuestionAuthorship } from "../question_authorship";
import {
  DecodeError,
  decodeArray,
  decodeBoolean,
  decodeNonemptyString,
  decodeNonnegativeInteger,
  decodeNullable,
  decodePositiveInteger,
  decodeRecord,
  decodeStringEnum,
  decodeUuid,
} from "../decoder";
import { MAX_QUESTION_SEARCH_CURSOR_ENCODED_BYTES } from "../../../generated/api/MAX_QUESTION_SEARCH_CURSOR_ENCODED_BYTES";
import { MAX_QUESTION_SEARCH_TAG_FILTERS } from "../../../generated/api/MAX_QUESTION_SEARCH_TAG_FILTERS";
import {
  MAX_QUESTION_SEARCH_PAGE_ITEMS,
  MAX_QUESTION_DETAILS_STATIC_PROMPT_BLOCKS,
  decodeCourseName,
  decodeQuestionBackendCapabilities,
  decodeBoundedArray,
  decodePublishedQuestionRevisionTuple,
  decodeQuestionAvailability,
  decodeAccountId,
  decodeCourseInstanceId,
  decodeQuestionMetadata,
  decodeQuestionId,
  decodeTimestamp,
  field,
  kind,
  requireOnlyFields,
} from "./shared";
import { decodeCourseTerm } from "./course_term";
import { decodeQuestionContentBlock } from "./question_model";
import { decodeQuestionSearchFacets } from "./question_type_facets";
import { decodeQuestionStatistics } from "./question_statistics";
import { decodeQuestionNativePrompt } from "./question_native_prompt";
import { decodeQuestionPoolLibrarySummary } from "./question_pool_summary";

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
      "publishedQuestionRevisionTuple",
      "parentPublishedQuestionRevisionTuple",
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
    publishedQuestionRevisionTuple: decodePublishedQuestionRevisionTuple(
      field(record, "publishedQuestionRevisionTuple", path),
      `${path}.publishedQuestionRevisionTuple`,
      strict,
    ),
    parentPublishedQuestionRevisionTuple: decodeNullable(
      field(record, "parentPublishedQuestionRevisionTuple", path),
      `${path}.parentPublishedQuestionRevisionTuple`,
      (parent, parentPath) => decodePublishedQuestionRevisionTuple(parent, parentPath, strict),
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
    // Bloom Classification can remain blank when automatic assignment does not complete.
    bloom: decodeNullable(
      field(record, "bloom", path),
      `${path}.bloom`,
      decodeBloomClassificationView,
    ),
  } satisfies QuestionSummary;
  if (decoded.publishedQuestionRevisionTuple.publishedQuestionId !== decoded.questionId) {
    throw new DecodeError(
      `${path}.publishedQuestionRevisionTuple.publishedQuestionId`,
      "the Question Summary questionId",
    );
  }
  if (decoded.parentPublishedQuestionRevisionTuple?.publishedQuestionId === decoded.questionId) {
    throw new DecodeError(
      `${path}.parentPublishedQuestionRevisionTuple.publishedQuestionId`,
      "a distinct immediate parent Question ID",
    );
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

function decodeQuestionThumbnail(
  value: unknown,
  path: string,
  summaryRevision: QuestionSummary["publishedQuestionRevisionTuple"],
): QuestionSearchResult["questionThumbnail"] {
  const thumbnail = decodeNullable(value, path, (tuple, tuplePath) =>
    decodePublishedQuestionRevisionTuple(tuple, tuplePath, true),
  );
  if (
    thumbnail !== null &&
    (thumbnail.publishedQuestionId !== summaryRevision.publishedQuestionId ||
      thumbnail.revisionNumber !== summaryRevision.revisionNumber)
  ) {
    throw new DecodeError(path, "the exact published revision in the Question Summary");
  }
  return thumbnail;
}

export function decodeQuestionSearchResult(value: unknown, path: string): QuestionSearchResult {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "summary",
    "questionThumbnail",
    "nativePrompt",
    "disciplineName",
    "disciplineIsRetired",
    "evidence",
  ]);
  const summary = decodeQuestionSummary(field(record, "summary", path), `${path}.summary`, true);
  const nativePrompt = decodeQuestionNativePrompt(
    field(record, "nativePrompt", path),
    `${path}.nativePrompt`,
  );
  if ((summary.backend === "ple") !== (nativePrompt !== null)) {
    throw new DecodeError(`${path}.nativePrompt`, "Native prompt content only for a PLE Question");
  }
  return {
    summary,
    nativePrompt,
    questionThumbnail: decodeQuestionThumbnail(
      field(record, "questionThumbnail", path),
      `${path}.questionThumbnail`,
      summary.publishedQuestionRevisionTuple,
    ),
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

/** Strictly decodes one discriminated mixed Library page item. */
export function decodeLibrarySearchClassification(
  value: unknown,
  path: string,
): LibrarySearchClassification {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "disciplineUuid",
    "disciplineName",
    "subjectUuid",
    "subjectName",
    "topicUuid",
    "topicName",
    "subtopicUuid",
    "subtopicName",
  ]);
  const topicUuid = field(record, "topicUuid", path);
  const topicName = field(record, "topicName", path);
  const subtopicUuid = field(record, "subtopicUuid", path);
  const subtopicName = field(record, "subtopicName", path);
  if (
    (topicUuid === null) !== (topicName === null) ||
    (subtopicUuid === null) !== (subtopicName === null)
  ) {
    throw new DecodeError(path, "matching optional classification UUID and name pairs");
  }
  if (subtopicUuid !== null && topicUuid === null) {
    throw new DecodeError(path, "a Topic for the selected Subtopic");
  }
  return {
    disciplineUuid: decodeUuid(field(record, "disciplineUuid", path), `${path}.disciplineUuid`),
    disciplineName: decodeNonemptyString(
      field(record, "disciplineName", path),
      `${path}.disciplineName`,
    ),
    subjectUuid: decodeUuid(field(record, "subjectUuid", path), `${path}.subjectUuid`),
    subjectName: decodeNonemptyString(field(record, "subjectName", path), `${path}.subjectName`),
    topicUuid: topicUuid === null ? null : decodeUuid(topicUuid, `${path}.topicUuid`),
    topicName: topicName === null ? null : decodeNonemptyString(topicName, `${path}.topicName`),
    subtopicUuid: subtopicUuid === null ? null : decodeUuid(subtopicUuid, `${path}.subtopicUuid`),
    subtopicName:
      subtopicName === null ? null : decodeNonemptyString(subtopicName, `${path}.subtopicName`),
  };
}

export function decodeLibrarySearchResult(value: unknown, path: string): LibrarySearchResult {
  const record = decodeRecord(value, path);
  const resultKind = kind(record, path);
  if (resultKind === "question") {
    requireOnlyFields(record, path, ["kind", "question", "ownerAccountId", "classification"]);
    return {
      kind: "question",
      question: decodeQuestionSearchResult(field(record, "question", path), `${path}.question`),
      ownerAccountId: decodeAccountId(
        field(record, "ownerAccountId", path),
        `${path}.ownerAccountId`,
      ),
      classification: decodeLibrarySearchClassification(
        field(record, "classification", path),
        `${path}.classification`,
      ),
    };
  }
  if (resultKind === "pool") {
    requireOnlyFields(record, path, ["kind", "pool", "classification"]);
    return {
      kind: "pool",
      pool: decodeQuestionPoolLibrarySummary(field(record, "pool", path), `${path}.pool`),
      classification: decodeLibrarySearchClassification(
        field(record, "classification", path),
        `${path}.classification`,
      ),
    };
  }
  throw new DecodeError(`${path}.kind`, "one of question, pool");
}

/** Strictly decodes the authorized kind hint used by the Library detail route. */
export function decodeLibraryObjectKindResponse(
  value: unknown,
  path = "response",
): LibraryObjectKindResponse {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["kind"]);
  return {
    kind: decodeStringEnum(field(record, "kind", path), `${path}.kind`, [
      "question",
      "questionPool",
    ] as const),
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
  requireOnlyFields(record, path, ["courseInstanceId", "title", "assessmentCount"]);
  return {
    courseInstanceId: decodeCourseInstanceId(
      field(record, "courseInstanceId", path),
      `${path}.courseInstanceId`,
    ),
    title: decodeCourseName(field(record, "title", path), `${path}.title`),
    assessmentCount: decodePositiveInteger(
      field(record, "assessmentCount", path),
      `${path}.assessmentCount`,
    ),
  };
}

function decodeQuestionUseDetails(value: unknown, path: string): QuestionUseDetails {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["summary", "ownCourses"]);
  const summary = decodeQuestionUseSummary(field(record, "summary", path), `${path}.summary`);
  const ownCourses = decodeArray(
    field(record, "ownCourses", path),
    `${path}.ownCourses`,
    decodeCourseQuestionUse,
  );
  const seenCourses = new Set<string>();
  for (const courseUsage of ownCourses) {
    if (seenCourses.has(courseUsage.courseInstanceId)) {
      throw new DecodeError(`${path}.ownCourses`, "unique course IDs");
    }
    seenCourses.add(courseUsage.courseInstanceId);
  }
  if (ownCourses.length !== summary.ownCourseCount) {
    throw new DecodeError(`${path}.ownCourses`, "a complete list matching ownCourseCount");
  }
  return {
    summary,
    ownCourses,
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
      MAX_QUESTION_DETAILS_STATIC_PROMPT_BLOCKS,
      (block, blockPath) => decodeQuestionContentBlock(block, blockPath, true),
    ),
  };
}
/** Strict, bounded metadata-only Question Search Results View. */
export function decodeLibraryObjectSearchPage(
  value: unknown,
  path = "response",
): LibraryObjectSearchPage {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["items", "pageNumber", "pageSize", "totalCount", "facets"]);
  const pageNumber = decodePositiveInteger(field(record, "pageNumber", path), `${path}.pageNumber`);
  const pageSize = decodePositiveInteger(field(record, "pageSize", path), `${path}.pageSize`);
  const totalCount = decodeNonnegativeInteger(
    field(record, "totalCount", path),
    `${path}.totalCount`,
  );
  if (
    pageNumber > 0xffffffff ||
    ![50, 100, 250].includes(pageSize) ||
    pageNumber > Math.max(1, Math.ceil(totalCount / pageSize))
  ) {
    throw new DecodeError(path, "a valid resolved numbered Question Library page");
  }
  return {
    items: decodeBoundedArray(
      field(record, "items", path),
      `${path}.items`,
      MAX_QUESTION_SEARCH_PAGE_ITEMS,
      decodeLibrarySearchResult,
    ),
    pageNumber,
    pageSize,
    totalCount,
    facets: decodeQuestionSearchFacets(field(record, "facets", path), `${path}.facets`),
  };
}

function decodeLibraryFacetChoice(value: unknown, path: string): LibraryFacetChoice {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["key", "label", "count"]);
  return {
    key: decodeNonemptyString(field(record, "key", path), `${path}.key`),
    label: decodeNonemptyString(field(record, "label", path), `${path}.label`),
    count: decodeNonnegativeInteger(field(record, "count", path), `${path}.count`),
  };
}

export function decodeLibraryFacetChoicePage(
  value: unknown,
  path = "response",
): LibraryFacetChoicePage {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["choices", "selected", "nextCursor"]);
  return {
    choices: decodeBoundedArray(
      field(record, "choices", path),
      `${path}.choices`,
      100,
      decodeLibraryFacetChoice,
    ),
    selected: decodeBoundedArray(
      field(record, "selected", path),
      `${path}.selected`,
      MAX_QUESTION_SEARCH_TAG_FILTERS,
      decodeLibraryFacetChoice,
    ),
    nextCursor: decodeNullable(
      field(record, "nextCursor", path),
      `${path}.nextCursor`,
      decodeQuestionSearchCursor,
    ),
  };
}

/** Decodes the continuation cursor specific to facet-choice enumeration. */
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

export function decodeCourseSummary(value: unknown, path = "response"): CourseSummary {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "id",
    "shortName",
    "longName",
    "term",
    "role",
    "classification",
    "lifecycleState",
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
    lifecycleState: decodeStringEnum(
      field(record, "lifecycleState", path),
      `${path}.lifecycleState`,
      ["active", "inactive"] as const,
    ),
  } satisfies CourseSummary;
  return decoded;
}

/** Decode the student transport, which deliberately excludes authority inputs. */
export {
  decodeAssessmentAuthoredContentValidationFailure,
  decodeInstructorAssessmentAuthoredContentLocal,
  decodeStudentAssessmentDetail,
  decodeStudentAssessmentLandingSummary,
} from "./assessment_teaching_delivery";
export {
  decodeAssessmentContentInput,
  decodeAssessmentEntry,
  decodeAssessmentSummary,
} from "./assessment_content";
export * from "./question_model";
