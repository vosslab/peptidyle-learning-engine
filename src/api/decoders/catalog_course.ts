// Catalog, course, and assignment browser-visible API DTOs.

import { MAX_CATALOG_TAXONOMY_FACETS } from "../../../generated/api/MAX_CATALOG_TAXONOMY_FACETS";
import type { AssignmentDeliveryState } from "../../../generated/api/AssignmentDeliveryState";
import type { AssignmentItemSummary as AssignmentItem } from "../../../generated/api/AssignmentItemSummary";
import type { AssignmentScoringMode } from "../../../generated/api/AssignmentScoringMode";
import type { AssignmentSelectionCandidateSummary as AssignmentSelectionCandidate } from "../../../generated/api/AssignmentSelectionCandidateSummary";
import type { AssignmentSelectionGroupSummary as AssignmentSelectionGroup } from "../../../generated/api/AssignmentSelectionGroupSummary";
import type { AssignmentSummary } from "../../../generated/api/AssignmentSummary";
import type { CatalogCapabilityFacet } from "../../../generated/api/CatalogCapabilityFacet";
import type { CatalogLicenseFacet } from "../../../generated/api/CatalogLicenseFacet";
import type { CatalogLicenseValue } from "../../../generated/api/CatalogLicenseValue";
import type { CatalogProblemDetail } from "../../../generated/api/CatalogProblemDetail";
import type { CatalogProblemSummary } from "../../../generated/api/CatalogProblemSummary";
import type { CatalogSearchFacets } from "../../../generated/api/CatalogSearchFacets";
import type { CatalogSearchPage } from "../../../generated/api/CatalogSearchPage";
import type { CatalogStatisticsFacet } from "../../../generated/api/CatalogStatisticsFacet";
import type { CatalogStatisticsStatus } from "../../../generated/api/CatalogStatisticsStatus";
import type { CatalogTaxonomyFacet } from "../../../generated/api/CatalogTaxonomyFacet";
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
import type { QuestionStatisticsView } from "../../../generated/api/QuestionStatisticsView";
import type { RunPolicies } from "../../../generated/api/RunPolicies";
import type { SelectionOrdering } from "../../../generated/api/SelectionOrdering";
import type {
  AssignmentCapabilityViolation,
  AddAssignmentItemInput,
  AssignmentCreateInput,
  AssignmentEditorEntryInput,
  AssignmentEditorDetail,
  AssignmentEditorInput,
  ReplaceAssignmentItemQuestionInput,
  CourseCreateInput,
  CourseRouteData,
} from "../contracts";
import {
  DecodeError,
  decodeArray,
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
import {
  MAX_CATALOG_CAPABILITY_FACETS,
  MAX_CATALOG_LICENSE_FACETS,
  MAX_CATALOG_PAGE_ITEMS,
  MINIMUM_STATISTICS_COHORT_SIZE,
  STATISTICS_DURATION_ESTIMATES_SECONDS,
  decodeAssignmentTitle,
  decodeBackendCapabilities,
  decodeBoundedArray,
  decodeCapability,
  decodeCatalogLifecycle,
  decodeCursor,
  decodeEnvelopeTitle,
  decodeIdentifier,
  decodeQuestionMetadata,
  decodeTaxonomyTerm,
  decodeTimestamp,
  field,
  kind,
  requireOnlyFields,
} from "./shared";
import { decodeCourseTerm } from "./course_term";
import { decodeContentBlock } from "./question_model";
import { decodeLearnerDisclosurePolicy } from "./assignment_policy";
import { decodeCourseAppearance } from "./course_appearance";

// Retain the established catalog-course import surface while course-term owns its decoding rules.
export { decodeCourseTerm, decodeCourseTermValidationFailure } from "./course_term";
export { decodeLearnerDisclosurePolicy } from "./assignment_policy";
export {
  decodeCourseAppearance,
  decodeCourseAppearanceUpdate,
  decodeCourseBannerCandidateReceipt,
} from "./course_appearance";

function decodeQuestionId(value: unknown, path: string): string {
  const questionId = decodeString(value, path);
  if (!/^[0-9A-HJKMNP-TV-Z]{3}-[0-9A-HJKMNP-TV-Z]{4}$/u.test(questionId)) {
    throw new Error(`${path} must be a canonical Question ID`);
  }
  return questionId;
}

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
      "capabilities",
      "metadata",
      "scope",
      "lifecycle",
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
    capabilities: decodeBackendCapabilities(
      field(record, "capabilities", path),
      `${path}.capabilities`,
    ),
    metadata: decodeQuestionMetadata(field(record, "metadata", path), `${path}.metadata`, strict),
    byline: decodePublicByline(field(record, "byline", path), `${path}.byline`),
    scope: decodeStringEnum(field(record, "scope", path), `${path}.scope`, [
      "institution",
      "public",
    ]),
    lifecycle: decodeCatalogLifecycle(
      field(record, "lifecycle", path),
      `${path}.lifecycle`,
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
 * additionally bind that DTO to their requested scope and the published state.
 */
export function isPublishedNativeCatalogProblemSummary(
  summary: CatalogProblemSummary,
  scope: CatalogProblemSummary["scope"],
): boolean {
  return (
    summary.backend === "native" &&
    summary.scope === scope &&
    summary.lifecycle.state === "published"
  );
}

function decodeCatalogTaxonomyFacet(value: unknown, path: string): CatalogTaxonomyFacet {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["term", "count"]);
  return {
    term: decodeTaxonomyTerm(field(record, "term", path), `${path}.term`, true),
    count: decodeNonnegativeInteger(field(record, "count", path), `${path}.count`),
  };
}

function decodeCatalogCapabilityFacet(value: unknown, path: string): CatalogCapabilityFacet {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["capability", "count"]);
  return {
    capability: decodeCapability(field(record, "capability", path), `${path}.capability`),
    count: decodeNonnegativeInteger(field(record, "count", path), `${path}.count`),
  };
}

function decodeCatalogLicenseFacet(value: unknown, path: string): CatalogLicenseFacet {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["license", "count"]);
  return {
    license: decodeStringEnum<CatalogLicenseValue>(
      field(record, "license", path),
      `${path}.license`,
      ["allRightsReserved", "ccBy", "ccBySa", "ccByNc", "cc0", "other"],
    ),
    count: decodeNonnegativeInteger(field(record, "count", path), `${path}.count`),
  };
}

function decodeCatalogStatisticsFacet(value: unknown, path: string): CatalogStatisticsFacet {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["available", "unavailable"]);
  return {
    available: decodeNonnegativeInteger(field(record, "available", path), `${path}.available`),
    unavailable: decodeNonnegativeInteger(
      field(record, "unavailable", path),
      `${path}.unavailable`,
    ),
  };
}

function decodeCatalogSearchFacets(value: unknown, path: string): CatalogSearchFacets {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["taxonomy", "capabilities", "licenses", "statistics"]);
  return {
    taxonomy: decodeBoundedArray(
      field(record, "taxonomy", path),
      `${path}.taxonomy`,
      MAX_CATALOG_TAXONOMY_FACETS,
      decodeCatalogTaxonomyFacet,
    ),
    capabilities: decodeBoundedArray(
      field(record, "capabilities", path),
      `${path}.capabilities`,
      MAX_CATALOG_CAPABILITY_FACETS,
      decodeCatalogCapabilityFacet,
    ),
    licenses: decodeBoundedArray(
      field(record, "licenses", path),
      `${path}.licenses`,
      MAX_CATALOG_LICENSE_FACETS,
      decodeCatalogLicenseFacet,
    ),
    statistics: decodeCatalogStatisticsFacet(
      field(record, "statistics", path),
      `${path}.statistics`,
    ),
  };
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

function decodeQuestionStatisticsView(value: unknown, path: string): QuestionStatisticsView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "cohortSize",
    "difficultyIndex",
    "attemptsMean",
    "timeMedianSecondsEstimate",
    "discriminationIndex",
  ]);
  const discriminationIndex =
    "discriminationIndex" in record
      ? decodeCorrelation(field(record, "discriminationIndex", path), `${path}.discriminationIndex`)
      : undefined;
  const decoded = {
    cohortSize: decodeNonnegativeInteger(field(record, "cohortSize", path), `${path}.cohortSize`),
    difficultyIndex: decodeUnitInterval(
      field(record, "difficultyIndex", path),
      `${path}.difficultyIndex`,
    ),
    attemptsMean: decodeFiniteNumber(field(record, "attemptsMean", path), `${path}.attemptsMean`),
    timeMedianSecondsEstimate: decodeNonnegativeInteger(
      field(record, "timeMedianSecondsEstimate", path),
      `${path}.timeMedianSecondsEstimate`,
    ),
    ...(discriminationIndex === undefined ? {} : { discriminationIndex }),
  } satisfies QuestionStatisticsView;
  if (decoded.cohortSize < MINIMUM_STATISTICS_COHORT_SIZE) {
    throw new DecodeError(
      `${path}.cohortSize`,
      `a safe integer at least ${MINIMUM_STATISTICS_COHORT_SIZE}`,
    );
  }
  if (decoded.attemptsMean < 1) {
    throw new DecodeError(`${path}.attemptsMean`, "a finite number at least 1");
  }
  if (
    !STATISTICS_DURATION_ESTIMATES_SECONDS.some(
      (estimate) => estimate === decoded.timeMedianSecondsEstimate,
    )
  ) {
    throw new DecodeError(
      `${path}.timeMedianSecondsEstimate`,
      "a supported fixed-histogram duration estimate",
    );
  }
  return decoded;
}

function decodeCatalogStatisticsStatus(value: unknown, path: string): CatalogStatisticsStatus {
  if (value === "unavailable") {
    return value;
  }
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["available"]);
  const decoded = {
    available: decodeQuestionStatisticsView(field(record, "available", path), `${path}.available`),
  } satisfies CatalogStatisticsStatus;
  return decoded;
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
      (item, itemPath) => decodeCatalogProblemSummary(item, itemPath, true),
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
  requireOnlyFields(record, path, ["summary", "prompt", "statistics"]);
  return {
    summary: decodeCatalogProblemSummary(field(record, "summary", path), `${path}.summary`, true),
    prompt: decodeBoundedArray(
      field(record, "prompt", path),
      `${path}.prompt`,
      MAX_CATALOG_PAGE_ITEMS,
      (block, blockPath) => decodeContentBlock(block, blockPath, true),
    ),
    statistics: decodeCatalogStatisticsStatus(
      field(record, "statistics", path),
      `${path}.statistics`,
    ),
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

function decodeRunPolicies(value: unknown, path: string, strict = false): RunPolicies {
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
  } satisfies RunPolicies;
  return decoded;
}

export function decodeCourseSummary(value: unknown, path = "response"): CourseSummary {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["id", "reference", "tenant", "title", "term", "role"]);
  const decoded = {
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    reference: decodeCourseReference(field(record, "reference", path), `${path}.reference`),
    tenant: decodeIdentifier(field(record, "tenant", path), `${path}.tenant`),
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
function decodeAssignmentEditorEntry(value: unknown, path: string): AssignmentEditorEntryInput {
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

function decodeAssignmentEditorEntries(
  value: unknown,
  path: string,
): ReadonlyArray<AssignmentEditorEntryInput> {
  const entries = decodeBoundedArray(
    value,
    path,
    MAX_ASSIGNMENT_ORDERED_ENTRIES,
    decodeAssignmentEditorEntry,
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
      "tenant",
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
    tenant: decodeIdentifier(field(record, "tenant", path), `${path}.tenant`),
    courseId: decodeIdentifier(field(record, "courseId", path), `${path}.courseId`),
    title: decodeNonemptyString(field(record, "title", path), `${path}.title`),
    items: decodeArray(field(record, "items", path), `${path}.items`, decodeAssignmentItem),
    selectionGroups: decodeArray(
      field(record, "selectionGroups", path),
      `${path}.selectionGroups`,
      decodeAssignmentSelectionGroup,
    ),
    disclosurePolicy: decodeLearnerDisclosurePolicy(
      field(record, "disclosurePolicy", path),
      `${path}.disclosurePolicy`,
    ),
    policies: decodeRunPolicies(field(record, "policies", path), `${path}.policies`),
  } satisfies AssignmentSummary;
  return decoded;
}

/** Decode the learner transport, which deliberately excludes authority inputs. */
import {
  decodeInstructorAssignmentCurrentState,
  decodeInstructorAssignmentTeachingSettingsLocal,
} from "./assignment_teaching_delivery";
export {
  decodeAssignmentTeachingSettingsValidationFailure,
  decodeInstructorAssignmentTeachingSettingsLocal,
  decodeLearnerAssignmentDetail,
  decodeLearnerAssignmentSummary,
} from "./assignment_teaching_delivery";
export function decodeAssignmentEditorInput(
  value: unknown,
  path = "response",
): AssignmentEditorInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["title", "entries", "policies", "disclosurePolicy"]);
  const decoded = {
    title: decodeAssignmentTitle(field(record, "title", path), `${path}.title`),
    entries: decodeAssignmentEditorEntries(field(record, "entries", path), `${path}.entries`),
    policies: decodeRunPolicies(field(record, "policies", path), `${path}.policies`, true),
    disclosurePolicy: decodeLearnerDisclosurePolicy(
      field(record, "disclosurePolicy", path),
      `${path}.disclosurePolicy`,
    ),
  } satisfies AssignmentEditorInput;
  return decoded;
}

export function decodeAssignmentCreateInput(
  value: unknown,
  path = "response",
): AssignmentCreateInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["title", "entries", "policies", "disclosurePolicy"]);
  return {
    title: decodeAssignmentTitle(field(record, "title", path), `${path}.title`),
    entries: decodeAssignmentEditorEntries(field(record, "entries", path), `${path}.entries`),
    policies: decodeRunPolicies(field(record, "policies", path), `${path}.policies`, true),
    disclosurePolicy: decodeLearnerDisclosurePolicy(
      field(record, "disclosurePolicy", path),
      `${path}.disclosurePolicy`,
    ),
  };
}

export function decodeAddAssignmentItemInput(
  value: unknown,
  path = "response",
): AddAssignmentItemInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["questionId", "position"]);
  return {
    questionId: decodeQuestionId(field(record, "questionId", path), `${path}.questionId`),
    position: decodeNonnegativeInteger(field(record, "position", path), `${path}.position`),
  };
}

export function decodeReplaceAssignmentItemQuestionInput(
  value: unknown,
  path = "response",
): ReplaceAssignmentItemQuestionInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["questionId"]);
  return {
    questionId: decodeQuestionId(field(record, "questionId", path), `${path}.questionId`),
  };
}

/**
 * Decode the assignment editor's deliberately narrow, revisioned projection.
 * It must never grow question content, source material, or server-only policy.
 */
export function decodeAssignmentEditorDetail(
  value: unknown,
  path = "response",
): Omit<AssignmentEditorDetail, "revision"> {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "id",
    "reference",
    "tenant",
    "courseId",
    "title",
    "items",
    "selectionGroups",
    "disclosurePolicy",
    "policies",
    "teachingSettings",
    "currentState",
  ]);
  const summary = decodeAssignmentSummary(record, path, false);
  const teachingSettings = decodeInstructorAssignmentTeachingSettingsLocal(
    field(record, "teachingSettings", path),
    `${path}.teachingSettings`,
  );
  const currentState = decodeInstructorAssignmentCurrentState(
    field(record, "currentState", path),
    `${path}.currentState`,
  );
  const currentMatchesIntent =
    (teachingSettings.lifecycle === "draft" && currentState.state === "draft") ||
    (teachingSettings.lifecycle === "archived" && currentState.state === "archived") ||
    (teachingSettings.lifecycle === "closed" &&
      currentState.state === "closed" &&
      currentState.closedAt === null) ||
    (teachingSettings.lifecycle === "published" &&
      (currentState.state === "scheduled" ||
        currentState.state === "open" ||
        (currentState.state === "closed" && currentState.closedAt !== null)));
  if (!currentMatchesIntent)
    throw new DecodeError(
      `${path}.currentState`,
      "a server-derived state consistent with the stored lifecycle intent",
    );
  const decoded = {
    id: summary.id,
    reference: summary.reference,
    tenant: summary.tenant,
    courseId: summary.courseId,
    title: summary.title,
    items: summary.items,
    selectionGroups: summary.selectionGroups,
    disclosurePolicy: summary.disclosurePolicy,
    policies: summary.policies,
    teachingSettings,
    currentState,
  } satisfies Omit<AssignmentEditorDetail, "revision">;
  return decoded;
}

export function decodeAssignmentCapabilityViolations(
  value: unknown,
  path = "response",
): ReadonlyArray<AssignmentCapabilityViolation> {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["error", "violations"]);
  if (field(record, "error", path) !== "assignment configuration is not supported") {
    throw new DecodeError(`${path}.error`, "the assignment capability validation failure marker");
  }
  return decodeArray(
    field(record, "violations", path),
    `${path}.violations`,
    (entry, entryPath) => {
      const violation = decodeRecord(entry, entryPath);
      requireOnlyFields(violation, entryPath, ["title", "questionId", "capability"]);
      const decoded = {
        title: decodeEnvelopeTitle(field(violation, "title", entryPath), `${entryPath}.title`),
        questionId: decodeQuestionId(
          field(violation, "questionId", entryPath),
          `${entryPath}.questionId`,
        ),
        capability: decodeCapability(
          field(violation, "capability", entryPath),
          `${entryPath}.capability`,
        ),
      } satisfies AssignmentCapabilityViolation;
      return decoded;
    },
  );
}

export * from "./question_model";
