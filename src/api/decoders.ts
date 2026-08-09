// decoders.ts - exhaustive runtime decoders for browser-visible API DTOs.

import type { AssetRef } from "../../generated/api/AssetRef";
import { MAX_CATALOG_TAXONOMY_FACETS } from "../../generated/api/MAX_CATALOG_TAXONOMY_FACETS";
import { MAX_QUESTION_TITLE_UNICODE_SCALARS } from "../../generated/api/MAX_QUESTION_TITLE_UNICODE_SCALARS";
import type { AssignmentEnrollment } from "../../generated/api/AssignmentEnrollment";
import type { AssignmentDeliveryState } from "../../generated/api/AssignmentDeliveryState";
import type { AssignmentItem } from "../../generated/api/AssignmentItem";
import type { AssignmentRun } from "../../generated/api/AssignmentRun";
import type { AssignmentScoringMode } from "../../generated/api/AssignmentScoringMode";
import type { AssignmentSelectionCandidate } from "../../generated/api/AssignmentSelectionCandidate";
import type { AssignmentSelectionGroup } from "../../generated/api/AssignmentSelectionGroup";
import type { AssignmentSummary } from "../../generated/api/AssignmentSummary";
import type { AttemptPolicy } from "../../generated/api/AttemptPolicy";
import type { AttemptProvenance } from "../../generated/api/AttemptProvenance";
import type { AttemptResult } from "../../generated/api/AttemptResult";
import type { AttemptStatus } from "../../generated/api/AttemptStatus";
import type { AttemptTimerRecord } from "../../generated/api/AttemptTimerRecord";
import type { BackendCapabilities } from "../../generated/api/BackendCapabilities";
import type { Capability } from "../../generated/api/Capability";
import type { CatalogLifecycle } from "../../generated/api/CatalogLifecycle";
import type { CatalogCapabilityFacet } from "../../generated/api/CatalogCapabilityFacet";
import type { CatalogLicenseFacet } from "../../generated/api/CatalogLicenseFacet";
import type { CatalogLicenseValue } from "../../generated/api/CatalogLicenseValue";
import type { CatalogProblemDetail } from "../../generated/api/CatalogProblemDetail";
import type { CatalogProblemSummary } from "../../generated/api/CatalogProblemSummary";
import type { CatalogSearchFacets } from "../../generated/api/CatalogSearchFacets";
import type { CatalogSearchPage } from "../../generated/api/CatalogSearchPage";
import type { CatalogStatisticsFacet } from "../../generated/api/CatalogStatisticsFacet";
import type { CatalogStatisticsStatus } from "../../generated/api/CatalogStatisticsStatus";
import type { CatalogTaxonomyFacet } from "../../generated/api/CatalogTaxonomyFacet";
import type { ChoiceOption } from "../../generated/api/ChoiceOption";
import type { CompletionRequirement } from "../../generated/api/CompletionRequirement";
import type { ContentBlock } from "../../generated/api/ContentBlock";
import type { ContinuedPractice } from "../../generated/api/ContinuedPractice";
import type { CourseSummary } from "../../generated/api/CourseSummary";
import type { DisclosedFeedback } from "../../generated/api/DisclosedFeedback";
import type { DraftQuestionDefinition } from "../../generated/api/DraftQuestionDefinition";
import type { DraftQuestionSource } from "../../generated/api/DraftQuestionSource";
import type { GradingDefinition } from "../../generated/api/GradingDefinition";
import type { GradebookSummaryRow } from "../../generated/api/GradebookSummaryRow";
import type { License } from "../../generated/api/License";
import type { NumericTolerance } from "../../generated/api/NumericTolerance";
import type { ParameterSpec } from "../../generated/api/ParameterSpec";
import type { PointValue } from "../../generated/api/PointValue";
import type { ProblemVersionRef } from "../../generated/api/ProblemVersionRef";
import type { QuestionAttempt } from "../../generated/api/QuestionAttempt";
import type { QuestionDefinition } from "../../generated/api/QuestionDefinition";
import type { QuestionEnvelope } from "../../generated/api/QuestionEnvelope";
import type { ExternalToolLaunch } from "./contracts";
import type { QuestionMetadata } from "../../generated/api/QuestionMetadata";
import type { QuestionSource } from "../../generated/api/QuestionSource";
import type { QuestionStatisticsView } from "../../generated/api/QuestionStatisticsView";
import type { RandomizationDefinition } from "../../generated/api/RandomizationDefinition";
import type { ResponseDefinition } from "../../generated/api/ResponseDefinition";
import type { RunPolicies } from "../../generated/api/RunPolicies";
import type { SelectionCardinality } from "../../generated/api/SelectionCardinality";
import type { SelectionOrdering } from "../../generated/api/SelectionOrdering";
import type { SourceArtifact } from "../../generated/api/SourceArtifact";
import type { StudentAssignmentSummary } from "../../generated/api/StudentAssignmentSummary";
import type { StudentResponse } from "../../generated/api/StudentResponse";
import type { TaxonomyTerm } from "../../generated/api/TaxonomyTerm";
import type { TimingPolicy } from "../../generated/api/TimingPolicy";
import type { WorkspaceDraftSummary } from "../../generated/api/WorkspaceDraftSummary";
import type { QuestionBackend } from "../../generated/api/QuestionBackend";
import type {
  CapabilityViolation,
  ResponseFormatReport,
  ResponseFormatViolation,
  TimerVerdict,
} from "../wasm/index";
import type {
  AuthSession,
  AssignmentCapabilityViolation,
  AssignmentEditorDetail,
  AssignmentEditorInput,
  CursorPage,
  EnrollmentView,
  FeedbackReleaseResponse,
  RunSummaryOutcome,
  RunSummaryResponse,
  SubmissionReceipt,
  PublicationDiff,
  PublicationSemanticProjection,
  PublicationResult,
  PublicationValidationReport,
  PublicationReadinessFailure,
  PublicationViolation,
  NextIssuedAttempt,
  PrefetchedNextQuestion,
} from "./contracts";
import {
  DecodeError,
  decodeArray,
  decodeBoolean,
  decodeDictionary,
  decodeField,
  decodeFiniteNumber,
  decodeNonemptyString,
  decodeNonnegativeInteger,
  decodeNullable,
  decodePositiveInteger,
  decodeRecord,
  decodeSafeInteger,
  decodeString,
  decodeStringEnum,
  decodeTrue,
  decodeUuid,
  type Decoder,
} from "./decoder";

const CAPABILITIES = [
  "algorithmicGeneration",
  "clientRendering",
  "serverGrading",
  "partialCredit",
  "hints",
  "perQuestionTiming",
  "printExport",
  "offlinePreview",
] as const satisfies ReadonlyArray<Capability>;
const MAX_CURSOR_LENGTH = 512;
const QUESTION_BACKENDS = [
  "native",
  "webwork",
  "qti",
  "h5p",
  "imathas",
] as const satisfies ReadonlyArray<QuestionBackend>;

const MAX_CATALOG_PAGE_ITEMS = 100;
const MAX_CATALOG_CAPABILITY_FACETS = CAPABILITIES.length;
const MAX_CATALOG_LICENSE_FACETS = 6;
const MINIMUM_STATISTICS_COHORT_SIZE = 5;
const STATISTICS_DURATION_ESTIMATES_SECONDS = [
  1, 5, 15, 30, 60, 120, 300, 900, 3_600, 86_400,
] as const;
const MAX_PUBLICATION_SEMANTIC_ENTRIES = 100;
const MAX_ASSIGNMENT_TITLE_UNICODE_SCALARS = 200;

function decodeEnvelopeTitle(value: unknown, path: string): string {
  const title = decodeNonemptyString(value, path);
  if (title.trim().length === 0) {
    throw new DecodeError(path, "a title containing non-whitespace content");
  }
  if (Array.from(title).length > MAX_QUESTION_TITLE_UNICODE_SCALARS) {
    throw new DecodeError(
      path,
      `a title no longer than ${MAX_QUESTION_TITLE_UNICODE_SCALARS} Unicode scalar values`,
    );
  }
  return title;
}

function decodeAssignmentTitle(value: unknown, path: string): string {
  const title = decodeNonemptyString(value, path);
  if (title.trim().length === 0) {
    throw new DecodeError(path, "an assignment title containing non-whitespace content");
  }
  if (Array.from(title).length > MAX_ASSIGNMENT_TITLE_UNICODE_SCALARS) {
    throw new DecodeError(
      path,
      `an assignment title no longer than ${MAX_ASSIGNMENT_TITLE_UNICODE_SCALARS} Unicode scalar values`,
    );
  }
  return title;
}

function field(record: Record<string, unknown>, key: string, path: string): unknown {
  return decodeField(record, key, path);
}

function requireOnlyFields(
  record: Record<string, unknown>,
  path: string,
  allowed: ReadonlyArray<string>,
): void {
  for (const key of Object.keys(record)) {
    if (!allowed.includes(key)) {
      throw new DecodeError(`${path}.${key}`, "a field allowed by this response contract");
    }
  }
}

function kind(record: Record<string, unknown>, path: string): string {
  return decodeString(field(record, "kind", path), `${path}.kind`);
}

function state(record: Record<string, unknown>, path: string): string {
  return decodeString(field(record, "state", path), `${path}.state`);
}

function decodeTimestamp(value: unknown, path: string): number {
  return decodeSafeInteger(value, path);
}

function decodeIdentifier(value: unknown, path: string): string {
  return decodeUuid(value, path);
}

function decodeSha256(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  if (!/^[0-9a-f]{64}$/i.test(decoded)) {
    throw new DecodeError(path, "a 64-character SHA-256 hexadecimal digest");
  }
  return decoded;
}

function decodeCapability(value: unknown, path: string): Capability {
  return decodeStringEnum(value, path, CAPABILITIES);
}

function decodeBackendCapabilities(value: unknown, path: string): BackendCapabilities {
  return decodeArray(value, path, decodeCapability);
}

function decodeProblemVersionRef(value: unknown, path: string, strict = false): ProblemVersionRef {
  const record = decodeRecord(value, path);
  if (strict) requireOnlyFields(record, path, ["problem", "version"]);
  const decoded = {
    problem: decodeIdentifier(field(record, "problem", path), `${path}.problem`),
    version: decodeIdentifier(field(record, "version", path), `${path}.version`),
  } satisfies ProblemVersionRef;
  return decoded;
}

function decodeTaxonomyTerm(value: unknown, path: string, strict = false): TaxonomyTerm {
  const record = decodeRecord(value, path);
  if (strict) {
    requireOnlyFields(record, path, ["scheme", "code", "label"]);
  }
  const decoded = {
    scheme: decodeNonemptyString(field(record, "scheme", path), `${path}.scheme`),
    code: decodeNonemptyString(field(record, "code", path), `${path}.code`),
    label: decodeNonemptyString(field(record, "label", path), `${path}.label`),
  } satisfies TaxonomyTerm;
  return decoded;
}

function decodeLicense(value: unknown, path: string, strict = false): License {
  const record = decodeRecord(value, path);
  const tag = kind(record, path);
  switch (tag) {
    case "allRightsReserved":
    case "ccBy":
    case "ccBySa":
    case "ccByNc":
    case "cc0":
      if (strict) {
        requireOnlyFields(record, path, ["kind"]);
      }
      return { kind: tag };
    case "other": {
      if (strict) {
        requireOnlyFields(record, path, ["kind", "spdx"]);
      }
      const decoded = {
        kind: tag,
        spdx: decodeNonemptyString(field(record, "spdx", path), `${path}.spdx`),
      } satisfies License;
      return decoded;
    }
    default:
      throw new DecodeError(`${path}.kind`, "a known license kind");
  }
}

function decodeQuestionMetadata(value: unknown, path: string, strict = false): QuestionMetadata {
  const record = decodeRecord(value, path);
  if (strict) {
    requireOnlyFields(record, path, ["title", "tags", "taxonomy", "license", "language"]);
  }
  const decoded = {
    title: decodeNonemptyString(field(record, "title", path), `${path}.title`),
    tags: decodeArray(field(record, "tags", path), `${path}.tags`, decodeString),
    taxonomy: decodeArray(field(record, "taxonomy", path), `${path}.taxonomy`, (term, termPath) =>
      decodeTaxonomyTerm(term, termPath, strict),
    ),
    license: decodeLicense(field(record, "license", path), `${path}.license`, strict),
    language: decodeNonemptyString(field(record, "language", path), `${path}.language`),
  } satisfies QuestionMetadata;
  return decoded;
}

function decodeCatalogLifecycle(value: unknown, path: string, strict = false): CatalogLifecycle {
  const record = decodeRecord(value, path);
  const lifecycle = state(record, path);
  switch (lifecycle) {
    case "published":
      if (strict) {
        requireOnlyFields(record, path, ["state"]);
      }
      return { state: lifecycle };
    case "deprecated":
    case "archived": {
      if (strict) {
        requireOnlyFields(record, path, ["state", "reason"]);
      }
      const decoded = {
        state: lifecycle,
        reason: decodeNonemptyString(field(record, "reason", path), `${path}.reason`),
      } satisfies CatalogLifecycle;
      return decoded;
    }
    default:
      throw new DecodeError(`${path}.state`, "a known catalog lifecycle");
  }
}

export function decodeCatalogProblemSummary(
  value: unknown,
  path = "response",
  strict = false,
): CatalogProblemSummary {
  const record = decodeRecord(value, path);
  if (strict) {
    requireOnlyFields(record, path, [
      "problem",
      "publicId",
      "version",
      "versionNumber",
      "backend",
      "capabilities",
      "metadata",
      "scope",
      "lifecycle",
      "authors",
      "previousVersion",
      "derivedFrom",
      "publishedAt",
    ]);
  }
  const decoded = {
    problem: decodeIdentifier(field(record, "problem", path), `${path}.problem`),
    publicId: decodePositiveInteger(field(record, "publicId", path), `${path}.publicId`),
    version: decodeIdentifier(field(record, "version", path), `${path}.version`),
    versionNumber: decodePositiveInteger(
      field(record, "versionNumber", path),
      `${path}.versionNumber`,
    ),
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
    authors: decodeArray(field(record, "authors", path), `${path}.authors`, decodeIdentifier),
    previousVersion: decodeNullable(
      field(record, "previousVersion", path),
      `${path}.previousVersion`,
      decodeIdentifier,
    ),
    derivedFrom: decodeNullable(
      field(record, "derivedFrom", path),
      `${path}.derivedFrom`,
      decodeProblemVersionRef,
    ),
    publishedAt: decodeTimestamp(field(record, "publishedAt", path), `${path}.publishedAt`),
  } satisfies CatalogProblemSummary;
  return decoded;
}

function decodeBoundedArray<T>(
  value: unknown,
  path: string,
  maximum: number,
  decodeElement: Decoder<T>,
): Array<T> {
  const decoded = decodeArray(value, path, decodeElement);
  if (decoded.length > maximum) {
    throw new DecodeError(path, `an array with at most ${maximum} entries`);
  }
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
      decodeNonemptyString,
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
  const decoded = {
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    tenant: decodeIdentifier(field(record, "tenant", path), `${path}.tenant`),
    title: decodeNonemptyString(field(record, "title", path), `${path}.title`),
    role: decodeStringEnum(field(record, "role", path), `${path}.role`, [
      "student",
      "instructor",
      "administrator",
    ]),
  } satisfies CourseSummary;
  return decoded;
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
    "reference",
    "position",
    "pointsPossible",
    "deliveryState",
    "scoringMode",
  ]);
  return {
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    reference: decodeProblemVersionRef(field(record, "reference", path), `${path}.reference`, true),
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
  requireOnlyFields(record, path, ["id", "position", "reference", "deliveryState"]);
  return {
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    position: decodeNonnegativeInteger(field(record, "position", path), `${path}.position`),
    reference: decodeProblemVersionRef(field(record, "reference", path), `${path}.reference`, true),
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
  requireOnlyFields(record, path, ["title", "problems", "policies"]);
  const decoded = {
    title: decodeAssignmentTitle(field(record, "title", path), `${path}.title`),
    problems: decodeArray(
      field(record, "problems", path),
      `${path}.problems`,
      (reference, referencePath) => decodeProblemVersionRef(reference, referencePath, true),
    ),
    policies: decodeRunPolicies(field(record, "policies", path), `${path}.policies`, true),
  } satisfies AssignmentEditorInput;
  return decoded;
}

/**
 * Decode the assignment editor's deliberately narrow, revisioned projection.
 * It must never grow question content, source material, or server-only policy.
 */
export function decodeAssignmentEditorDetail(
  value: unknown,
  path = "response",
): Omit<AssignmentEditorDetail, "revision"> {
  const summary = decodeAssignmentSummary(value, path, true);
  const decoded = {
    id: summary.id,
    tenant: summary.tenant,
    courseId: summary.courseId,
    title: summary.title,
    problems: summary.items
      .filter((item) => item.deliveryState === "active")
      .sort((left, right) => left.position - right.position)
      .map((item) => item.reference),
    policies: summary.policies,
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
      requireOnlyFields(violation, entryPath, ["title", "reference", "capability"]);
      const decoded = {
        title: decodeEnvelopeTitle(field(violation, "title", entryPath), `${entryPath}.title`),
        reference: decodeProblemVersionRef(
          field(violation, "reference", entryPath),
          `${entryPath}.reference`,
          true,
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

function decodeAssetRef(value: unknown, path: string, strict = false): AssetRef {
  const record = decodeRecord(value, path);
  if (strict) {
    requireOnlyFields(record, path, ["asset", "checksum"]);
  }
  const decoded = {
    asset: decodeIdentifier(field(record, "asset", path), `${path}.asset`),
    checksum: decodeSha256(field(record, "checksum", path), `${path}.checksum`),
  } satisfies AssetRef;
  return decoded;
}

function decodeContentBlock(value: unknown, path: string, strict = false): ContentBlock {
  const record = decodeRecord(value, path);
  const block = kind(record, path);
  switch (block) {
    case "text": {
      if (strict) {
        requireOnlyFields(record, path, ["kind", "markdown"]);
      }
      const decoded = {
        kind: block,
        markdown: decodeString(field(record, "markdown", path), `${path}.markdown`),
      } satisfies ContentBlock;
      return decoded;
    }
    case "math": {
      if (strict) {
        requireOnlyFields(record, path, ["kind", "latex", "description"]);
      }
      const decoded = {
        kind: block,
        latex: decodeString(field(record, "latex", path), `${path}.latex`),
        description: decodeNonemptyString(
          field(record, "description", path),
          `${path}.description`,
        ),
      } satisfies ContentBlock;
      return decoded;
    }
    case "image": {
      if (strict) {
        requireOnlyFields(record, path, ["kind", "asset", "description"]);
      }
      const decoded = {
        kind: block,
        asset: decodeAssetRef(field(record, "asset", path), `${path}.asset`, strict),
        description: decodeNonemptyString(
          field(record, "description", path),
          `${path}.description`,
        ),
      } satisfies ContentBlock;
      return decoded;
    }
    case "code": {
      if (strict) {
        requireOnlyFields(record, path, ["kind", "language", "source"]);
      }
      const decoded = {
        kind: block,
        language: decodeNonemptyString(field(record, "language", path), `${path}.language`),
        source: decodeString(field(record, "source", path), `${path}.source`),
      } satisfies ContentBlock;
      return decoded;
    }
    case "table": {
      if (strict) {
        requireOnlyFields(record, path, ["kind", "headers", "rows", "description"]);
      }
      const decoded = {
        kind: block,
        headers: decodeArray(field(record, "headers", path), `${path}.headers`, decodeString),
        rows: decodeArray(field(record, "rows", path), `${path}.rows`, (row, rowPath) =>
          decodeArray(row, rowPath, decodeString),
        ),
        description: decodeNonemptyString(
          field(record, "description", path),
          `${path}.description`,
        ),
      } satisfies ContentBlock;
      return decoded;
    }
    default:
      throw new DecodeError(`${path}.kind`, "a known content-block kind");
  }
}

/** Strict key-free preview projection shared by the local WASM boundary. */
export function decodeKeyFreeDraftPreview(
  value: unknown,
  path = "wasmPreview",
): {
  workspace: string;
  seed: number;
  title: string;
  prompt: ReadonlyArray<ContentBlock>;
  response: ResponseDefinition;
} {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["workspace", "seed", "title", "prompt", "response"]);
  return {
    workspace: decodeIdentifier(field(record, "workspace", path), `${path}.workspace`),
    seed: decodeNonnegativeInteger(field(record, "seed", path), `${path}.seed`),
    title: decodeNonemptyString(field(record, "title", path), `${path}.title`),
    prompt: decodeArray(field(record, "prompt", path), `${path}.prompt`, (block, blockPath) =>
      decodeContentBlock(block, blockPath, true),
    ),
    response: decodeResponseDefinition(field(record, "response", path), `${path}.response`, true),
  };
}

function decodeChoiceOption(value: unknown, path: string, strict = false): ChoiceOption {
  const record = decodeRecord(value, path);
  if (strict) {
    requireOnlyFields(record, path, ["id", "body"]);
  }
  const decoded = {
    id: decodeNonemptyString(field(record, "id", path), `${path}.id`),
    body: decodeArray(field(record, "body", path), `${path}.body`, (block, blockPath) =>
      decodeContentBlock(block, blockPath, strict),
    ),
  } satisfies ChoiceOption;
  return decoded;
}

function decodeNumericTolerance(value: unknown, path: string, strict = false): NumericTolerance {
  const record = decodeRecord(value, path);
  const tolerance = kind(record, path);
  switch (tolerance) {
    case "exact":
      if (strict) {
        requireOnlyFields(record, path, ["kind"]);
      }
      return { kind: tolerance };
    case "absolute": {
      if (strict) {
        requireOnlyFields(record, path, ["kind", "epsilon"]);
      }
      const decoded = {
        kind: tolerance,
        epsilon: decodeFiniteNumber(field(record, "epsilon", path), `${path}.epsilon`),
      } satisfies NumericTolerance;
      return decoded;
    }
    case "relative": {
      if (strict) {
        requireOnlyFields(record, path, ["kind", "fraction"]);
      }
      const decoded = {
        kind: tolerance,
        fraction: decodeFiniteNumber(field(record, "fraction", path), `${path}.fraction`),
      } satisfies NumericTolerance;
      return decoded;
    }
    case "significantFigures": {
      if (strict) {
        requireOnlyFields(record, path, ["kind", "digits"]);
      }
      const decoded = {
        kind: tolerance,
        digits: decodePositiveInteger(field(record, "digits", path), `${path}.digits`),
      } satisfies NumericTolerance;
      return decoded;
    }
    default:
      throw new DecodeError(`${path}.kind`, "a known numeric tolerance");
  }
}

function decodeSelectionCardinality(
  value: unknown,
  path: string,
  strict = false,
): SelectionCardinality {
  const record = decodeRecord(value, path);
  const selection = kind(record, path);
  switch (selection) {
    case "exactlyOne":
    case "anyNumber":
    case "atLeastOne":
      if (strict) {
        requireOnlyFields(record, path, ["kind"]);
      }
      return { kind: selection };
    case "exactly": {
      if (strict) {
        requireOnlyFields(record, path, ["kind", "count"]);
      }
      const decoded = {
        kind: selection,
        count: decodeNonnegativeInteger(field(record, "count", path), `${path}.count`),
      } satisfies SelectionCardinality;
      return decoded;
    }
    default:
      throw new DecodeError(`${path}.kind`, "a known selection cardinality");
  }
}

function decodeResponseDefinition(
  value: unknown,
  path: string,
  strict = false,
): ResponseDefinition {
  const record = decodeRecord(value, path);
  const response = kind(record, path);
  switch (response) {
    case "numeric": {
      if (strict) {
        requireOnlyFields(record, path, ["kind", "tolerance", "unit"]);
      }
      const decoded = {
        kind: response,
        tolerance: decodeNumericTolerance(
          field(record, "tolerance", path),
          `${path}.tolerance`,
          strict,
        ),
        unit: decodeNullable(field(record, "unit", path), `${path}.unit`, decodeString),
      } satisfies ResponseDefinition;
      return decoded;
    }
    case "multipleChoice": {
      if (strict) {
        requireOnlyFields(record, path, ["kind", "choices", "selection"]);
      }
      const decoded = {
        kind: response,
        choices: decodeArray(
          field(record, "choices", path),
          `${path}.choices`,
          (choice, choicePath) => decodeChoiceOption(choice, choicePath, strict),
        ),
        selection: decodeSelectionCardinality(
          field(record, "selection", path),
          `${path}.selection`,
          strict,
        ),
      } satisfies ResponseDefinition;
      return decoded;
    }
    case "shortText": {
      if (strict) {
        requireOnlyFields(record, path, ["kind", "matchMode", "maxLength"]);
      }
      const decoded = {
        kind: response,
        matchMode: decodeStringEnum(field(record, "matchMode", path), `${path}.matchMode`, [
          "exact",
          "caseInsensitive",
          "normalized",
        ]),
        maxLength: decodeNonnegativeInteger(field(record, "maxLength", path), `${path}.maxLength`),
      } satisfies ResponseDefinition;
      return decoded;
    }
    case "ordering": {
      if (strict) {
        requireOnlyFields(record, path, ["kind", "items"]);
      }
      const decoded = {
        kind: response,
        items: decodeArray(field(record, "items", path), `${path}.items`, (item, itemPath) =>
          decodeChoiceOption(item, itemPath, strict),
        ),
      } satisfies ResponseDefinition;
      return decoded;
    }
    case "fileUpload": {
      if (strict) {
        requireOnlyFields(record, path, ["kind", "maxBytes", "acceptedExtensions"]);
      }
      const decoded = {
        kind: response,
        maxBytes: decodePositiveInteger(field(record, "maxBytes", path), `${path}.maxBytes`),
        acceptedExtensions: decodeArray(
          field(record, "acceptedExtensions", path),
          `${path}.acceptedExtensions`,
          decodeNonemptyString,
        ),
      } satisfies ResponseDefinition;
      return decoded;
    }
    case "externalTool": {
      if (strict) {
        requireOnlyFields(record, path, ["kind"]);
      }
      return { kind: response } satisfies ResponseDefinition;
    }
    default:
      throw new DecodeError(`${path}.kind`, "a known response definition");
  }
}

function decodeQuestionSource(value: unknown, path: string): QuestionSource {
  const record = decodeRecord(value, path);
  const backend = decodeString(field(record, "backend", path), `${path}.backend`);
  switch (backend) {
    case "native": {
      requireOnlyFields(record, path, ["backend", "family"]);
      const decoded = {
        backend,
        family: decodeNonemptyString(field(record, "family", path), `${path}.family`),
      } satisfies QuestionSource;
      return decoded;
    }
    case "webwork": {
      requireOnlyFields(record, path, ["backend", "pgPath"]);
      const decoded = {
        backend,
        pgPath: decodeNonemptyString(field(record, "pgPath", path), `${path}.pgPath`),
      } satisfies QuestionSource;
      return decoded;
    }
    case "qti": {
      requireOnlyFields(record, path, ["backend", "itemId", "packageObject", "packageSha256"]);
      const decoded = {
        backend,
        itemId: decodeNonemptyString(field(record, "itemId", path), `${path}.itemId`),
        packageObject: decodeIdentifier(
          field(record, "packageObject", path),
          `${path}.packageObject`,
        ),
        packageSha256: decodeNonemptyString(
          field(record, "packageSha256", path),
          `${path}.packageSha256`,
        ),
      } satisfies QuestionSource;
      return decoded;
    }
    case "h5p": {
      requireOnlyFields(record, path, ["backend", "contentType"]);
      const decoded = {
        backend,
        contentType: decodeNonemptyString(
          field(record, "contentType", path),
          `${path}.contentType`,
        ),
      } satisfies QuestionSource;
      return decoded;
    }
    case "imathas": {
      requireOnlyFields(record, path, [
        "backend",
        "provider",
        "itemRef",
        "snapshot",
        "snapshotSha256",
        "integrationProfile",
      ]);
      const decoded = {
        backend,
        provider: decodeNonemptyString(field(record, "provider", path), `${path}.provider`),
        itemRef: decodeNonemptyString(field(record, "itemRef", path), `${path}.itemRef`),
        snapshot: decodeIdentifier(field(record, "snapshot", path), `${path}.snapshot`),
        snapshotSha256: decodeNonemptyString(
          field(record, "snapshotSha256", path),
          `${path}.snapshotSha256`,
        ),
        integrationProfile: decodeNonemptyString(
          field(record, "integrationProfile", path),
          `${path}.integrationProfile`,
        ),
      } satisfies QuestionSource;
      return decoded;
    }
    default:
      throw new DecodeError(`${path}.backend`, "a known question backend");
  }
}

function decodeDraftQuestionSource(value: unknown, path: string): DraftQuestionSource {
  const record = decodeRecord(value, path);
  const backend = decodeString(field(record, "backend", path), `${path}.backend`);
  switch (backend) {
    case "native":
      requireOnlyFields(record, path, ["backend", "family"]);
      return {
        backend,
        family: decodeNonemptyString(field(record, "family", path), `${path}.family`),
      } satisfies DraftQuestionSource;
    case "webwork":
      requireOnlyFields(record, path, ["backend", "pgPath"]);
      return {
        backend,
        pgPath: decodeNonemptyString(field(record, "pgPath", path), `${path}.pgPath`),
      } satisfies DraftQuestionSource;
    case "qti":
      requireOnlyFields(record, path, ["backend", "itemId", "importId"]);
      return {
        backend,
        itemId: decodeNonemptyString(field(record, "itemId", path), `${path}.itemId`),
        importId: decodeIdentifier(field(record, "importId", path), `${path}.importId`),
      } satisfies DraftQuestionSource;
    case "h5p":
      requireOnlyFields(record, path, ["backend", "contentType"]);
      return {
        backend,
        contentType: decodeNonemptyString(
          field(record, "contentType", path),
          `${path}.contentType`,
        ),
      } satisfies DraftQuestionSource;
    case "imathas":
      requireOnlyFields(record, path, ["backend", "provider", "itemRef"]);
      return {
        backend,
        provider: decodeNonemptyString(field(record, "provider", path), `${path}.provider`),
        itemRef: decodeNonemptyString(field(record, "itemRef", path), `${path}.itemRef`),
      } satisfies DraftQuestionSource;
    default:
      throw new DecodeError(`${path}.backend`, "a known draft question backend");
  }
}

function decodeGeneratorReference(
  value: unknown,
  path: string,
  strict = false,
): { id: string; version: string } {
  const record = decodeRecord(value, path);
  if (strict) {
    requireOnlyFields(record, path, ["id", "version"]);
  }
  return {
    id: decodeNonemptyString(field(record, "id", path), `${path}.id`),
    version: decodeNonemptyString(field(record, "version", path), `${path}.version`),
  };
}

function decodeParameterSpec(value: unknown, path: string, strict = false): ParameterSpec {
  const record = decodeRecord(value, path);
  const parameter = kind(record, path);
  switch (parameter) {
    case "integerRange": {
      if (strict) {
        requireOnlyFields(record, path, ["kind", "low", "high"]);
      }
      const decoded = {
        kind: parameter,
        low: decodeSafeInteger(field(record, "low", path), `${path}.low`),
        high: decodeSafeInteger(field(record, "high", path), `${path}.high`),
      } satisfies ParameterSpec;
      return decoded;
    }
    case "decimalRange": {
      if (strict) {
        requireOnlyFields(record, path, ["kind", "low", "high", "decimals"]);
      }
      const decoded = {
        kind: parameter,
        low: decodeFiniteNumber(field(record, "low", path), `${path}.low`),
        high: decodeFiniteNumber(field(record, "high", path), `${path}.high`),
        decimals: decodeNonnegativeInteger(field(record, "decimals", path), `${path}.decimals`),
      } satisfies ParameterSpec;
      return decoded;
    }
    case "choice": {
      if (strict) {
        requireOnlyFields(record, path, ["kind", "options"]);
      }
      const decoded = {
        kind: parameter,
        options: decodeArray(field(record, "options", path), `${path}.options`, decodeString),
      } satisfies ParameterSpec;
      return decoded;
    }
    case "fixed": {
      if (strict) {
        requireOnlyFields(record, path, ["kind", "value"]);
      }
      const decoded = {
        kind: parameter,
        value: decodeString(field(record, "value", path), `${path}.value`),
      } satisfies ParameterSpec;
      return decoded;
    }
    default:
      throw new DecodeError(`${path}.kind`, "a known parameter specification");
  }
}

function decodeRandomization(
  value: unknown,
  path: string,
  strict = false,
): RandomizationDefinition {
  const record = decodeRecord(value, path);
  const randomization = kind(record, path);
  switch (randomization) {
    case "static":
      if (strict) {
        requireOnlyFields(record, path, ["kind"]);
      }
      return { kind: randomization };
    case "seeded": {
      if (strict) {
        requireOnlyFields(record, path, ["kind", "generator", "parameters"]);
      }
      const decoded = {
        kind: randomization,
        generator: decodeGeneratorReference(
          field(record, "generator", path),
          `${path}.generator`,
          strict,
        ),
        parameters: decodeDictionary(
          field(record, "parameters", path),
          `${path}.parameters`,
          (parameter, parameterPath) => decodeParameterSpec(parameter, parameterPath, strict),
        ),
      } satisfies RandomizationDefinition;
      return decoded;
    }
    default:
      throw new DecodeError(`${path}.kind`, "a known randomization definition");
  }
}

function decodeAttemptPolicy(value: unknown, path: string, strict = false): AttemptPolicy {
  const record = decodeRecord(value, path);
  if (strict) {
    requireOnlyFields(record, path, ["maxAttempts", "feedback"]);
  }
  const decoded = {
    maxAttempts: decodeNullable(
      field(record, "maxAttempts", path),
      `${path}.maxAttempts`,
      decodePositiveInteger,
    ),
    feedback: decodeStringEnum(field(record, "feedback", path), `${path}.feedback`, [
      "immediateFull",
      "immediateCorrectness",
      "deferred",
      "onRelease",
    ]),
  } satisfies AttemptPolicy;
  return decoded;
}

function decodeTimingPolicy(value: unknown, path: string, strict = false): TimingPolicy {
  const record = decodeRecord(value, path);
  const timing = kind(record, path);
  switch (timing) {
    case "untimed":
      if (strict) {
        requireOnlyFields(record, path, ["kind"]);
      }
      return { kind: timing };
    case "perQuestion":
    case "perAttempt": {
      if (strict) {
        requireOnlyFields(record, path, ["kind", "seconds", "graceSeconds"]);
      }
      const decoded = {
        kind: timing,
        seconds: decodePositiveInteger(field(record, "seconds", path), `${path}.seconds`),
        graceSeconds: decodeNonnegativeInteger(
          field(record, "graceSeconds", path),
          `${path}.graceSeconds`,
        ),
      } satisfies TimingPolicy;
      return decoded;
    }
    default:
      throw new DecodeError(`${path}.kind`, "a known timing policy");
  }
}

function decodeGradingDefinition(value: unknown, path: string, strict = false): GradingDefinition {
  const record = decodeRecord(value, path);
  const mode = decodeString(field(record, "mode", path), `${path}.mode`);
  switch (mode) {
    case "allOrNothing":
    case "partialCredit": {
      if (strict) {
        requireOnlyFields(record, path, ["mode", "points"]);
      }
      const decoded = {
        mode,
        points: decodeFiniteNumber(field(record, "points", path), `${path}.points`),
      } satisfies GradingDefinition;
      return decoded;
    }
    case "ungraded":
      if (strict) {
        requireOnlyFields(record, path, ["mode"]);
      }
      return { mode };
    default:
      throw new DecodeError(`${path}.mode`, "a known grading definition");
  }
}

export function decodeQuestionDefinition(value: unknown, path = "response"): QuestionDefinition {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "problem",
    "version",
    "workspace",
    "source",
    "prompt",
    "response",
    "attemptPolicy",
    "timingPolicy",
    "randomization",
    "grading",
    "metadata",
  ]);
  const decoded = {
    problem: decodeIdentifier(field(record, "problem", path), `${path}.problem`),
    version: decodeIdentifier(field(record, "version", path), `${path}.version`),
    ...decodeQuestionContent(record, path),
  } satisfies QuestionDefinition;
  return decoded;
}

/** Strictly decodes editable content, which cannot carry published IDs. */
export function decodeDraftQuestionDefinition(
  value: unknown,
  path = "response",
): DraftQuestionDefinition {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "workspace",
    "source",
    "prompt",
    "response",
    "attemptPolicy",
    "timingPolicy",
    "randomization",
    "grading",
    "metadata",
  ]);
  return decodeDraftQuestionContent(record, path);
}

/** Strict compact projection for a tenant-owned, unversioned workspace draft. */
export function decodeWorkspaceDraftSummary(
  value: unknown,
  path = "response",
): WorkspaceDraftSummary {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["workspace", "title", "sourceBackend"]);
  return {
    workspace: decodeIdentifier(field(record, "workspace", path), `${path}.workspace`),
    title: decodeEnvelopeTitle(field(record, "title", path), `${path}.title`),
    sourceBackend: decodeStringEnum(
      field(record, "sourceBackend", path),
      `${path}.sourceBackend`,
      QUESTION_BACKENDS,
    ),
  } satisfies WorkspaceDraftSummary;
}

export function decodeWorkspaceDraftPage(
  value: unknown,
  path = "response",
): {
  readonly items: ReadonlyArray<WorkspaceDraftSummary>;
  readonly nextCursor: string | null;
} {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["items", "nextCursor"]);
  const items = decodeArray(field(record, "items", path), `${path}.items`, (item, itemPath) =>
    decodeWorkspaceDraftSummary(item, itemPath),
  );
  if (items.length > 100) {
    throw new DecodeError(`${path}.items`, "at most 100 workspace summaries");
  }
  const nextCursor = decodeNullable(
    field(record, "nextCursor", path),
    `${path}.nextCursor`,
    decodeString,
  );
  if (nextCursor !== null && (nextCursor.length === 0 || nextCursor.length > MAX_CURSOR_LENGTH)) {
    throw new DecodeError(`${path}.nextCursor`, "a cursor of 1 through 512 characters or null");
  }
  return { items, nextCursor };
}

const PUBLICATION_FIELDS = [
  "sourceBackend",
  "title",
  "prompt",
  "response",
  "attemptPolicy",
  "timingPolicy",
  "randomization",
  "metadata",
] as const;

export function decodePublicationValidationReport(
  value: unknown,
  path = "response",
): PublicationValidationReport {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["violations"]);
  return {
    violations: decodeBoundedArray(
      field(record, "violations", path),
      `${path}.violations`,
      MAX_PUBLICATION_SEMANTIC_ENTRIES,
      (entry, entryPath) => {
        const violation = decodeRecord(entry, entryPath);
        requireOnlyFields(violation, entryPath, ["workspace", "title", "capability"]);
        return {
          workspace: decodeIdentifier(
            field(violation, "workspace", entryPath),
            `${entryPath}.workspace`,
          ),
          title: decodeEnvelopeTitle(field(violation, "title", entryPath), `${entryPath}.title`),
          capability: decodeCapability(
            field(violation, "capability", entryPath),
            `${entryPath}.capability`,
          ),
        };
      },
    ),
  };
}

/** Exact readiness-only 422 body. Capability failures use a distinct violations shape. */
export function decodePublicationReadinessFailure(
  value: unknown,
  path = "response",
): PublicationReadinessFailure {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["error"]);
  return {
    kind: "readinessFailure",
    message: decodeNonemptyString(field(record, "error", path), `${path}.error`),
  };
}

/** Exact publish 422 body: the message and every capability violation are retained. */
export function decodePublicationValidationFailure(
  value: unknown,
  path = "response",
): { readonly message: string; readonly violations: ReadonlyArray<PublicationViolation> } {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["error", "violations"]);
  const report = decodePublicationValidationReport(
    { violations: field(record, "violations", path) },
    path,
  );
  return {
    message: decodeNonemptyString(field(record, "error", path), `${path}.error`),
    violations: report.violations,
  };
}

function decodePublicationSemanticProjection(
  value: unknown,
  path: string,
): PublicationSemanticProjection {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "sourceBackend",
    "title",
    "prompt",
    "response",
    "attemptPolicy",
    "timingPolicy",
    "randomization",
    "metadata",
  ]);
  const prompt = decodeRecord(field(record, "prompt", path), `${path}.prompt`);
  requireOnlyFields(prompt, `${path}.prompt`, ["blocks"]);
  const response = decodeRecord(field(record, "response", path), `${path}.response`);
  requireOnlyFields(response, `${path}.response`, ["kind", "optionCount"]);
  const responseKind = decodeStringEnum(
    field(response, "kind", `${path}.response`),
    `${path}.response.kind`,
    ["numeric", "multipleChoice", "shortText", "ordering", "fileUpload", "externalTool"],
  );
  const optionCount = decodeNullable(
    field(response, "optionCount", `${path}.response`),
    `${path}.response.optionCount`,
    decodeNonnegativeInteger,
  );
  if (
    (responseKind === "multipleChoice" || responseKind === "ordering") !==
    (optionCount !== null)
  ) {
    throw new DecodeError(
      `${path}.response.optionCount`,
      "present only for option-based responses",
    );
  }
  const randomization = decodeRecord(field(record, "randomization", path), `${path}.randomization`);
  requireOnlyFields(randomization, `${path}.randomization`, ["kind"]);
  const metadata = decodeRecord(field(record, "metadata", path), `${path}.metadata`);
  requireOnlyFields(metadata, `${path}.metadata`, ["tags", "taxonomy", "license", "language"]);
  return {
    sourceBackend: decodeStringEnum(
      field(record, "sourceBackend", path),
      `${path}.sourceBackend`,
      QUESTION_BACKENDS,
    ),
    title: decodeEnvelopeTitle(field(record, "title", path), `${path}.title`),
    prompt: {
      blocks: decodeBoundedArray(
        field(prompt, "blocks", `${path}.prompt`),
        `${path}.prompt.blocks`,
        MAX_PUBLICATION_SEMANTIC_ENTRIES,
        (block, blockPath) =>
          decodeStringEnum(block, blockPath, ["text", "math", "image", "code", "table"]),
      ),
    },
    response: { kind: responseKind, optionCount },
    attemptPolicy: decodeAttemptPolicy(
      field(record, "attemptPolicy", path),
      `${path}.attemptPolicy`,
      true,
    ),
    timingPolicy: decodeTimingPolicy(
      field(record, "timingPolicy", path),
      `${path}.timingPolicy`,
      true,
    ),
    randomization: {
      kind: decodeStringEnum(
        field(randomization, "kind", `${path}.randomization`),
        `${path}.randomization.kind`,
        ["static", "seeded"],
      ),
    },
    metadata: {
      tags: decodeBoundedArray(
        field(metadata, "tags", `${path}.metadata`),
        `${path}.metadata.tags`,
        MAX_PUBLICATION_SEMANTIC_ENTRIES,
        decodeString,
      ),
      taxonomy: decodeBoundedArray(
        field(metadata, "taxonomy", `${path}.metadata`),
        `${path}.metadata.taxonomy`,
        MAX_PUBLICATION_SEMANTIC_ENTRIES,
        (term, termPath) => decodeTaxonomyTerm(term, termPath, true),
      ),
      license: decodeLicense(
        field(metadata, "license", `${path}.metadata`),
        `${path}.metadata.license`,
        true,
      ),
      language: decodeNonemptyString(
        field(metadata, "language", `${path}.metadata`),
        `${path}.metadata.language`,
      ),
    },
  };
}

export function decodePublicationDiff(value: unknown, path = "response"): PublicationDiff {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "draftRevision",
    "baseline",
    "prior",
    "previous",
    "current",
    "changed",
  ]);
  const draftRevision = decodePositiveInteger(
    field(record, "draftRevision", path),
    `${path}.draftRevision`,
  );
  const baseline = decodeStringEnum(field(record, "baseline", path), `${path}.baseline`, [
    "firstPublication",
    "revision",
  ] as const);
  const prior = decodeNullable(
    field(record, "prior", path),
    `${path}.prior`,
    decodeProblemVersionRef,
  );
  const previous = decodeNullable(
    field(record, "previous", path),
    `${path}.previous`,
    decodePublicationSemanticProjection,
  );
  const current = decodePublicationSemanticProjection(
    field(record, "current", path),
    `${path}.current`,
  );
  if (
    baseline === "firstPublication"
      ? prior !== null || previous !== null
      : prior === null || previous === null
  ) {
    throw new DecodeError(
      `${path}.prior`,
      "a baseline-consistent immutable reference and semantic predecessor",
    );
  }
  const changed = decodeBoundedArray(
    field(record, "changed", path),
    `${path}.changed`,
    PUBLICATION_FIELDS.length,
    (entry, entryPath) => decodeStringEnum(entry, entryPath, PUBLICATION_FIELDS),
  );
  if (
    new Set(changed).size !== changed.length ||
    (baseline === "firstPublication" && changed.length !== 0)
  ) {
    throw new DecodeError(`${path}.changed`, "unique baseline-consistent semantic fields");
  }
  return {
    draftRevision,
    revision: `"${draftRevision}"`,
    baseline,
    prior,
    previous,
    current,
    changed,
  };
}

export function decodePublicationResult(value: unknown, path = "response"): PublicationResult {
  const definition = decodeQuestionDefinition(value, path);
  return { reference: { problem: definition.problem, version: definition.version } };
}

function decodeQuestionContent(
  record: Record<string, unknown>,
  path: string,
): Omit<QuestionDefinition, "problem" | "version"> {
  return {
    workspace: decodeIdentifier(field(record, "workspace", path), `${path}.workspace`),
    source: decodeQuestionSource(field(record, "source", path), `${path}.source`),
    prompt: decodeArray(field(record, "prompt", path), `${path}.prompt`, (block, blockPath) =>
      decodeContentBlock(block, blockPath, true),
    ),
    response: decodeResponseDefinition(field(record, "response", path), `${path}.response`, true),
    attemptPolicy: decodeAttemptPolicy(
      field(record, "attemptPolicy", path),
      `${path}.attemptPolicy`,
      true,
    ),
    timingPolicy: decodeTimingPolicy(
      field(record, "timingPolicy", path),
      `${path}.timingPolicy`,
      true,
    ),
    randomization: decodeRandomization(
      field(record, "randomization", path),
      `${path}.randomization`,
      true,
    ),
    grading: decodeGradingDefinition(field(record, "grading", path), `${path}.grading`, true),
    metadata: decodeQuestionMetadata(field(record, "metadata", path), `${path}.metadata`, true),
  } satisfies Omit<QuestionDefinition, "problem" | "version">;
}

function decodeDraftQuestionContent(
  record: Record<string, unknown>,
  path: string,
): DraftQuestionDefinition {
  return {
    workspace: decodeIdentifier(field(record, "workspace", path), `${path}.workspace`),
    source: decodeDraftQuestionSource(field(record, "source", path), `${path}.source`),
    prompt: decodeArray(field(record, "prompt", path), `${path}.prompt`, (block, blockPath) =>
      decodeContentBlock(block, blockPath, true),
    ),
    response: decodeResponseDefinition(field(record, "response", path), `${path}.response`, true),
    attemptPolicy: decodeAttemptPolicy(
      field(record, "attemptPolicy", path),
      `${path}.attemptPolicy`,
      true,
    ),
    timingPolicy: decodeTimingPolicy(
      field(record, "timingPolicy", path),
      `${path}.timingPolicy`,
      true,
    ),
    randomization: decodeRandomization(
      field(record, "randomization", path),
      `${path}.randomization`,
      true,
    ),
    grading: decodeGradingDefinition(field(record, "grading", path), `${path}.grading`, true),
    metadata: decodeQuestionMetadata(field(record, "metadata", path), `${path}.metadata`, true),
  } satisfies DraftQuestionDefinition;
}

/** Strictly decodes the key-free rendered variant delivered for an attempt. */
export function decodeQuestionEnvelope(value: unknown, path = "response"): QuestionEnvelope {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["version", "seed", "title", "prompt", "response"]);
  const decoded = {
    version: decodeIdentifier(field(record, "version", path), `${path}.version`),
    seed: decodeNonnegativeInteger(field(record, "seed", path), `${path}.seed`),
    title: decodeEnvelopeTitle(field(record, "title", path), `${path}.title`),
    prompt: decodeArray(field(record, "prompt", path), `${path}.prompt`, (block, blockPath) =>
      decodeContentBlock(block, blockPath, true),
    ),
    response: decodeResponseDefinition(field(record, "response", path), `${path}.response`, true),
  } satisfies QuestionEnvelope;
  return decoded;
}

/** Decodes the route-only external-tool broker projection. */
export function decodeExternalToolLaunch(value: unknown, path = "response"): ExternalToolLaunch {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["launchUrl"]);
  const launchUrl = decodeNonemptyString(field(record, "launchUrl", path), `${path}.launchUrl`);
  const placeholderOrigin = "https://ple-invalid.example";
  let parsed: URL;
  try {
    parsed = new URL(launchUrl, placeholderOrigin);
  } catch {
    throw new DecodeError(`${path}.launchUrl`, "a same-origin absolute path");
  }
  if (
    !launchUrl.startsWith("/") ||
    launchUrl.startsWith("//") ||
    launchUrl.includes("?") ||
    launchUrl.includes("#") ||
    parsed.origin !== placeholderOrigin ||
    parsed.username !== "" ||
    parsed.password !== "" ||
    parsed.search !== "" ||
    parsed.hash !== ""
  ) {
    throw new DecodeError(
      `${path}.launchUrl`,
      "a same-origin absolute path without query or fragment",
    );
  }
  return { launchUrl };
}

/** Strict outbound and inbound student-response boundary. */
export function decodeStudentResponse(value: unknown, path = "response"): StudentResponse {
  const record = decodeRecord(value, path);
  const response = kind(record, path);
  switch (response) {
    case "numeric": {
      requireOnlyFields(record, path, ["kind", "value"]);
      const decoded = {
        kind: response,
        value: decodeFiniteNumber(field(record, "value", path), `${path}.value`),
      } satisfies StudentResponse;
      return decoded;
    }
    case "multipleChoice": {
      requireOnlyFields(record, path, ["kind", "selected"]);
      const decoded = {
        kind: response,
        selected: decodeArray(
          field(record, "selected", path),
          `${path}.selected`,
          decodeNonemptyString,
        ),
      } satisfies StudentResponse;
      return decoded;
    }
    case "shortText": {
      requireOnlyFields(record, path, ["kind", "text"]);
      const decoded = {
        kind: response,
        text: decodeString(field(record, "text", path), `${path}.text`),
      } satisfies StudentResponse;
      return decoded;
    }
    case "ordering": {
      requireOnlyFields(record, path, ["kind", "order"]);
      const decoded = {
        kind: response,
        order: decodeArray(field(record, "order", path), `${path}.order`, decodeNonemptyString),
      } satisfies StudentResponse;
      return decoded;
    }
    case "fileUpload": {
      requireOnlyFields(record, path, ["kind", "objectKey"]);
      const decoded = {
        kind: response,
        objectKey: decodeNonemptyString(field(record, "objectKey", path), `${path}.objectKey`),
      } satisfies StudentResponse;
      return decoded;
    }
    case "externalTool":
      requireOnlyFields(record, path, ["kind"]);
      return { kind: response } satisfies StudentResponse;
    default:
      throw new DecodeError(`${path}.kind`, "a known student-response kind");
  }
}

function decodeAttemptResult(value: unknown, path: string): AttemptResult {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["correct", "pointsEarned", "pointsPossible"]);
  const decoded = {
    correct: decodeBoolean(field(record, "correct", path), `${path}.correct`),
    pointsEarned: decodeFiniteNumber(field(record, "pointsEarned", path), `${path}.pointsEarned`),
    pointsPossible: decodeFiniteNumber(
      field(record, "pointsPossible", path),
      `${path}.pointsPossible`,
    ),
  } satisfies AttemptResult;
  return decoded;
}

/**
 * Decodes the server's already-redacted teaching projection.
 *
 * Every field is optional because absence is a security property: a client
 * must reject unknown properties rather than silently retaining a provider
 * transcript, key, or other server-private material.
 */
export function decodeDisclosedFeedback(value: unknown, path = "response"): DisclosedFeedback {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "correctness",
    "pointsEarned",
    "pointsPossible",
    "hint",
    "correctResponse",
    "rationale",
  ]);
  const correctness =
    "correctness" in record
      ? decodeBoolean(field(record, "correctness", path), `${path}.correctness`)
      : undefined;
  const pointsEarned =
    "pointsEarned" in record
      ? decodeFiniteNumber(field(record, "pointsEarned", path), `${path}.pointsEarned`)
      : undefined;
  const pointsPossible =
    "pointsPossible" in record
      ? decodeFiniteNumber(field(record, "pointsPossible", path), `${path}.pointsPossible`)
      : undefined;
  const hint =
    "hint" in record
      ? decodeArray(field(record, "hint", path), `${path}.hint`, (block, blockPath) =>
          decodeContentBlock(block, blockPath, true),
        )
      : undefined;
  const correctResponse =
    "correctResponse" in record
      ? decodeArray(
          field(record, "correctResponse", path),
          `${path}.correctResponse`,
          (block, blockPath) => decodeContentBlock(block, blockPath, true),
        )
      : undefined;
  const rationale =
    "rationale" in record
      ? decodeArray(field(record, "rationale", path), `${path}.rationale`, (block, blockPath) =>
          decodeContentBlock(block, blockPath, true),
        )
      : undefined;
  return {
    ...(correctness === undefined ? {} : { correctness }),
    ...(pointsEarned === undefined ? {} : { pointsEarned }),
    ...(pointsPossible === undefined ? {} : { pointsPossible }),
    ...(hint === undefined ? {} : { hint }),
    ...(correctResponse === undefined ? {} : { correctResponse }),
    ...(rationale === undefined ? {} : { rationale }),
  } satisfies DisclosedFeedback;
}

function decodeAttemptTimer(value: unknown, path: string): AttemptTimerRecord {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["issuedAt", "deadline", "submittedAt"]);
  const decoded = {
    issuedAt: decodeTimestamp(field(record, "issuedAt", path), `${path}.issuedAt`),
    deadline: decodeNullable(field(record, "deadline", path), `${path}.deadline`, decodeTimestamp),
    submittedAt: decodeNullable(
      field(record, "submittedAt", path),
      `${path}.submittedAt`,
      decodeTimestamp,
    ),
  } satisfies AttemptTimerRecord;
  return decoded;
}

function decodeImplementationVersion(
  value: unknown,
  path: string,
): { id: string; version: string } {
  return decodeGeneratorReference(value, path, true);
}

function decodeSourceArtifact(value: unknown, path: string): SourceArtifact {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["object", "sha256"]);
  const decoded = {
    object: decodeIdentifier(field(record, "object", path), `${path}.object`),
    sha256: decodeSha256(field(record, "sha256", path), `${path}.sha256`),
  } satisfies SourceArtifact;
  return decoded;
}

function decodeAttemptProvenance(value: unknown, path: string): AttemptProvenance {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "adapter",
    "renderer",
    "generator",
    "sourceArtifact",
    "assetObjects",
    "grading",
    "renderedQuestionSha256",
  ]);
  const decoded = {
    adapter: decodeImplementationVersion(field(record, "adapter", path), `${path}.adapter`),
    renderer: decodeNullable(
      field(record, "renderer", path),
      `${path}.renderer`,
      decodeImplementationVersion,
    ),
    generator: decodeNullable(
      field(record, "generator", path),
      `${path}.generator`,
      (generator, generatorPath) => decodeGeneratorReference(generator, generatorPath, true),
    ),
    sourceArtifact: decodeNullable(
      field(record, "sourceArtifact", path),
      `${path}.sourceArtifact`,
      decodeSourceArtifact,
    ),
    assetObjects: decodeArray(
      field(record, "assetObjects", path),
      `${path}.assetObjects`,
      decodeIdentifier,
    ),
    grading: decodeImplementationVersion(field(record, "grading", path), `${path}.grading`),
    renderedQuestionSha256: decodeSha256(
      field(record, "renderedQuestionSha256", path),
      `${path}.renderedQuestionSha256`,
    ),
  } satisfies AttemptProvenance;
  return decoded;
}

export function decodeQuestionAttempt(value: unknown, path = "response"): QuestionAttempt {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "id",
    "tenant",
    "run",
    "problem",
    "questionVersion",
    "assignmentPosition",
    "seed",
    "parameterHash",
    "response",
    "status",
    "result",
    "timer",
    "provenance",
  ]);
  const decoded = {
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    tenant: decodeIdentifier(field(record, "tenant", path), `${path}.tenant`),
    run: decodeIdentifier(field(record, "run", path), `${path}.run`),
    problem: decodeIdentifier(field(record, "problem", path), `${path}.problem`),
    questionVersion: decodeIdentifier(
      field(record, "questionVersion", path),
      `${path}.questionVersion`,
    ),
    assignmentPosition: decodeNonnegativeInteger(
      field(record, "assignmentPosition", path),
      `${path}.assignmentPosition`,
    ),
    seed: decodeNonnegativeInteger(field(record, "seed", path), `${path}.seed`),
    parameterHash: decodeSha256(field(record, "parameterHash", path), `${path}.parameterHash`),
    response: decodeNullable(
      field(record, "response", path),
      `${path}.response`,
      decodeStudentResponse,
    ),
    status: decodeStringEnum(field(record, "status", path), `${path}.status`, [
      "in_progress",
      "submitted",
      "auto_submitted",
      "needs_manual_grading",
      "cleared",
      "exempt",
    ] as const satisfies ReadonlyArray<AttemptStatus>),
    result: decodeNullable(field(record, "result", path), `${path}.result`, decodeAttemptResult),
    timer: decodeAttemptTimer(field(record, "timer", path), `${path}.timer`),
    provenance: decodeAttemptProvenance(field(record, "provenance", path), `${path}.provenance`),
  } satisfies QuestionAttempt;
  return decoded;
}

export function decodeAssignmentRun(value: unknown, path = "response"): AssignmentRun {
  const record = decodeRecord(value, path);
  const decoded = {
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    tenant: decodeIdentifier(field(record, "tenant", path), `${path}.tenant`),
    enrollment: decodeIdentifier(field(record, "enrollment", path), `${path}.enrollment`),
    runNumber: decodePositiveInteger(field(record, "runNumber", path), `${path}.runNumber`),
    startedAt: decodeTimestamp(field(record, "startedAt", path), `${path}.startedAt`),
    completedAt: decodeNullable(
      field(record, "completedAt", path),
      `${path}.completedAt`,
      decodeTimestamp,
    ),
    score: decodeNullable(field(record, "score", path), `${path}.score`, decodeFiniteNumber),
    mode: decodeStringEnum(field(record, "mode", path), `${path}.mode`, ["assigned", "practice"]),
    variation: decodeStringEnum(field(record, "variation", path), `${path}.variation`, [
      "newSeeds",
      "selectedProblemVariants",
      "fullRegeneration",
    ]),
  } satisfies AssignmentRun;
  return decoded;
}

function decodeStrictAssignmentRun(value: unknown, path: string): AssignmentRun {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "id",
    "tenant",
    "enrollment",
    "runNumber",
    "startedAt",
    "completedAt",
    "score",
    "mode",
    "variation",
  ]);
  return decodeAssignmentRun(value, path);
}

function decodeAssignmentEnrollment(value: unknown, path: string): AssignmentEnrollment {
  const record = decodeRecord(value, path);
  const decoded = {
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    tenant: decodeIdentifier(field(record, "tenant", path), `${path}.tenant`),
    assignment: decodeIdentifier(field(record, "assignment", path), `${path}.assignment`),
    user: decodeIdentifier(field(record, "user", path), `${path}.user`),
    student: decodeIdentifier(field(record, "student", path), `${path}.student`),
    firstCompletedAt: decodeNullable(
      field(record, "firstCompletedAt", path),
      `${path}.firstCompletedAt`,
      decodeTimestamp,
    ),
    currentGradeRun: decodeNullable(
      field(record, "currentGradeRun", path),
      `${path}.currentGradeRun`,
      decodeIdentifier,
    ),
    bestGradeRun: decodeNullable(
      field(record, "bestGradeRun", path),
      `${path}.bestGradeRun`,
      decodeIdentifier,
    ),
  } satisfies AssignmentEnrollment;
  return decoded;
}

export function decodeStudentAssignmentSummary(
  value: unknown,
  path = "response",
): StudentAssignmentSummary {
  const record = decodeRecord(value, path);
  const decoded = {
    tenant: decodeIdentifier(field(record, "tenant", path), `${path}.tenant`),
    enrollment: decodeIdentifier(field(record, "enrollment", path), `${path}.enrollment`),
    currentScore: decodeNullable(
      field(record, "currentScore", path),
      `${path}.currentScore`,
      decodeFiniteNumber,
    ),
    bestScore: decodeNullable(
      field(record, "bestScore", path),
      `${path}.bestScore`,
      decodeFiniteNumber,
    ),
    latestScore: decodeNullable(
      field(record, "latestScore", path),
      `${path}.latestScore`,
      decodeFiniteNumber,
    ),
    completedRunCount: decodeNonnegativeInteger(
      field(record, "completedRunCount", path),
      `${path}.completedRunCount`,
    ),
    totalQuestionAttempts: decodeNonnegativeInteger(
      field(record, "totalQuestionAttempts", path),
      `${path}.totalQuestionAttempts`,
    ),
    lastActivityAt: decodeNullable(
      field(record, "lastActivityAt", path),
      `${path}.lastActivityAt`,
      decodeTimestamp,
    ),
  } satisfies StudentAssignmentSummary;
  return decoded;
}

function decodeStrictStudentAssignmentSummary(
  value: unknown,
  path: string,
): StudentAssignmentSummary {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "tenant",
    "enrollment",
    "currentScore",
    "bestScore",
    "latestScore",
    "completedRunCount",
    "totalQuestionAttempts",
    "lastActivityAt",
  ]);
  return decodeStudentAssignmentSummary(value, path);
}

function decodeRunSummaryOutcome(value: unknown, path: string): RunSummaryOutcome {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "attempt",
    "assignmentPosition",
    "submittedAt",
    "response",
    "feedback",
  ]);
  return {
    attempt: decodeIdentifier(field(record, "attempt", path), `${path}.attempt`),
    assignmentPosition: decodeNonnegativeInteger(
      field(record, "assignmentPosition", path),
      `${path}.assignmentPosition`,
    ),
    submittedAt: decodeNullable(
      field(record, "submittedAt", path),
      `${path}.submittedAt`,
      decodeTimestamp,
    ),
    response: decodeNullable(
      field(record, "response", path),
      `${path}.response`,
      decodeStudentResponse,
    ),
    feedback: decodeNullable(
      field(record, "feedback", path),
      `${path}.feedback`,
      decodeDisclosedFeedback,
    ),
  } satisfies RunSummaryOutcome;
}

export function decodeRunSummaryResponse(value: unknown, path = "response"): RunSummaryResponse {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["run", "summary", "practiceAllowed", "outcomes"]);
  const outcomes = decodeRecord(field(record, "outcomes", path), `${path}.outcomes`);
  requireOnlyFields(outcomes, `${path}.outcomes`, ["items", "nextCursor"]);
  const decoded = {
    run: decodeStrictAssignmentRun(field(record, "run", path), `${path}.run`),
    summary: decodeStrictStudentAssignmentSummary(
      field(record, "summary", path),
      `${path}.summary`,
    ),
    practiceAllowed: decodeBoolean(
      field(record, "practiceAllowed", path),
      `${path}.practiceAllowed`,
    ),
    outcomes: {
      items: decodeArray(
        field(outcomes, "items", `${path}.outcomes`),
        `${path}.outcomes.items`,
        decodeRunSummaryOutcome,
      ),
      nextCursor: decodeNullable(
        field(outcomes, "nextCursor", `${path}.outcomes`),
        `${path}.outcomes.nextCursor`,
        (cursor, cursorPath) => {
          const decoded = decodeNonemptyString(cursor, cursorPath);
          if (decoded.length > MAX_CURSOR_LENGTH)
            throw new DecodeError(
              cursorPath,
              `a cursor no longer than ${MAX_CURSOR_LENGTH} characters`,
            );
          return decoded;
        },
      ),
    },
  } satisfies RunSummaryResponse;
  if (
    decoded.run.tenant !== decoded.summary.tenant ||
    decoded.run.enrollment !== decoded.summary.enrollment
  ) {
    throw new DecodeError(path, "a run and summary owned by the same tenant enrollment");
  }
  return decoded;
}

export function decodeFeedbackReleaseResponse(
  value: unknown,
  path = "response",
): FeedbackReleaseResponse {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["released"]);
  return { released: decodeTrue(field(record, "released", path), `${path}.released`) };
}

/**
 * Decodes the gradebook's deliberately compact, tenant-owned projection.
 *
 * This boundary is exact because browser gradebook consumers must not silently
 * accept history, question content, or a cross-tenant record appended by a
 * future server regression.
 */
export function decodeGradebookSummaryRow(value: unknown, path = "response"): GradebookSummaryRow {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "tenant",
    "courseId",
    "enrollmentId",
    "studentId",
    "assignmentId",
    "assignmentTitle",
    "summary",
  ]);
  const tenant = decodeIdentifier(field(record, "tenant", path), `${path}.tenant`);
  const summary = decodeStudentAssignmentSummary(field(record, "summary", path), `${path}.summary`);
  const summaryRecord = decodeRecord(field(record, "summary", path), `${path}.summary`);
  requireOnlyFields(summaryRecord, `${path}.summary`, [
    "tenant",
    "enrollment",
    "currentScore",
    "bestScore",
    "latestScore",
    "completedRunCount",
    "totalQuestionAttempts",
    "lastActivityAt",
  ]);
  const enrollmentId = decodeIdentifier(
    field(record, "enrollmentId", path),
    `${path}.enrollmentId`,
  );
  if (summary.tenant !== tenant) {
    throw new DecodeError(`${path}.summary.tenant`, "the row tenant");
  }
  if (summary.enrollment !== enrollmentId) {
    throw new DecodeError(`${path}.summary.enrollment`, "the row enrollmentId");
  }
  const decoded = {
    tenant,
    courseId: decodeIdentifier(field(record, "courseId", path), `${path}.courseId`),
    enrollmentId,
    studentId: decodeIdentifier(field(record, "studentId", path), `${path}.studentId`),
    assignmentId: decodeIdentifier(field(record, "assignmentId", path), `${path}.assignmentId`),
    assignmentTitle: decodeNonemptyString(
      field(record, "assignmentTitle", path),
      `${path}.assignmentTitle`,
    ),
    summary,
  } satisfies GradebookSummaryRow;
  return decoded;
}

export function decodeAuthSession(value: unknown, path = "response"): AuthSession {
  const record = decodeRecord(value, path);
  const user = decodeRecord(field(record, "user", path), `${path}.user`);
  const decoded = {
    authenticated: decodeTrue(field(record, "authenticated", path), `${path}.authenticated`),
    tenant: decodeIdentifier(field(record, "tenant", path), `${path}.tenant`),
    user: {
      id: decodeIdentifier(field(user, "id", `${path}.user`), `${path}.user.id`),
      displayName: decodeNonemptyString(
        field(user, "displayName", `${path}.user`),
        `${path}.user.displayName`,
      ),
      roles: decodeArray(field(user, "roles", `${path}.user`), `${path}.user.roles`, (role, p) =>
        decodeStringEnum(role, p, ["student", "instructor", "publisher", "administrator"]),
      ),
    },
  } satisfies AuthSession;
  return decoded;
}

export function decodeEnrollmentView(value: unknown, path = "response"): EnrollmentView {
  const record = decodeRecord(value, path);
  const decoded = {
    enrollment: decodeAssignmentEnrollment(field(record, "enrollment", path), `${path}.enrollment`),
    summary: decodeStudentAssignmentSummary(field(record, "summary", path), `${path}.summary`),
  } satisfies EnrollmentView;
  return decoded;
}

export function decodeSubmissionReceipt(value: unknown, path = "response"): SubmissionReceipt {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["accepted", "attempt", "feedback", "nextIssued"]);
  const decoded = {
    accepted: decodeTrue(field(record, "accepted", path), `${path}.accepted`),
    attempt: decodeQuestionAttempt(field(record, "attempt", path), `${path}.attempt`),
    feedback: decodeNullable(
      field(record, "feedback", path),
      `${path}.feedback`,
      decodeDisclosedFeedback,
    ),
    nextIssued: decodeNullable(
      field(record, "nextIssued", path),
      `${path}.nextIssued`,
      decodeNextIssuedAttempt,
    ),
  } satisfies SubmissionReceipt;
  return decoded;
}

export function decodeNextIssuedAttempt(value: unknown, path = "response"): NextIssuedAttempt {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "id",
    "run",
    "questionVersion",
    "seed",
    "deadline",
    "assignmentPosition",
    "renderedQuestionSha256",
  ]);
  return {
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    run: decodeIdentifier(field(record, "run", path), `${path}.run`),
    questionVersion: decodeIdentifier(
      field(record, "questionVersion", path),
      `${path}.questionVersion`,
    ),
    seed: decodeNonnegativeInteger(field(record, "seed", path), `${path}.seed`),
    deadline: decodeNullable(
      field(record, "deadline", path),
      `${path}.deadline`,
      decodeFiniteNumber,
    ),
    assignmentPosition: decodeNonnegativeInteger(
      field(record, "assignmentPosition", path),
      `${path}.assignmentPosition`,
    ),
    renderedQuestionSha256: decodeSha256(
      field(record, "renderedQuestionSha256", path),
      `${path}.renderedQuestionSha256`,
    ),
  } satisfies NextIssuedAttempt;
}

export function decodePrefetchedNextQuestion(
  value: unknown,
  path = "response",
): PrefetchedNextQuestion {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "predecessor",
    "run",
    "assignmentPosition",
    "questionVersion",
    "seed",
    "renderedQuestionSha256",
    "envelope",
  ]);
  const decoded = {
    predecessor: decodeIdentifier(field(record, "predecessor", path), `${path}.predecessor`),
    run: decodeIdentifier(field(record, "run", path), `${path}.run`),
    assignmentPosition: decodeNonnegativeInteger(
      field(record, "assignmentPosition", path),
      `${path}.assignmentPosition`,
    ),
    questionVersion: decodeIdentifier(
      field(record, "questionVersion", path),
      `${path}.questionVersion`,
    ),
    seed: decodeNonnegativeInteger(field(record, "seed", path), `${path}.seed`),
    renderedQuestionSha256: decodeSha256(
      field(record, "renderedQuestionSha256", path),
      `${path}.renderedQuestionSha256`,
    ),
    envelope: decodeQuestionEnvelope(field(record, "envelope", path), `${path}.envelope`),
  } satisfies PrefetchedNextQuestion;
  if (
    decoded.envelope.version !== decoded.questionVersion ||
    decoded.envelope.seed !== decoded.seed
  ) {
    throw new DecodeError(path, "a prefetch envelope bound to its descriptor");
  }
  return decoded;
}

function decodeCursorPage<T>(value: unknown, path: string, decodeItem: Decoder<T>): CursorPage<T> {
  const record = decodeRecord(value, path);
  const decoded = {
    items: decodeArray(field(record, "items", path), `${path}.items`, decodeItem),
    nextCursor: decodeNullable(
      field(record, "nextCursor", path),
      `${path}.nextCursor`,
      decodeNonemptyString,
    ),
  } satisfies CursorPage<T>;
  return decoded;
}

function decodeStrictCursorPage<T>(
  value: unknown,
  path: string,
  decodeItem: Decoder<T>,
): CursorPage<T> {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["items", "nextCursor"]);
  const decoded = {
    items: decodeArray(field(record, "items", path), `${path}.items`, decodeItem),
    nextCursor: decodeNullable(
      field(record, "nextCursor", path),
      `${path}.nextCursor`,
      decodeNonemptyString,
    ),
  } satisfies CursorPage<T>;
  return decoded;
}

export function decodeCatalogPage(
  value: unknown,
  path = "response",
): CursorPage<CatalogProblemSummary> {
  return decodeCursorPage(value, path, decodeCatalogProblemSummary);
}

export function decodeTaxonomyPage(value: unknown, path = "response"): CursorPage<TaxonomyTerm> {
  return decodeCursorPage(value, path, decodeTaxonomyTerm);
}

export function decodeCoursePage(value: unknown, path = "response"): CursorPage<CourseSummary> {
  return decodeCursorPage(value, path, decodeCourseSummary);
}

export function decodeAssignmentPage(
  value: unknown,
  path = "response",
): CursorPage<AssignmentSummary> {
  return decodeCursorPage(value, path, decodeAssignmentSummary);
}

export function decodeRunPage(value: unknown, path = "response"): CursorPage<AssignmentRun> {
  return decodeCursorPage(value, path, decodeAssignmentRun);
}

export function decodeAttemptPage(value: unknown, path = "response"): CursorPage<QuestionAttempt> {
  return decodeCursorPage(value, path, decodeQuestionAttempt);
}

export function decodeGradebookPage(
  value: unknown,
  path = "response",
): CursorPage<GradebookSummaryRow> {
  return decodeStrictCursorPage(value, path, decodeGradebookSummaryRow);
}

function decodeResponseFormatViolation(value: unknown, path: string): ResponseFormatViolation {
  const record = decodeRecord(value, path);
  const violation = kind(record, path);
  switch (violation) {
    case "responseKindMismatch":
    case "numericNotFinite":
    case "orderingItemsMismatch":
    case "missingUploadReference":
      return { kind: violation };
    case "selectionCount": {
      const decoded = {
        kind: violation,
        expected: decodeSelectionCardinality(field(record, "expected", path), `${path}.expected`),
        actual: decodeNonnegativeInteger(field(record, "actual", path), `${path}.actual`),
      } satisfies ResponseFormatViolation;
      return decoded;
    }
    case "duplicateChoice":
    case "unknownChoice": {
      const decoded = {
        kind: violation,
        choice: decodeNonemptyString(field(record, "choice", path), `${path}.choice`),
      } satisfies ResponseFormatViolation;
      return decoded;
    }
    case "textTooLong": {
      const decoded = {
        kind: violation,
        maxLength: decodeNonnegativeInteger(field(record, "maxLength", path), `${path}.maxLength`),
        actualLength: decodeNonnegativeInteger(
          field(record, "actualLength", path),
          `${path}.actualLength`,
        ),
      } satisfies ResponseFormatViolation;
      return decoded;
    }
    default:
      throw new DecodeError(`${path}.kind`, "a known response-format violation");
  }
}

export function decodeResponseFormatReport(
  value: unknown,
  path = "response",
): ResponseFormatReport {
  const record = decodeRecord(value, path);
  const decoded = {
    violations: decodeArray(
      field(record, "violations", path),
      `${path}.violations`,
      decodeResponseFormatViolation,
    ),
  } satisfies ResponseFormatReport;
  return decoded;
}

export function decodeTimerVerdict(value: unknown, path = "response"): TimerVerdict {
  return decodeStringEnum(value, path, [
    "untimed",
    "open",
    "gracePeriod",
    "submittedOnTime",
    "submittedWithinGrace",
    "timedOut",
  ]);
}

function decodeCapabilityViolation(value: unknown, path: string): CapabilityViolation {
  const record = decodeRecord(value, path);
  const decoded = {
    question: decodeIdentifier(field(record, "question", path), `${path}.question`),
    capability: decodeCapability(field(record, "capability", path), `${path}.capability`),
  } satisfies CapabilityViolation;
  return decoded;
}

export function decodeCapabilityViolations(
  value: unknown,
  path = "response",
): ReadonlyArray<CapabilityViolation> {
  return decodeArray(value, path, decodeCapabilityViolation);
}
