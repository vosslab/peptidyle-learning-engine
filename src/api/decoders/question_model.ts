// Authoring and publication decoders.

import type { QuestionAttemptLimit } from "../../../generated/api/QuestionAttemptLimit";
import type { ContentBlock } from "../../../generated/api/ContentBlock";
import type { DraftQuestionSource } from "../../../generated/api/DraftQuestionSource";
import type { GradingDefinition } from "../../../generated/api/GradingDefinition";
import type { QuestionGeneratorParameter } from "../../../generated/api/QuestionGeneratorParameter";
import type { QuestionSource } from "../../../generated/api/QuestionSource";
import type { QuestionVariationDefinition } from "../../../generated/api/QuestionVariationDefinition";
import type { QuestionResponseFormat } from "../../../generated/api/QuestionResponseFormat";
import type { QuestionFormat } from "../../../generated/api/QuestionFormat";
import type { QuestionAttemptTimeLimit } from "../../../generated/api/QuestionAttemptTimeLimit";
import type { WorkspaceDraftSummary } from "../../../generated/api/WorkspaceDraftSummary";
import type { WorkspaceRouteReference } from "../../navigation/public_route";
import { parseWorkspaceReference } from "../../navigation/public_route";

function decodeAuthoringWorkspaceReference(value: unknown, path: string): WorkspaceRouteReference {
  if (typeof value !== "string") throw new DecodeError(path, "a W- reference");
  const reference = parseWorkspaceReference(value);
  if (reference === null) throw new DecodeError(path, "a W- reference");
  return reference;
}
import type {
  PublicationDiff,
  PublicationReadinessFailure,
  PublicationSemanticProjection,
  PublicationValidationReport,
  PublicationViolation,
} from "../contracts";
import {
  DecodeError,
  decodeArray,
  decodeDictionary,
  decodeFiniteNumber,
  decodeNonemptyString,
  decodeNonnegativeInteger,
  decodeNullable,
  decodePositiveInteger,
  decodeRecord,
  decodeSafeInteger,
  decodeString,
  decodeStringEnum,
} from "../decoder";
import {
  MAX_CURSOR_PAGE_ITEMS,
  MAX_PUBLICATION_SEMANTIC_ENTRIES,
  QUESTION_BACKENDS,
  decodeCapability,
  decodeBoundedArray,
  decodeCursor,
  decodeEnvelopeTitle,
  decodeIdentifier,
  decodeLicense,
  decodeTaxonomyTerm,
  field,
  kind,
  requireOnlyFields,
} from "./shared";
import {
  decodeContentBlock,
  decodeQuestionResponseFormat,
  decodeQuestionType,
  questionResponseFormatSupportsType,
  decodeResponseSelectionRule,
} from "./question_response_format";

export {
  decodeContentBlock,
  decodeQuestionResponseFormat,
  decodeQuestionType,
  decodeResponseSelectionRule,
  questionResponseFormatSupportsType,
};

const QUESTION_FORMATS = [
  "pleFlatQuestionV2",
  "nativeAlgorithmic",
  "webworkPg",
  "qti",
  "h5p",
  "imathas",
] as const satisfies ReadonlyArray<QuestionFormat>;

/** Strict decoder for the authored or imported Question representation. */
export function decodeQuestionFormat(value: unknown, path: string): QuestionFormat {
  return decodeStringEnum(value, path, QUESTION_FORMATS);
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
  response: QuestionResponseFormat;
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
    response: decodeQuestionResponseFormat(
      field(record, "response", path),
      `${path}.response`,
      true,
    ),
  };
}

export function decodeQuestionSource(value: unknown, path: string): QuestionSource {
  const record = decodeRecord(value, path);
  const backend = decodeString(field(record, "backend", path), `${path}.backend`);
  switch (backend) {
    case "native": {
      requireOnlyFields(record, path, ["backend"]);
      const decoded = { backend } satisfies QuestionSource;
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

export function decodeDraftQuestionSource(value: unknown, path: string): DraftQuestionSource {
  const record = decodeRecord(value, path);
  const backend = decodeString(field(record, "backend", path), `${path}.backend`);
  switch (backend) {
    case "native":
      requireOnlyFields(record, path, ["backend"]);
      return { backend } satisfies DraftQuestionSource;
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

export function decodeGeneratorReference(
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

export function decodeQuestionGeneratorParameter(
  value: unknown,
  path: string,
  strict = false,
): QuestionGeneratorParameter {
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
      } satisfies QuestionGeneratorParameter;
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
      } satisfies QuestionGeneratorParameter;
      return decoded;
    }
    case "choice": {
      if (strict) {
        requireOnlyFields(record, path, ["kind", "options"]);
      }
      const decoded = {
        kind: parameter,
        options: decodeArray(field(record, "options", path), `${path}.options`, decodeString),
      } satisfies QuestionGeneratorParameter;
      return decoded;
    }
    case "fixed": {
      if (strict) {
        requireOnlyFields(record, path, ["kind", "value"]);
      }
      const decoded = {
        kind: parameter,
        value: decodeString(field(record, "value", path), `${path}.value`),
      } satisfies QuestionGeneratorParameter;
      return decoded;
    }
    default:
      throw new DecodeError(`${path}.kind`, "a known parameter specification");
  }
}

export function decodeQuestionVariationDefinition(
  value: unknown,
  path: string,
  strict = false,
): QuestionVariationDefinition {
  const record = decodeRecord(value, path);
  const variationDefinition = kind(record, path);
  switch (variationDefinition) {
    case "static":
      if (strict) {
        requireOnlyFields(record, path, ["kind"]);
      }
      return { kind: variationDefinition };
    case "seeded": {
      if (strict) {
        requireOnlyFields(record, path, ["kind", "generator", "parameters"]);
      }
      const decoded = {
        kind: variationDefinition,
        generator: decodeGeneratorReference(
          field(record, "generator", path),
          `${path}.generator`,
          strict,
        ),
        parameters: decodeDictionary(
          field(record, "parameters", path),
          `${path}.parameters`,
          (parameter, parameterPath) =>
            decodeQuestionGeneratorParameter(parameter, parameterPath, strict),
        ),
      } satisfies QuestionVariationDefinition;
      return decoded;
    }
    default:
      throw new DecodeError(`${path}.kind`, "a known Question Variation Definition");
  }
}

export function decodeQuestionAttemptLimit(
  value: unknown,
  path: string,
  strict = false,
): QuestionAttemptLimit {
  const record = decodeRecord(value, path);
  if (strict) {
    requireOnlyFields(record, path, ["maxAttempts"]);
  }
  const decoded = {
    maxAttempts: decodeNullable(
      field(record, "maxAttempts", path),
      `${path}.maxAttempts`,
      decodePositiveInteger,
    ),
  } satisfies QuestionAttemptLimit;
  return decoded;
}

export function decodeQuestionAttemptTimeLimit(
  value: unknown,
  path: string,
  strict = false,
): QuestionAttemptTimeLimit {
  const record = decodeRecord(value, path);
  const timing = kind(record, path);
  switch (timing) {
    case "unlimited":
      if (strict) {
        requireOnlyFields(record, path, ["kind"]);
      }
      return { kind: timing };
    case "limited": {
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
      } satisfies QuestionAttemptTimeLimit;
      return decoded;
    }
    default:
      throw new DecodeError(`${path}.kind`, "a known timing policy");
  }
}

export function decodeGradingDefinition(
  value: unknown,
  path: string,
  strict = false,
): GradingDefinition {
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

/** Strict compact projection for an instructor-owned, unversioned workspace draft. */
export function decodeWorkspaceDraftSummary(
  value: unknown,
  path = "response",
): WorkspaceDraftSummary {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["workspace", "reference", "title", "sourceBackend"]);
  return {
    workspace: decodeIdentifier(field(record, "workspace", path), `${path}.workspace`),
    reference: decodeAuthoringWorkspaceReference(
      field(record, "reference", path),
      `${path}.reference`,
    ),
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
  const items = decodeBoundedArray(
    field(record, "items", path),
    `${path}.items`,
    MAX_CURSOR_PAGE_ITEMS,
    decodeWorkspaceDraftSummary,
  );
  const nextCursor = decodeNullable(
    field(record, "nextCursor", path),
    `${path}.nextCursor`,
    decodeCursor,
  );
  return { items, nextCursor };
}

const PUBLICATION_FIELDS = [
  "sourceBackend",
  "title",
  "prompt",
  "response",
  "questionAttemptLimit",
  "questionAttemptTimeLimit",
  "questionVariationDefinition",
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
    "questionAttemptLimit",
    "questionAttemptTimeLimit",
    "questionVariationDefinition",
    "metadata",
  ]);
  const prompt = decodeRecord(field(record, "prompt", path), `${path}.prompt`);
  requireOnlyFields(prompt, `${path}.prompt`, ["blocks"]);
  const response = decodeRecord(field(record, "response", path), `${path}.response`);
  requireOnlyFields(response, `${path}.response`, ["kind", "optionCount"]);
  const responseKind = decodeStringEnum(
    field(response, "kind", `${path}.response`),
    `${path}.response.kind`,
    [
      "numeric",
      "multipleChoice",
      "shortText",
      "multiBlank",
      "matching",
      "ordering",
      "hotspot",
      "externalTool",
    ],
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
  const questionVariationDefinition = decodeRecord(
    field(record, "questionVariationDefinition", path),
    `${path}.questionVariationDefinition`,
  );
  requireOnlyFields(questionVariationDefinition, `${path}.questionVariationDefinition`, ["kind"]);
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
    questionAttemptLimit: decodeQuestionAttemptLimit(
      field(record, "questionAttemptLimit", path),
      `${path}.questionAttemptLimit`,
      true,
    ),
    questionAttemptTimeLimit: decodeQuestionAttemptTimeLimit(
      field(record, "questionAttemptTimeLimit", path),
      `${path}.questionAttemptTimeLimit`,
      true,
    ),
    questionVariationDefinition: {
      kind: decodeStringEnum(
        field(questionVariationDefinition, "kind", `${path}.questionVariationDefinition`),
        `${path}.questionVariationDefinition.kind`,
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
  requireOnlyFields(record, path, ["draftRevision", "baseline", "current", "changed"]);
  const draftRevision = decodePositiveInteger(
    field(record, "draftRevision", path),
    `${path}.draftRevision`,
  );
  const baseline = decodeStringEnum(field(record, "baseline", path), `${path}.baseline`, [
    "newQuestion",
  ] as const);
  const current = decodePublicationSemanticProjection(
    field(record, "current", path),
    `${path}.current`,
  );
  const changed = decodeBoundedArray(
    field(record, "changed", path),
    `${path}.changed`,
    PUBLICATION_FIELDS.length,
    (entry, entryPath) => decodeStringEnum(entry, entryPath, PUBLICATION_FIELDS),
  );
  if (
    new Set(changed).size !== changed.length ||
    (baseline === "newQuestion" && changed.length !== 0)
  ) {
    throw new DecodeError(`${path}.changed`, "unique baseline-consistent semantic fields");
  }
  return {
    draftRevision,
    revision: `"${draftRevision}"`,
    baseline,
    current,
    changed,
  };
}
