// Catalog, course, and assignment browser-visible API DTOs.

import type { AssignmentDeliveryState } from "../../../generated/api/AssignmentDeliveryState";
import type { AssignmentItemSummary as AssignmentItem } from "../../../generated/api/AssignmentItemSummary";
import type { AssignmentScoringMode } from "../../../generated/api/AssignmentScoringMode";
import type { AssignmentSelectionCandidateSummary as AssignmentSelectionCandidate } from "../../../generated/api/AssignmentSelectionCandidateSummary";
import type { AssignmentSelectionGroupSummary as AssignmentSelectionGroup } from "../../../generated/api/AssignmentSelectionGroupSummary";
import type { AssignmentSummary } from "../../../generated/api/AssignmentSummary";
import type { CatalogDiscoveryEvidence } from "../../../generated/api/CatalogDiscoveryEvidence";
import type { CatalogDiscoveryItem } from "../../../generated/api/CatalogDiscoveryItem";
import type { CatalogOwnCourseUsage } from "../../../generated/api/CatalogOwnCourseUsage";
import type { CatalogProblemDetail } from "../../../generated/api/CatalogProblemDetail";
import type { CatalogProblemSummary } from "../../../generated/api/CatalogProblemSummary";
import type { CatalogPromptProjection } from "../../../generated/api/CatalogPromptProjection";
import type { CatalogSearchPage } from "../../../generated/api/CatalogSearchPage";
import type { CatalogUsageDetail } from "../../../generated/api/CatalogUsageDetail";
import type { CatalogUsageSummary } from "../../../generated/api/CatalogUsageSummary";
import type { CompletionRequirement } from "../../../generated/api/CompletionRequirement";
import type { ContinuedPractice } from "../../../generated/api/ContinuedPractice";
import type { CourseSummary } from "../../../generated/api/CourseSummary";
import type { AssignmentRouteReference, CourseRouteReference } from "../../navigation/public_route";
import { decodePublicByline } from "../public_byline";
import { parseAssignmentReference, parseCourseReference } from "../../navigation/public_route";

function decodeCourseReference(value: unknown, path: string): CourseRouteReference {
  if (typeof value !== "string") throw new DecodeError(path, "a C- reference");
  const reference = parseCourseReference(value);
  if (reference === null) throw new DecodeError(path, "a C- reference");
  return reference;
}
export function decodeAssignmentReference(value: unknown, path: string): AssignmentRouteReference {
  if (typeof value !== "string") throw new DecodeError(path, "an A- reference");
  const reference = parseAssignmentReference(value);
  if (reference === null) throw new DecodeError(path, "an A- reference");
  return reference;
}
import type { PointValue } from "../../../generated/api/PointValue";
import type { AssignmentActivityRules } from "../../../generated/api/AssignmentActivityRules";
import type { SelectionOrdering } from "../../../generated/api/SelectionOrdering";
import type {
  AssignmentContentInput,
  AssignmentEditorEntryInput,
  CourseCreateInput,
  CourseRouteData,
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
import { MAX_ASSIGNMENT_CANDIDATES_PER_SELECTION_GROUP } from "../../../generated/api/MAX_ASSIGNMENT_CANDIDATES_PER_SELECTION_GROUP";
import { MAX_ASSIGNMENT_ORDERED_ENTRIES } from "../../../generated/api/MAX_ASSIGNMENT_ORDERED_ENTRIES";
import { MAX_ASSIGNMENT_TOTAL_SELECTION_CANDIDATES } from "../../../generated/api/MAX_ASSIGNMENT_TOTAL_SELECTION_CANDIDATES";
import { MAX_CATALOG_OWN_COURSE_USAGES } from "../../../generated/api/MAX_CATALOG_OWN_COURSE_USAGES";
import {
  MAX_CATALOG_PAGE_ITEMS,
  MINIMUM_STATISTICS_COHORT_SIZE,
  STATISTICS_DURATION_ESTIMATES_SECONDS,
  decodeAssignmentTitle,
  decodeBackendCapabilities,
  decodeBoundedArray,
  decodeQuestionVersionAvailability,
  decodeCursor,
  decodeEnvelopeTitle,
  decodeIdentifier,
  decodeQuestionMetadata,
  decodeQuestionId,
  decodeTimestamp,
  field,
  kind,
  requireOnlyFields,
} from "./shared";
import { decodeCourseTerm } from "./course_term";
import { decodeContentBlock } from "./question_model";
import { decodeStudentDisclosurePolicy } from "./assignment_policy";
import { decodeCourseAppearance } from "./course_appearance";
import { decodeCatalogSearchFacets } from "./catalog_search_facets";

// Retain the established catalog-course import surface while course-term owns its decoding rules.
export { decodeCourseTerm, decodeCourseTermValidationFailure } from "./course_term";
export { decodeStudentDisclosurePolicy } from "./assignment_policy";
export {
  decodeCourseAppearance,
  decodeCourseAppearanceUpdate,
  decodeCourseBannerCandidateReceipt,
} from "./course_appearance";

export function decodeCatalogProblemSummary(
  value: unknown,
  path = "response",
  strict = false,
): CatalogProblemSummary {
  const record = decodeRecord(value, path);
  if (strict) {
    requireOnlyFields(record, path, [
      "questionId",
      "backend",
      "responseFamily",
      "capabilities",
      "metadata",
      "availability",
      "publishedAt",
      "byline",
    ]);
  }
  const decoded = {
    questionId: decodeQuestionId(field(record, "questionId", path), `${path}.questionId`),
    backend: decodeStringEnum(field(record, "backend", path), `${path}.backend`, [
      "native",
      "webwork",
      "qti",
      "h5p",
      "imathas",
    ]),
    responseFamily: decodeStringEnum(
      field(record, "responseFamily", path),
      `${path}.responseFamily`,
      [
        "numeric",
        "multipleChoice",
        "shortText",
        "multiBlank",
        "matching",
        "ordering",
        "hotspot",
        "fileUpload",
        "externalTool",
      ],
    ),
    capabilities: decodeBackendCapabilities(
      field(record, "capabilities", path),
      `${path}.capabilities`,
    ),
    metadata: decodeQuestionMetadata(field(record, "metadata", path), `${path}.metadata`, strict),
    byline: decodePublicByline(field(record, "byline", path), `${path}.byline`),
    availability: decodeQuestionVersionAvailability(
      field(record, "availability", path),
      `${path}.availability`,
      strict,
    ),
    publishedAt: decodeTimestamp(field(record, "publishedAt", path), `${path}.publishedAt`),
  } satisfies CatalogProblemSummary;
  return decoded;
}

/**
 * Verifies the exact browser-safe success projection for a native publication.
 *
 * Decoding establishes the DTO's shape; callers of a publication command must
 * additionally bind that DTO to the published state.
 */
export function isAvailableNativeCatalogProblemSummary(
  summary: CatalogProblemSummary,
): boolean {
  return (
    summary.backend === "native" &&
    summary.availability.availability === "available"
  );
}

function decodeUnitInterval(value: unknown, path: string): number {
  const decoded = decodeFiniteNumber(value, path);
  if (decoded < 0 || decoded > 1) {
    throw new DecodeError(path, "a finite number from 0 through 1");
  }
  return decoded;
}

function decodeCorrelation(value: unknown, path: string): number {
  const decoded = decodeFiniteNumber(value, path);
  if (decoded < -1 || decoded > 1) {
    throw new DecodeError(path, "a finite correlation from -1 through 1");
  }
  return decoded;
}

function decodeCatalogDiscoveryEvidence(value: unknown, path: string): CatalogDiscoveryEvidence {
  const record = decodeRecord(value, path);
  const evidenceState = decodeStringEnum(field(record, "state", path), `${path}.state`, [
    "insufficientEvidence",
    "available",
  ]);
  if (evidenceState === "insufficientEvidence") {
    requireOnlyFields(record, path, ["state"]);
    return { state: evidenceState };
  }
  requireOnlyFields(record, path, [
    "state",
    "formulaVersion",
    "observedCourseCount",
    "independentLearnerObservationCount",
    "difficultyIndex",
    "attemptsMean",
    "timeMedianSecondsEstimate",
    "discriminationIndex",
    "evidenceAt",
  ]);
  const formulaVersion = decodePositiveInteger(
    field(record, "formulaVersion", path),
    `${path}.formulaVersion`,
  );
  if (formulaVersion > 65_535) {
    throw new DecodeError(`${path}.formulaVersion`, "a positive 16-bit formula version");
  }
  const observedCourseCount = decodePositiveInteger(
    field(record, "observedCourseCount", path),
    `${path}.observedCourseCount`,
  );
  if (observedCourseCount < 2) {
    throw new DecodeError(`${path}.observedCourseCount`, "a safe integer at least 2");
  }
  const independentLearnerObservationCount = decodeNonnegativeInteger(
    field(record, "independentLearnerObservationCount", path),
    `${path}.independentLearnerObservationCount`,
  );
  if (independentLearnerObservationCount < MINIMUM_STATISTICS_COHORT_SIZE) {
    throw new DecodeError(
      `${path}.independentLearnerObservationCount`,
      `a safe integer at least ${MINIMUM_STATISTICS_COHORT_SIZE}`,
    );
  }
  if (independentLearnerObservationCount < observedCourseCount) {
    throw new DecodeError(
      `${path}.independentLearnerObservationCount`,
      "a count at least as large as observedCourseCount",
    );
  }
  const attemptsMean = decodeFiniteNumber(
    field(record, "attemptsMean", path),
    `${path}.attemptsMean`,
  );
  if (attemptsMean < 1) {
    throw new DecodeError(`${path}.attemptsMean`, "a finite number at least 1");
  }
  const timeMedianSecondsEstimate = decodeNonnegativeInteger(
    field(record, "timeMedianSecondsEstimate", path),
    `${path}.timeMedianSecondsEstimate`,
  );
  if (
    !STATISTICS_DURATION_ESTIMATES_SECONDS.some(
      (estimate) => estimate === timeMedianSecondsEstimate,
    )
  ) {
    throw new DecodeError(
      `${path}.timeMedianSecondsEstimate`,
      "a supported fixed-histogram duration estimate",
    );
  }
  const discriminationIndex =
    "discriminationIndex" in record
      ? decodeCorrelation(field(record, "discriminationIndex", path), `${path}.discriminationIndex`)
      : undefined;
  const decoded = {
    state: evidenceState,
    formulaVersion,
    observedCourseCount,
    independentLearnerObservationCount,
    difficultyIndex: decodeUnitInterval(
      field(record, "difficultyIndex", path),
      `${path}.difficultyIndex`,
    ),
    attemptsMean,
    timeMedianSecondsEstimate,
    ...(discriminationIndex === undefined ? {} : { discriminationIndex }),
    evidenceAt: decodeNonnegativeInteger(field(record, "evidenceAt", path), `${path}.evidenceAt`),
  } satisfies CatalogDiscoveryEvidence;
  return decoded;
}

function decodeCatalogDiscoveryItem(value: unknown, path: string): CatalogDiscoveryItem {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["summary", "evidence"]);
  return {
    summary: decodeCatalogProblemSummary(field(record, "summary", path), `${path}.summary`, true),
    evidence: decodeCatalogDiscoveryEvidence(field(record, "evidence", path), `${path}.evidence`),
  };
}

function decodeCatalogUsageSummary(value: unknown, path: string): CatalogUsageSummary {
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

function decodeCatalogOwnCourseUsage(value: unknown, path: string): CatalogOwnCourseUsage {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["course", "title", "assignmentCount"]);
  return {
    course: decodeCourseReference(field(record, "course", path), `${path}.course`),
    title: decodeEnvelopeTitle(field(record, "title", path), `${path}.title`),
    assignmentCount: decodePositiveInteger(
      field(record, "assignmentCount", path),
      `${path}.assignmentCount`,
    ),
  };
}

function decodeCatalogUsageDetail(value: unknown, path: string): CatalogUsageDetail {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["summary", "ownCourses", "ownCoursesTruncated"]);
  const summary = decodeCatalogUsageSummary(field(record, "summary", path), `${path}.summary`);
  const ownCourses = decodeBoundedArray(
    field(record, "ownCourses", path),
    `${path}.ownCourses`,
    MAX_CATALOG_OWN_COURSE_USAGES,
    decodeCatalogOwnCourseUsage,
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
    (ownCourses.length !== MAX_CATALOG_OWN_COURSE_USAGES ||
      summary.ownCourseCount <= MAX_CATALOG_OWN_COURSE_USAGES)
  ) {
    throw new DecodeError(
      `${path}.ownCoursesTruncated`,
      `true only for ${MAX_CATALOG_OWN_COURSE_USAGES} listed rows with additional own courses`,
    );
  }
  return {
    summary,
    ownCourses,
    ownCoursesTruncated,
  };
}

function decodeCatalogPromptProjection(value: unknown, path: string): CatalogPromptProjection {
  const record = decodeRecord(value, path);
  const projectionKind = kind(record, path);
  if (projectionKind !== "static" && projectionKind !== "generatedExample") {
    throw new DecodeError(`${path}.kind`, "a known catalog prompt projection");
  }
  requireOnlyFields(record, path, ["kind", "blocks"]);
  return {
    kind: projectionKind,
    blocks: decodeBoundedArray(
      field(record, "blocks", path),
      `${path}.blocks`,
      MAX_CATALOG_PAGE_ITEMS,
      (block, blockPath) => decodeContentBlock(block, blockPath, true),
    ),
  };
}
/** Strict, bounded metadata-only projection used by the catalog search endpoint. */
export function decodeCatalogSearchPage(value: unknown, path = "response"): CatalogSearchPage {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["items", "nextCursor", "facets"]);
  return {
    items: decodeBoundedArray(
      field(record, "items", path),
      `${path}.items`,
      MAX_CATALOG_PAGE_ITEMS,
      decodeCatalogDiscoveryItem,
    ),
    nextCursor: decodeNullable(
      field(record, "nextCursor", path),
      `${path}.nextCursor`,
      decodeCursor,
    ),
    facets: decodeCatalogSearchFacets(field(record, "facets", path), `${path}.facets`),
  };
}

/** Strict safe immutable detail projection; source and grading fields are rejected. */
export function decodeCatalogProblemDetail(
  value: unknown,
  path = "response",
): CatalogProblemDetail {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["summary", "prompt", "evidence", "usage"]);
  return {
    summary: decodeCatalogProblemSummary(field(record, "summary", path), `${path}.summary`, true),
    prompt: decodeCatalogPromptProjection(field(record, "prompt", path), `${path}.prompt`),
    evidence: decodeCatalogDiscoveryEvidence(field(record, "evidence", path), `${path}.evidence`),
    usage: decodeCatalogUsageDetail(field(record, "usage", path), `${path}.usage`),
  };
}

function decodeCompletionRequirement(
  value: unknown,
  path: string,
  strict = false,
): CompletionRequirement {
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
      } satisfies CompletionRequirement;
      return decoded;
    }
    default:
      throw new DecodeError(`${path}.kind`, "a known completion requirement");
  }
}

function decodeContinuedPractice(value: unknown, path: string, strict = false): ContinuedPractice {
  const record = decodeRecord(value, path);
  const practice = kind(record, path);
  switch (practice) {
    case "unlimited":
    case "closed":
      if (strict) requireOnlyFields(record, path, ["kind"]);
      return { kind: practice };
    case "capped": {
      if (strict) requireOnlyFields(record, path, ["kind", "maxAdditionalRuns"]);
      const decoded = {
        kind: practice,
        maxAdditionalRuns: decodeNonnegativeInteger(
          field(record, "maxAdditionalRuns", path),
          `${path}.maxAdditionalRuns`,
        ),
      } satisfies ContinuedPractice;
      return decoded;
    }
    default:
      throw new DecodeError(`${path}.kind`, "a known continued-practice policy");
  }
}

function decodeAssignmentActivityRules(value: unknown, path: string, strict = false): AssignmentActivityRules {
  const record = decodeRecord(value, path);
  if (strict) {
    requireOnlyFields(record, path, ["completion", "grade", "continuedPractice", "variation"]);
  }
  const decoded = {
    completion: decodeCompletionRequirement(
      field(record, "completion", path),
      `${path}.completion`,
      strict,
    ),
    grade: decodeStringEnum(field(record, "grade", path), `${path}.grade`, [
      "first",
      "latest",
      "highest",
      "instructorSelected",
    ]),
    continuedPractice: decodeContinuedPractice(
      field(record, "continuedPractice", path),
      `${path}.continuedPractice`,
      strict,
    ),
    variation: decodeStringEnum(field(record, "variation", path), `${path}.variation`, [
      "newSeeds",
      "selectedProblemVariants",
      "fullRegeneration",
    ]),
  } satisfies AssignmentActivityRules;
  return decoded;
}

export function decodeCourseSummary(value: unknown, path = "response"): CourseSummary {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["id", "reference", "title", "term", "role"]);
  const decoded = {
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    reference: decodeCourseReference(field(record, "reference", path), `${path}.reference`),
    title: decodeNonemptyString(field(record, "title", path), `${path}.title`),
    term: decodeCourseTerm(field(record, "term", path), `${path}.term`),
    role: decodeStringEnum(field(record, "role", path), `${path}.role`, ["student", "instructor"]),
  } satisfies CourseSummary;
  return decoded;
}

/** Strict request decoder for the public course-creation transport boundary. */
export function decodeCourseCreateInput(value: unknown, path = "request"): CourseCreateInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["title", "term"]);
  const title = decodeNonemptyString(field(record, "title", path), `${path}.title`);
  if (title.trim().length === 0) {
    throw new DecodeError(`${path}.title`, "a course title containing non-whitespace content");
  }
  const decoded = {
    title,
    term: decodeCourseTerm(field(record, "term", path), `${path}.term`),
  } satisfies CourseCreateInput;
  return decoded;
}

export function decodeCourseRouteData(value: unknown, path: string): CourseRouteData {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["summary", "appearance"]);
  return {
    summary: decodeCourseSummary(field(record, "summary", path), `${path}.summary`),
    appearance: decodeCourseAppearance(field(record, "appearance", path), `${path}.appearance`),
  };
}

function decodePointValue(value: unknown, path: string): PointValue {
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

export function decodeAssignmentItem(value: unknown, path: string): AssignmentItem {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "id",
    "questionId",
    "title",
    "backend",
    "capabilities",
    "position",
    "pointsPossible",
    "deliveryState",
    "scoringMode",
  ]);
  return {
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    questionId: decodeQuestionId(field(record, "questionId", path), `${path}.questionId`),
    title: decodeEnvelopeTitle(field(record, "title", path), `${path}.title`),
    backend: decodeStringEnum(field(record, "backend", path), `${path}.backend`, [
      "native",
      "webwork",
      "qti",
      "h5p",
      "imathas",
    ]),
    capabilities: decodeBackendCapabilities(
      field(record, "capabilities", path),
      `${path}.capabilities`,
    ),
    position: decodeNonnegativeInteger(field(record, "position", path), `${path}.position`),
    pointsPossible: decodePointValue(
      field(record, "pointsPossible", path),
      `${path}.pointsPossible`,
    ),
    deliveryState: decodeStringEnum(field(record, "deliveryState", path), `${path}.deliveryState`, [
      "active",
      "retired",
    ] as const satisfies ReadonlyArray<AssignmentDeliveryState>),
    scoringMode: decodeStringEnum(field(record, "scoringMode", path), `${path}.scoringMode`, [
      "normal",
      "fullCredit",
      "extraCredit",
      "excluded",
    ] as const satisfies ReadonlyArray<AssignmentScoringMode>),
  };
}

/** Request-only entry shape: the server owns display metadata and all internal identities. */
function decodeAssignmentContentEntry(value: unknown, path: string): AssignmentEditorEntryInput {
  const record = decodeRecord(value, path);
  const entryKind = decodeStringEnum(field(record, "kind", path), `${path}.kind`, [
    "fixed",
    "selectionGroup",
  ] as const);
  if (entryKind === "fixed") {
    requireOnlyFields(record, path, [
      "kind",
      "questionId",
      "position",
      "pointsPossible",
      "deliveryState",
      "scoringMode",
    ]);
    return {
      kind: "fixed",
      questionId: decodeQuestionId(field(record, "questionId", path), `${path}.questionId`),
      position: decodeNonnegativeInteger(field(record, "position", path), `${path}.position`),
      pointsPossible: decodePointValue(
        field(record, "pointsPossible", path),
        `${path}.pointsPossible`,
      ),
      deliveryState: decodeStringEnum(
        field(record, "deliveryState", path),
        `${path}.deliveryState`,
        ["active", "retired"] as const satisfies ReadonlyArray<AssignmentDeliveryState>,
      ),
      scoringMode: decodeStringEnum(field(record, "scoringMode", path), `${path}.scoringMode`, [
        "normal",
        "fullCredit",
        "extraCredit",
        "excluded",
      ] as const satisfies ReadonlyArray<AssignmentScoringMode>),
    };
  }
  requireOnlyFields(record, path, [
    "kind",
    "candidateQuestionIds",
    "position",
    "drawCount",
    "pointsPerItem",
    "ordering",
  ]);
  const candidateQuestionIds = decodeBoundedArray(
    field(record, "candidateQuestionIds", path),
    `${path}.candidateQuestionIds`,
    MAX_ASSIGNMENT_CANDIDATES_PER_SELECTION_GROUP,
    decodeQuestionId,
  );
  if (new Set(candidateQuestionIds).size !== candidateQuestionIds.length)
    throw new DecodeError(`${path}.candidateQuestionIds`, "unique Question IDs");
  const drawCount = decodePositiveInteger(field(record, "drawCount", path), `${path}.drawCount`);
  if (drawCount > candidateQuestionIds.length)
    throw new DecodeError(`${path}.drawCount`, "a value no greater than the candidate count");
  return {
    kind: "selectionGroup",
    candidateQuestionIds,
    position: decodeNonnegativeInteger(field(record, "position", path), `${path}.position`),
    drawCount,
    pointsPerItem: decodePointValue(field(record, "pointsPerItem", path), `${path}.pointsPerItem`),
    ordering: decodeStringEnum(field(record, "ordering", path), `${path}.ordering`, [
      "candidateOrder",
      "randomized",
    ] as const satisfies ReadonlyArray<SelectionOrdering>),
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
  const totalCandidates = entries.reduce(
    (total, entry) =>
      total + (entry.kind === "selectionGroup" ? entry.candidateQuestionIds.length : 0),
    0,
  );
  if (totalCandidates > MAX_ASSIGNMENT_TOTAL_SELECTION_CANDIDATES)
    throw new DecodeError(
      path,
      `no more than ${MAX_ASSIGNMENT_TOTAL_SELECTION_CANDIDATES} selection-group candidate Question IDs`,
    );
  const positions = entries.map((entry) => entry.position).sort((left, right) => left - right);
  for (let position = 0; position < positions.length; position += 1) {
    if (positions[position] !== position)
      throw new DecodeError(path, "one complete entry list with positions from zero in order");
  }
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

function decodeAssignmentSelectionCandidate(
  value: unknown,
  path: string,
): AssignmentSelectionCandidate {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "id",
    "position",
    "questionId",
    "title",
    "backend",
    "capabilities",
    "deliveryState",
  ]);
  return {
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    position: decodeNonnegativeInteger(field(record, "position", path), `${path}.position`),
    questionId: decodeQuestionId(field(record, "questionId", path), `${path}.questionId`),
    title: decodeEnvelopeTitle(field(record, "title", path), `${path}.title`),
    backend: decodeStringEnum(field(record, "backend", path), `${path}.backend`, [
      "native",
      "webwork",
      "qti",
      "h5p",
      "imathas",
    ]),
    capabilities: decodeBackendCapabilities(
      field(record, "capabilities", path),
      `${path}.capabilities`,
    ),
    deliveryState: decodeStringEnum(field(record, "deliveryState", path), `${path}.deliveryState`, [
      "active",
      "retired",
    ] as const satisfies ReadonlyArray<AssignmentDeliveryState>),
  };
}

export function decodeAssignmentSelectionGroup(
  value: unknown,
  path: string,
): AssignmentSelectionGroup {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "id",
    "position",
    "drawCount",
    "pointsPerItem",
    "ordering",
    "algorithmVersion",
    "candidates",
  ]);
  return {
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    position: decodeNonnegativeInteger(field(record, "position", path), `${path}.position`),
    drawCount: decodePositiveInteger(field(record, "drawCount", path), `${path}.drawCount`),
    pointsPerItem: decodePointValue(field(record, "pointsPerItem", path), `${path}.pointsPerItem`),
    ordering: decodeStringEnum(field(record, "ordering", path), `${path}.ordering`, [
      "candidateOrder",
      "randomized",
    ] as const satisfies ReadonlyArray<SelectionOrdering>),
    algorithmVersion: decodePositiveInteger(
      field(record, "algorithmVersion", path),
      `${path}.algorithmVersion`,
    ),
    candidates: decodeArray(
      field(record, "candidates", path),
      `${path}.candidates`,
      decodeAssignmentSelectionCandidate,
    ),
  };
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
      "items",
      "selectionGroups",
      "disclosurePolicy",
      "policies",
    ]);
  }
  const decoded = {
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    reference: decodeAssignmentReference(field(record, "reference", path), `${path}.reference`),
    courseId: decodeIdentifier(field(record, "courseId", path), `${path}.courseId`),
    title: decodeNonemptyString(field(record, "title", path), `${path}.title`),
    items: decodeArray(field(record, "items", path), `${path}.items`, decodeAssignmentItem),
    selectionGroups: decodeArray(
      field(record, "selectionGroups", path),
      `${path}.selectionGroups`,
      decodeAssignmentSelectionGroup,
    ),
    disclosurePolicy: decodeStudentDisclosurePolicy(
      field(record, "disclosurePolicy", path),
      `${path}.disclosurePolicy`,
    ),
    policies: decodeAssignmentActivityRules(field(record, "policies", path), `${path}.policies`),
  } satisfies AssignmentSummary;
  return decoded;
}

/** Decode the student transport, which deliberately excludes authority inputs. */
export {
  decodeAssignmentTeachingSettingsValidationFailure,
  decodeInstructorAssignmentTeachingSettingsLocal,
  decodeStudentAssignmentDetail,
  decodeStudentAssignmentLandingSummary,
} from "./assignment_teaching_delivery";
export * from "./question_model";
