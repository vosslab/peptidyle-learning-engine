// Catalog, course, and assignment browser-visible API DTOs.

import { MAX_CATALOG_TAXONOMY_FACETS } from "../../../generated/api/MAX_CATALOG_TAXONOMY_FACETS";
import type { AssignmentDeliveryState } from "../../../generated/api/AssignmentDeliveryState";
import type { AssignmentItemSummary as AssignmentItem } from "../../../generated/api/AssignmentItemSummary";
import type { AssignmentScoringMode } from "../../../generated/api/AssignmentScoringMode";
import type { AssignmentSelectionCandidateSummary as AssignmentSelectionCandidate } from "../../../generated/api/AssignmentSelectionCandidateSummary";
import type { AssignmentSelectionGroupSummary as AssignmentSelectionGroup } from "../../../generated/api/AssignmentSelectionGroupSummary";
import type { AssignmentSummary } from "../../../generated/api/AssignmentSummary";
import type { AssignmentRunTiming } from "../../../generated/api/AssignmentRunTiming";
import { MAX_ASSIGNMENT_TIME_LIMIT_SECONDS } from "../../../generated/api/MAX_ASSIGNMENT_TIME_LIMIT_SECONDS";
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
import type { CourseAppearance } from "../../../generated/api/CourseAppearance";
import type { CourseAppearanceUpdate } from "../../../generated/api/CourseAppearanceUpdate";
import type { CourseBannerAlternativeText } from "../../../generated/api/CourseBannerAlternativeText";
import type { CourseBannerCandidateReceipt } from "../../../generated/api/CourseBannerCandidateReceipt";
import type { CourseBannerPresentation } from "../../../generated/api/CourseBannerPresentation";
import type { CourseSummary } from "../../../generated/api/CourseSummary";
import type { CourseThemeId } from "../../../generated/api/CourseThemeId";
import type { PointValue } from "../../../generated/api/PointValue";
import type { QuestionStatisticsView } from "../../../generated/api/QuestionStatisticsView";
import type { RunPolicies } from "../../../generated/api/RunPolicies";
import type { SelectionOrdering } from "../../../generated/api/SelectionOrdering";
import type {
  AssignmentCapabilityViolation,
  AddAssignmentItemInput,
  AssignmentCreateInput,
  AssignmentEditorDetail,
  AssignmentEditorInput,
  ReplaceAssignmentItemQuestionInput,
  AssignmentSummaryWithTiming,
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
  decodeUuid,
} from "../decoder";
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
  decodePublicRouteNumber,
  decodeQuestionMetadata,
  decodeTaxonomyTerm,
  decodeTimestamp,
  field,
  kind,
  requireOnlyFields,
} from "./shared";
import { decodeContentBlock } from "./question_model";

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

export function decodeCourseSummary(
  value: unknown,
  path = "response",
  strict = false,
): CourseSummary {
  const record = decodeRecord(value, path);
  if (strict) requireOnlyFields(record, path, ["id", "publicId", "tenant", "title", "role"]);
  const decoded = {
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    publicId: decodePublicRouteNumber(field(record, "publicId", path), `${path}.publicId`),
    tenant: decodeIdentifier(field(record, "tenant", path), `${path}.tenant`),
    title: decodeNonemptyString(field(record, "title", path), `${path}.title`),
    role: decodeStringEnum(field(record, "role", path), `${path}.role`, ["student", "instructor"]),
  } satisfies CourseSummary;
  return decoded;
}

/** Strict request decoder for the public course-creation transport boundary. */
export function decodeCourseCreateInput(value: unknown, path = "request"): CourseCreateInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["title"]);
  const title = decodeNonemptyString(field(record, "title", path), `${path}.title`);
  if (title.trim().length === 0) {
    throw new DecodeError(`${path}.title`, "a course title containing non-whitespace content");
  }
  const decoded = {
    title,
  } satisfies CourseCreateInput;
  return decoded;
}

const COURSE_THEME_IDS = [
  "tundra",
  "forest",
  "desert",
  "grass",
  "arctic",
  "ocean",
  "tropical",
  "coral-reef",
  "swamp",
  "underground",
  "salt-marsh",
  "wetland",
  "sea-floor",
  "magma",
  "beach",
] as const satisfies ReadonlyArray<CourseThemeId>;

function decodeCourseBannerAlternativeText(
  value: unknown,
  path: string,
): CourseBannerAlternativeText {
  const record = decodeRecord(value, path);
  const kind = decodeStringEnum(field(record, "kind", path), `${path}.kind`, [
    "decorative",
    "informative",
  ]);
  if (kind === "decorative") {
    requireOnlyFields(record, path, ["kind"]);
    return { kind };
  }
  requireOnlyFields(record, path, ["kind", "text"]);
  const text = decodeNonemptyString(field(record, "text", path), `${path}.text`);
  if (text.trim().length === 0 || [...text].length > 160) {
    throw new DecodeError(`${path}.text`, "1 through 160 nonblank characters");
  }
  return { kind, text };
}

function decodeCourseBannerPresentation(value: unknown, path: string): CourseBannerPresentation {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["id", "alternativeText"]);
  return {
    id: decodeUuid(field(record, "id", path), `${path}.id`),
    alternativeText: decodeCourseBannerAlternativeText(
      field(record, "alternativeText", path),
      `${path}.alternativeText`,
    ),
  };
}

/** Strict decoder for the safe course-appearance projection. */
export function decodeCourseAppearance(value: unknown, path = "response"): CourseAppearance {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["theme", "revision", "banner"]);
  const revision = decodeString(field(record, "revision", path), `${path}.revision`);
  if (!/^[1-9][0-9]*$/u.test(revision) || BigInt(revision) > 9_223_372_036_854_775_807n) {
    throw new DecodeError(`${path}.revision`, "a canonical positive PostgreSQL bigint string");
  }
  return {
    theme: decodeStringEnum(field(record, "theme", path), `${path}.theme`, COURSE_THEME_IDS),
    revision,
    banner: decodeNullable(
      field(record, "banner", path),
      `${path}.banner`,
      decodeCourseBannerPresentation,
    ),
  };
}

/** Strict receipt for a course-bound, server-normalized temporary banner. */
export function decodeCourseBannerCandidateReceipt(
  value: unknown,
  path = "response",
): CourseBannerCandidateReceipt {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["candidate"]);
  return { candidate: decodeUuid(field(record, "candidate", path), `${path}.candidate`) };
}

/** Strict atomic course-appearance update used by mocks and boundary tests. */
export function decodeCourseAppearanceUpdate(
  value: unknown,
  path = "request",
): CourseAppearanceUpdate {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["theme", "banner"]);
  const theme = decodeStringEnum(field(record, "theme", path), `${path}.theme`, COURSE_THEME_IDS);
  const banner = decodeRecord(field(record, "banner", path), `${path}.banner`);
  const kind = decodeStringEnum(field(banner, "kind", `${path}.banner`), `${path}.banner.kind`, [
    "keep",
    "remove",
    "replace",
  ]);
  switch (kind) {
    case "remove":
      requireOnlyFields(banner, `${path}.banner`, ["kind"]);
      return { theme, banner: { kind } };
    case "keep":
      requireOnlyFields(banner, `${path}.banner`, ["kind", "alternativeText"]);
      return {
        theme,
        banner: {
          kind,
          alternativeText: decodeCourseBannerAlternativeText(
            field(banner, "alternativeText", `${path}.banner`),
            `${path}.banner.alternativeText`,
          ),
        },
      };
    case "replace":
      requireOnlyFields(banner, `${path}.banner`, ["kind", "candidate", "alternativeText"]);
      return {
        theme,
        banner: {
          kind,
          candidate: decodeUuid(
            field(banner, "candidate", `${path}.banner`),
            `${path}.banner.candidate`,
          ),
          alternativeText: decodeCourseBannerAlternativeText(
            field(banner, "alternativeText", `${path}.banner`),
            `${path}.banner.alternativeText`,
          ),
        },
      };
  }
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

function decodeAssignmentItem(value: unknown, path: string): AssignmentItem {
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

/** Request-only item shape: the server already owns display metadata. */
function decodeAssignmentUpdateItem(
  value: unknown,
  path: string,
): AssignmentEditorInput["items"][number] {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "id",
    "questionId",
    "position",
    "pointsPossible",
    "deliveryState",
    "scoringMode",
  ]);
  return {
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    questionId: decodeQuestionId(field(record, "questionId", path), `${path}.questionId`),
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

function decodeAssignmentSelectionGroup(value: unknown, path: string): AssignmentSelectionGroup {
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
      "publicId",
      "tenant",
      "courseId",
      "title",
      "items",
      "selectionGroups",
      "policies",
    ]);
  }
  const decoded = {
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    publicId: decodePublicRouteNumber(field(record, "publicId", path), `${path}.publicId`),
    tenant: decodeIdentifier(field(record, "tenant", path), `${path}.tenant`),
    courseId: decodeIdentifier(field(record, "courseId", path), `${path}.courseId`),
    title: decodeNonemptyString(field(record, "title", path), `${path}.title`),
    items: decodeArray(field(record, "items", path), `${path}.items`, decodeAssignmentItem),
    selectionGroups: decodeArray(
      field(record, "selectionGroups", path),
      `${path}.selectionGroups`,
      decodeAssignmentSelectionGroup,
    ),
    policies: decodeRunPolicies(field(record, "policies", path), `${path}.policies`),
  } satisfies AssignmentSummary;
  return decoded;
}

export function decodeAssignmentSummaryWithTiming(
  value: unknown,
  path = "response",
  strict = false,
): AssignmentSummaryWithTiming {
  const record = decodeRecord(value, path);
  if (strict) {
    requireOnlyFields(record, path, [
      "id",
      "publicId",
      "tenant",
      "courseId",
      "title",
      "items",
      "selectionGroups",
      "policies",
      "assignmentTiming",
    ]);
  }
  const summary = decodeAssignmentSummary(record, path, strict);
  const timingValue = record.assignmentTiming ?? { timeLimitSeconds: null };
  return {
    ...summary,
    assignmentTiming: decodeAssignmentRunTiming(timingValue, `${path}.assignmentTiming`),
  } satisfies AssignmentSummaryWithTiming;
}

/**
 * Decode the exact mutable assignment body before it leaves the browser and
 * when a mock validates a request. This keeps request and response drift
 * visible instead of silently dropping a new field.
 */
export function decodeAssignmentEditorInput(
  value: unknown,
  path = "response",
): AssignmentEditorInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["title", "items", "policies", "assignmentTiming"]);
  const decoded = {
    title: decodeAssignmentTitle(field(record, "title", path), `${path}.title`),
    items: decodeArray(field(record, "items", path), `${path}.items`, decodeAssignmentUpdateItem),
    policies: decodeRunPolicies(field(record, "policies", path), `${path}.policies`, true),
    assignmentTiming: decodeAssignmentRunTiming(
      field(record, "assignmentTiming", path),
      `${path}.assignmentTiming`,
    ),
  } satisfies AssignmentEditorInput;
  return decoded;
}

export function decodeAssignmentCreateInput(
  value: unknown,
  path = "response",
): AssignmentCreateInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["title", "questionIds", "policies", "assignmentTiming"]);
  return {
    title: decodeAssignmentTitle(field(record, "title", path), `${path}.title`),
    questionIds: decodeArray(
      field(record, "questionIds", path),
      `${path}.questionIds`,
      decodeQuestionId,
    ),
    policies: decodeRunPolicies(field(record, "policies", path), `${path}.policies`, true),
    assignmentTiming: decodeAssignmentRunTiming(
      field(record, "assignmentTiming", path),
      `${path}.assignmentTiming`,
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
    "publicId",
    "tenant",
    "courseId",
    "title",
    "items",
    "selectionGroups",
    "policies",
    "assignmentTiming",
  ]);
  const summary = decodeAssignmentSummary(record, path, false);
  const decoded = {
    id: summary.id,
    publicId: summary.publicId,
    tenant: summary.tenant,
    courseId: summary.courseId,
    title: summary.title,
    items: summary.items,
    selectionGroups: summary.selectionGroups,
    policies: summary.policies,
    assignmentTiming: decodeAssignmentRunTiming(
      field(record, "assignmentTiming", path),
      `${path}.assignmentTiming`,
    ),
  } satisfies Omit<AssignmentEditorDetail, "revision">;
  return decoded;
}

function decodeAssignmentRunTiming(value: unknown, path: string): AssignmentRunTiming {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["timeLimitSeconds"]);
  const timeLimitSeconds = decodeNullable(
    field(record, "timeLimitSeconds", path),
    `${path}.timeLimitSeconds`,
    (seconds, secondsPath) => {
      const decoded = decodePositiveInteger(seconds, secondsPath);
      if (decoded > MAX_ASSIGNMENT_TIME_LIMIT_SECONDS) {
        throw new DecodeError(
          secondsPath,
          `a positive whole-second limit no greater than ${MAX_ASSIGNMENT_TIME_LIMIT_SECONDS}`,
        );
      }
      return decoded;
    },
  );
  return { timeLimitSeconds };
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
