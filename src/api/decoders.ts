// decoders.ts - exhaustive runtime decoders for browser-visible API DTOs.

import type { AssetRef } from "../../generated/api/AssetRef";
import type { AssignmentEnrollment } from "../../generated/api/AssignmentEnrollment";
import type { AssignmentRun } from "../../generated/api/AssignmentRun";
import type { AssignmentSummary } from "../../generated/api/AssignmentSummary";
import type { AttemptPolicy } from "../../generated/api/AttemptPolicy";
import type { AttemptProvenance } from "../../generated/api/AttemptProvenance";
import type { AttemptResult } from "../../generated/api/AttemptResult";
import type { AttemptTimerRecord } from "../../generated/api/AttemptTimerRecord";
import type { BackendCapabilities } from "../../generated/api/BackendCapabilities";
import type { Capability } from "../../generated/api/Capability";
import type { CatalogLifecycle } from "../../generated/api/CatalogLifecycle";
import type { CatalogProblemSummary } from "../../generated/api/CatalogProblemSummary";
import type { ChoiceOption } from "../../generated/api/ChoiceOption";
import type { CompletionRequirement } from "../../generated/api/CompletionRequirement";
import type { ContentBlock } from "../../generated/api/ContentBlock";
import type { ContinuedPractice } from "../../generated/api/ContinuedPractice";
import type { CourseSummary } from "../../generated/api/CourseSummary";
import type { GradingDefinition } from "../../generated/api/GradingDefinition";
import type { License } from "../../generated/api/License";
import type { NumericTolerance } from "../../generated/api/NumericTolerance";
import type { ParameterSpec } from "../../generated/api/ParameterSpec";
import type { ProblemVersionRef } from "../../generated/api/ProblemVersionRef";
import type { QuestionAttempt } from "../../generated/api/QuestionAttempt";
import type { QuestionDefinition } from "../../generated/api/QuestionDefinition";
import type { QuestionMetadata } from "../../generated/api/QuestionMetadata";
import type { QuestionSource } from "../../generated/api/QuestionSource";
import type { RandomizationDefinition } from "../../generated/api/RandomizationDefinition";
import type { ResponseDefinition } from "../../generated/api/ResponseDefinition";
import type { RunPolicies } from "../../generated/api/RunPolicies";
import type { SelectionCardinality } from "../../generated/api/SelectionCardinality";
import type { SourceArtifact } from "../../generated/api/SourceArtifact";
import type { StudentAssignmentSummary } from "../../generated/api/StudentAssignmentSummary";
import type { StudentResponse } from "../../generated/api/StudentResponse";
import type { TaxonomyTerm } from "../../generated/api/TaxonomyTerm";
import type { TimingPolicy } from "../../generated/api/TimingPolicy";
import type {
  CapabilityViolation,
  ResponseFormatReport,
  ResponseFormatViolation,
  TimerVerdict,
} from "../wasm/index";
import type { AuthSession, CursorPage, EnrollmentView, SubmissionReceipt } from "./contracts";
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

function field(record: Record<string, unknown>, key: string, path: string): unknown {
  return decodeField(record, key, path);
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

function decodeProblemVersionRef(value: unknown, path: string): ProblemVersionRef {
  const record = decodeRecord(value, path);
  const decoded = {
    problem: decodeIdentifier(field(record, "problem", path), `${path}.problem`),
    version: decodeIdentifier(field(record, "version", path), `${path}.version`),
  } satisfies ProblemVersionRef;
  return decoded;
}

function decodeTaxonomyTerm(value: unknown, path: string): TaxonomyTerm {
  const record = decodeRecord(value, path);
  const decoded = {
    scheme: decodeNonemptyString(field(record, "scheme", path), `${path}.scheme`),
    code: decodeNonemptyString(field(record, "code", path), `${path}.code`),
    label: decodeNonemptyString(field(record, "label", path), `${path}.label`),
  } satisfies TaxonomyTerm;
  return decoded;
}

function decodeLicense(value: unknown, path: string): License {
  const record = decodeRecord(value, path);
  const tag = kind(record, path);
  switch (tag) {
    case "allRightsReserved":
    case "ccBy":
    case "ccBySa":
    case "ccByNc":
    case "cc0":
      return { kind: tag };
    case "other": {
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

function decodeQuestionMetadata(value: unknown, path: string): QuestionMetadata {
  const record = decodeRecord(value, path);
  const decoded = {
    title: decodeNonemptyString(field(record, "title", path), `${path}.title`),
    tags: decodeArray(field(record, "tags", path), `${path}.tags`, decodeString),
    taxonomy: decodeArray(field(record, "taxonomy", path), `${path}.taxonomy`, decodeTaxonomyTerm),
    license: decodeLicense(field(record, "license", path), `${path}.license`),
    language: decodeNonemptyString(field(record, "language", path), `${path}.language`),
  } satisfies QuestionMetadata;
  return decoded;
}

function decodeCatalogLifecycle(value: unknown, path: string): CatalogLifecycle {
  const record = decodeRecord(value, path);
  const lifecycle = state(record, path);
  switch (lifecycle) {
    case "published":
      return { state: lifecycle };
    case "deprecated":
    case "archived": {
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
): CatalogProblemSummary {
  const record = decodeRecord(value, path);
  const decoded = {
    problem: decodeIdentifier(field(record, "problem", path), `${path}.problem`),
    version: decodeIdentifier(field(record, "version", path), `${path}.version`),
    backend: decodeStringEnum(field(record, "backend", path), `${path}.backend`, [
      "native",
      "webwork",
      "qti",
      "h5p",
    ]),
    capabilities: decodeBackendCapabilities(
      field(record, "capabilities", path),
      `${path}.capabilities`,
    ),
    metadata: decodeQuestionMetadata(field(record, "metadata", path), `${path}.metadata`),
    scope: decodeStringEnum(field(record, "scope", path), `${path}.scope`, [
      "institution",
      "public",
    ]),
    lifecycle: decodeCatalogLifecycle(field(record, "lifecycle", path), `${path}.lifecycle`),
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

function decodeCompletionRequirement(value: unknown, path: string): CompletionRequirement {
  const record = decodeRecord(value, path);
  const requirement = kind(record, path);
  switch (requirement) {
    case "answerAll":
    case "allCorrect":
      return { kind: requirement };
    case "scoreAtLeast": {
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

function decodeContinuedPractice(value: unknown, path: string): ContinuedPractice {
  const record = decodeRecord(value, path);
  const practice = kind(record, path);
  switch (practice) {
    case "unlimited":
    case "closed":
      return { kind: practice };
    case "capped": {
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

function decodeRunPolicies(value: unknown, path: string): RunPolicies {
  const record = decodeRecord(value, path);
  const decoded = {
    completion: decodeCompletionRequirement(
      field(record, "completion", path),
      `${path}.completion`,
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

export function decodeAssignmentSummary(value: unknown, path = "response"): AssignmentSummary {
  const record = decodeRecord(value, path);
  const decoded = {
    id: decodeIdentifier(field(record, "id", path), `${path}.id`),
    tenant: decodeIdentifier(field(record, "tenant", path), `${path}.tenant`),
    courseId: decodeIdentifier(field(record, "courseId", path), `${path}.courseId`),
    title: decodeNonemptyString(field(record, "title", path), `${path}.title`),
    problems: decodeArray(
      field(record, "problems", path),
      `${path}.problems`,
      decodeProblemVersionRef,
    ),
    policies: decodeRunPolicies(field(record, "policies", path), `${path}.policies`),
  } satisfies AssignmentSummary;
  return decoded;
}

function decodeAssetRef(value: unknown, path: string): AssetRef {
  const record = decodeRecord(value, path);
  const decoded = {
    asset: decodeIdentifier(field(record, "asset", path), `${path}.asset`),
    checksum: decodeSha256(field(record, "checksum", path), `${path}.checksum`),
  } satisfies AssetRef;
  return decoded;
}

function decodeContentBlock(value: unknown, path: string): ContentBlock {
  const record = decodeRecord(value, path);
  const block = kind(record, path);
  switch (block) {
    case "text": {
      const decoded = {
        kind: block,
        markdown: decodeString(field(record, "markdown", path), `${path}.markdown`),
      } satisfies ContentBlock;
      return decoded;
    }
    case "math": {
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
      const decoded = {
        kind: block,
        asset: decodeAssetRef(field(record, "asset", path), `${path}.asset`),
        description: decodeNonemptyString(
          field(record, "description", path),
          `${path}.description`,
        ),
      } satisfies ContentBlock;
      return decoded;
    }
    case "code": {
      const decoded = {
        kind: block,
        language: decodeNonemptyString(field(record, "language", path), `${path}.language`),
        source: decodeString(field(record, "source", path), `${path}.source`),
      } satisfies ContentBlock;
      return decoded;
    }
    case "table": {
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

function decodeChoiceOption(value: unknown, path: string): ChoiceOption {
  const record = decodeRecord(value, path);
  const decoded = {
    id: decodeNonemptyString(field(record, "id", path), `${path}.id`),
    body: decodeArray(field(record, "body", path), `${path}.body`, decodeContentBlock),
  } satisfies ChoiceOption;
  return decoded;
}

function decodeNumericTolerance(value: unknown, path: string): NumericTolerance {
  const record = decodeRecord(value, path);
  const tolerance = kind(record, path);
  switch (tolerance) {
    case "exact":
      return { kind: tolerance };
    case "absolute": {
      const decoded = {
        kind: tolerance,
        epsilon: decodeFiniteNumber(field(record, "epsilon", path), `${path}.epsilon`),
      } satisfies NumericTolerance;
      return decoded;
    }
    case "relative": {
      const decoded = {
        kind: tolerance,
        fraction: decodeFiniteNumber(field(record, "fraction", path), `${path}.fraction`),
      } satisfies NumericTolerance;
      return decoded;
    }
    case "significantFigures": {
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

function decodeSelectionCardinality(value: unknown, path: string): SelectionCardinality {
  const record = decodeRecord(value, path);
  const selection = kind(record, path);
  switch (selection) {
    case "exactlyOne":
    case "anyNumber":
    case "atLeastOne":
      return { kind: selection };
    case "exactly": {
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

function decodeResponseDefinition(value: unknown, path: string): ResponseDefinition {
  const record = decodeRecord(value, path);
  const response = kind(record, path);
  switch (response) {
    case "numeric": {
      const decoded = {
        kind: response,
        tolerance: decodeNumericTolerance(field(record, "tolerance", path), `${path}.tolerance`),
        unit: decodeNullable(field(record, "unit", path), `${path}.unit`, decodeString),
      } satisfies ResponseDefinition;
      return decoded;
    }
    case "multipleChoice": {
      const decoded = {
        kind: response,
        choices: decodeArray(field(record, "choices", path), `${path}.choices`, decodeChoiceOption),
        selection: decodeSelectionCardinality(
          field(record, "selection", path),
          `${path}.selection`,
        ),
      } satisfies ResponseDefinition;
      return decoded;
    }
    case "shortText": {
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
      const decoded = {
        kind: response,
        items: decodeArray(field(record, "items", path), `${path}.items`, decodeChoiceOption),
      } satisfies ResponseDefinition;
      return decoded;
    }
    case "fileUpload": {
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
    default:
      throw new DecodeError(`${path}.kind`, "a known response definition");
  }
}

function decodeQuestionSource(value: unknown, path: string): QuestionSource {
  const record = decodeRecord(value, path);
  const backend = decodeString(field(record, "backend", path), `${path}.backend`);
  switch (backend) {
    case "native": {
      const decoded = {
        backend,
        family: decodeNonemptyString(field(record, "family", path), `${path}.family`),
      } satisfies QuestionSource;
      return decoded;
    }
    case "webwork": {
      const decoded = {
        backend,
        pgPath: decodeNonemptyString(field(record, "pgPath", path), `${path}.pgPath`),
      } satisfies QuestionSource;
      return decoded;
    }
    case "qti": {
      const decoded = {
        backend,
        itemId: decodeNonemptyString(field(record, "itemId", path), `${path}.itemId`),
        packageAsset: decodeIdentifier(field(record, "packageAsset", path), `${path}.packageAsset`),
      } satisfies QuestionSource;
      return decoded;
    }
    case "h5p": {
      const decoded = {
        backend,
        contentType: decodeNonemptyString(
          field(record, "contentType", path),
          `${path}.contentType`,
        ),
      } satisfies QuestionSource;
      return decoded;
    }
    default:
      throw new DecodeError(`${path}.backend`, "a known question backend");
  }
}

function decodeGeneratorReference(value: unknown, path: string): { id: string; version: string } {
  const record = decodeRecord(value, path);
  return {
    id: decodeNonemptyString(field(record, "id", path), `${path}.id`),
    version: decodeNonemptyString(field(record, "version", path), `${path}.version`),
  };
}

function decodeParameterSpec(value: unknown, path: string): ParameterSpec {
  const record = decodeRecord(value, path);
  const parameter = kind(record, path);
  switch (parameter) {
    case "integerRange": {
      const decoded = {
        kind: parameter,
        low: decodeSafeInteger(field(record, "low", path), `${path}.low`),
        high: decodeSafeInteger(field(record, "high", path), `${path}.high`),
      } satisfies ParameterSpec;
      return decoded;
    }
    case "decimalRange": {
      const decoded = {
        kind: parameter,
        low: decodeFiniteNumber(field(record, "low", path), `${path}.low`),
        high: decodeFiniteNumber(field(record, "high", path), `${path}.high`),
        decimals: decodeNonnegativeInteger(field(record, "decimals", path), `${path}.decimals`),
      } satisfies ParameterSpec;
      return decoded;
    }
    case "choice": {
      const decoded = {
        kind: parameter,
        options: decodeArray(field(record, "options", path), `${path}.options`, decodeString),
      } satisfies ParameterSpec;
      return decoded;
    }
    case "fixed": {
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

function decodeRandomization(value: unknown, path: string): RandomizationDefinition {
  const record = decodeRecord(value, path);
  const randomization = kind(record, path);
  switch (randomization) {
    case "static":
      return { kind: randomization };
    case "seeded": {
      const decoded = {
        kind: randomization,
        generator: decodeGeneratorReference(field(record, "generator", path), `${path}.generator`),
        parameters: decodeDictionary(
          field(record, "parameters", path),
          `${path}.parameters`,
          decodeParameterSpec,
        ),
      } satisfies RandomizationDefinition;
      return decoded;
    }
    default:
      throw new DecodeError(`${path}.kind`, "a known randomization definition");
  }
}

function decodeAttemptPolicy(value: unknown, path: string): AttemptPolicy {
  const record = decodeRecord(value, path);
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

function decodeTimingPolicy(value: unknown, path: string): TimingPolicy {
  const record = decodeRecord(value, path);
  const timing = kind(record, path);
  switch (timing) {
    case "untimed":
      return { kind: timing };
    case "perQuestion":
    case "perAttempt": {
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

function decodeGradingDefinition(value: unknown, path: string): GradingDefinition {
  const record = decodeRecord(value, path);
  const mode = decodeString(field(record, "mode", path), `${path}.mode`);
  switch (mode) {
    case "allOrNothing":
    case "partialCredit": {
      const decoded = {
        mode,
        points: decodeFiniteNumber(field(record, "points", path), `${path}.points`),
      } satisfies GradingDefinition;
      return decoded;
    }
    case "ungraded":
      return { mode };
    default:
      throw new DecodeError(`${path}.mode`, "a known grading definition");
  }
}

export function decodeQuestionDefinition(value: unknown, path = "response"): QuestionDefinition {
  const record = decodeRecord(value, path);
  const decoded = {
    version: decodeIdentifier(field(record, "version", path), `${path}.version`),
    problem: decodeNullable(field(record, "problem", path), `${path}.problem`, decodeIdentifier),
    workspace: decodeIdentifier(field(record, "workspace", path), `${path}.workspace`),
    source: decodeQuestionSource(field(record, "source", path), `${path}.source`),
    prompt: decodeArray(field(record, "prompt", path), `${path}.prompt`, decodeContentBlock),
    response: decodeResponseDefinition(field(record, "response", path), `${path}.response`),
    attemptPolicy: decodeAttemptPolicy(
      field(record, "attemptPolicy", path),
      `${path}.attemptPolicy`,
    ),
    timingPolicy: decodeTimingPolicy(field(record, "timingPolicy", path), `${path}.timingPolicy`),
    randomization: decodeRandomization(
      field(record, "randomization", path),
      `${path}.randomization`,
    ),
    grading: decodeGradingDefinition(field(record, "grading", path), `${path}.grading`),
    metadata: decodeQuestionMetadata(field(record, "metadata", path), `${path}.metadata`),
  } satisfies QuestionDefinition;
  return decoded;
}

function decodeStudentResponse(value: unknown, path: string): StudentResponse {
  const record = decodeRecord(value, path);
  const response = kind(record, path);
  switch (response) {
    case "numeric": {
      const decoded = {
        kind: response,
        value: decodeFiniteNumber(field(record, "value", path), `${path}.value`),
      } satisfies StudentResponse;
      return decoded;
    }
    case "multipleChoice": {
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
      const decoded = {
        kind: response,
        text: decodeString(field(record, "text", path), `${path}.text`),
      } satisfies StudentResponse;
      return decoded;
    }
    case "ordering": {
      const decoded = {
        kind: response,
        order: decodeArray(field(record, "order", path), `${path}.order`, decodeNonemptyString),
      } satisfies StudentResponse;
      return decoded;
    }
    case "fileUpload": {
      const decoded = {
        kind: response,
        objectKey: decodeNonemptyString(field(record, "objectKey", path), `${path}.objectKey`),
      } satisfies StudentResponse;
      return decoded;
    }
    default:
      throw new DecodeError(`${path}.kind`, "a known student-response kind");
  }
}

function decodeAttemptResult(value: unknown, path: string): AttemptResult {
  const record = decodeRecord(value, path);
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

function decodeAttemptTimer(value: unknown, path: string): AttemptTimerRecord {
  const record = decodeRecord(value, path);
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
  return decodeGeneratorReference(value, path);
}

function decodeSourceArtifact(value: unknown, path: string): SourceArtifact {
  const record = decodeRecord(value, path);
  const decoded = {
    object: decodeIdentifier(field(record, "object", path), `${path}.object`),
    sha256: decodeSha256(field(record, "sha256", path), `${path}.sha256`),
  } satisfies SourceArtifact;
  return decoded;
}

function decodeAttemptProvenance(value: unknown, path: string): AttemptProvenance {
  const record = decodeRecord(value, path);
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
      decodeGeneratorReference,
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
  const decoded = {
    accepted: decodeTrue(field(record, "accepted", path), `${path}.accepted`),
    attempt: decodeQuestionAttempt(field(record, "attempt", path), `${path}.attempt`),
  } satisfies SubmissionReceipt;
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
