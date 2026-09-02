// Authoring and publication decoders.

import type { QuestionAttemptLimit } from "../../../generated/api/QuestionAttemptLimit";
import type { QuestionContentBlock } from "../../../generated/api/QuestionContentBlock";
import type { DraftQuestionBackendLocator } from "../../../generated/api/DraftQuestionBackendLocator";
import type { QuestionGradingRule } from "../../../generated/api/QuestionGradingRule";
import type { QuestionGeneratorParameter } from "../../../generated/api/QuestionGeneratorParameter";
import type { QuestionBackendLocator } from "../../../generated/api/QuestionBackendLocator";
import type { QuestionVariationRule } from "../../../generated/api/QuestionVariationRule";
import type { QuestionResponseFormat } from "../../../generated/api/QuestionResponseFormat";
import type { QuestionFormat } from "../../../generated/api/QuestionFormat";
import type { QuestionAttemptTimeLimit } from "../../../generated/api/QuestionAttemptTimeLimit";
import type { DraftQuestionSummary } from "../../../generated/api/DraftQuestionSummary";
import type {
  AuthoringWorkspaceRouteReference,
  DraftQuestionRouteReference,
} from "../../navigation/public_route";
import {
  parseAuthoringWorkspaceReference,
  parseDraftQuestionReference,
} from "../../navigation/public_route";

function decodeDraftQuestionReference(value: unknown, path: string): DraftQuestionRouteReference {
  if (typeof value !== "string") throw new DecodeError(path, "a D- reference");
  const reference = parseDraftQuestionReference(value);
  if (reference === null) throw new DecodeError(path, "a D- reference");
  return reference;
}

function decodeAuthoringWorkspaceReference(
  value: unknown,
  path: string,
): AuthoringWorkspaceRouteReference {
  if (typeof value !== "string") throw new DecodeError(path, "a W- reference");
  const reference = parseAuthoringWorkspaceReference(value);
  if (reference === null) throw new DecodeError(path, "a W- reference");
  return reference;
}
import type {
  QuestionPublicationReview,
  QuestionPublicationValidationUnavailable,
  QuestionPublicationReviewSummary,
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
  decodeQuestionLicense,
  decodeQuestionCitation,
  decodeQuestionClassification,
  field,
  kind,
  requireOnlyFields,
} from "./shared";
import {
  decodeQuestionContentBlock,
  decodeQuestionResponseFormat,
  decodeQuestionType,
  questionResponseFormatSupportsType,
  decodeResponseSelectionRule,
} from "./question_response_format";

export {
  decodeQuestionContentBlock,
  decodeQuestionResponseFormat,
  decodeQuestionType,
  decodeResponseSelectionRule,
  questionResponseFormatSupportsType,
};

const QUESTION_FORMATS = [
  "pleQuestionJson",
  "pleAlgorithmic",
  "webworkPg",
  "qti",
  "h5p",
  "imathas",
] as const satisfies ReadonlyArray<QuestionFormat>;

const IMATHAS_IDENTIFIER = /^[A-Za-z0-9._-]{1,128}$/;

function decodeImathasDeploymentReference(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  if (!IMATHAS_IDENTIFIER.test(decoded)) {
    throw new DecodeError(path, "an iMathAS deployment reference");
  }
  return decoded;
}

function decodeImathasItemReference(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  if (!IMATHAS_IDENTIFIER.test(decoded) || decoded.includes("..")) {
    throw new DecodeError(path, "an iMathAS item reference");
  }
  return decoded;
}

function decodeImathasProfile(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  if (!IMATHAS_IDENTIFIER.test(decoded)) {
    throw new DecodeError(path, "an iMathAS profile");
  }
  return decoded;
}

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
  prompt: ReadonlyArray<QuestionContentBlock>;
  response: QuestionResponseFormat;
} {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["workspace", "seed", "title", "prompt", "response"]);
  return {
    workspace: decodeIdentifier(field(record, "workspace", path), `${path}.workspace`),
    seed: decodeNonnegativeInteger(field(record, "seed", path), `${path}.seed`),
    title: decodeNonemptyString(field(record, "title", path), `${path}.title`),
    prompt: decodeArray(field(record, "prompt", path), `${path}.prompt`, (block, blockPath) =>
      decodeQuestionContentBlock(block, blockPath, true),
    ),
    response: decodeQuestionResponseFormat(
      field(record, "response", path),
      `${path}.response`,
      true,
    ),
  };
}

export function decodeQuestionBackendLocator(value: unknown, path: string): QuestionBackendLocator {
  const record = decodeRecord(value, path);
  const backend = decodeString(field(record, "backend", path), `${path}.backend`);
  switch (backend) {
    case "ple": {
      requireOnlyFields(record, path, ["backend"]);
      const decoded = { backend } satisfies QuestionBackendLocator;
      return decoded;
    }
    case "webwork": {
      requireOnlyFields(record, path, ["backend", "pgPath"]);
      const decoded = {
        backend,
        pgPath: decodeNonemptyString(field(record, "pgPath", path), `${path}.pgPath`),
      } satisfies QuestionBackendLocator;
      return decoded;
    }
    case "qti": {
      requireOnlyFields(record, path, ["backend", "itemId"]);
      const decoded = {
        backend,
        itemId: decodeNonemptyString(field(record, "itemId", path), `${path}.itemId`),
      } satisfies QuestionBackendLocator;
      return decoded;
    }
    case "imathas": {
      requireOnlyFields(record, path, [
        "backend",
        "deploymentReference",
        "itemReference",
        "profile",
      ]);
      const decoded = {
        backend,
        deploymentReference: decodeImathasDeploymentReference(
          field(record, "deploymentReference", path),
          `${path}.deploymentReference`,
        ),
        itemReference: decodeImathasItemReference(
          field(record, "itemReference", path),
          `${path}.itemReference`,
        ),
        profile: decodeImathasProfile(field(record, "profile", path), `${path}.profile`),
      } satisfies QuestionBackendLocator;
      return decoded;
    }
    default:
      throw new DecodeError(`${path}.backend`, "a known question backend");
  }
}

export function decodeDraftQuestionBackendLocator(
  value: unknown,
  path: string,
): DraftQuestionBackendLocator {
  const record = decodeRecord(value, path);
  const backend = decodeString(field(record, "backend", path), `${path}.backend`);
  switch (backend) {
    case "ple":
      requireOnlyFields(record, path, ["backend"]);
      return { backend } satisfies DraftQuestionBackendLocator;
    case "webwork":
      requireOnlyFields(record, path, ["backend", "pgPath"]);
      return {
        backend,
        pgPath: decodeNonemptyString(field(record, "pgPath", path), `${path}.pgPath`),
      } satisfies DraftQuestionBackendLocator;
    case "qti":
      requireOnlyFields(record, path, ["backend", "itemId", "importId"]);
      return {
        backend,
        itemId: decodeNonemptyString(field(record, "itemId", path), `${path}.itemId`),
        importId: decodeIdentifier(field(record, "importId", path), `${path}.importId`),
      } satisfies DraftQuestionBackendLocator;
    case "imathas":
      requireOnlyFields(record, path, ["backend", "deploymentReference", "itemReference"]);
      return {
        backend,
        deploymentReference: decodeImathasDeploymentReference(
          field(record, "deploymentReference", path),
          `${path}.deploymentReference`,
        ),
        itemReference: decodeImathasItemReference(
          field(record, "itemReference", path),
          `${path}.itemReference`,
        ),
      } satisfies DraftQuestionBackendLocator;
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

export function decodeQuestionVariationRule(
  value: unknown,
  path: string,
  strict = false,
): QuestionVariationRule {
  const record = decodeRecord(value, path);
  const variationRule = kind(record, path);
  switch (variationRule) {
    case "static":
      if (strict) {
        requireOnlyFields(record, path, ["kind"]);
      }
      return { kind: variationRule };
    case "seeded": {
      if (strict) {
        requireOnlyFields(record, path, ["kind", "generator", "parameters"]);
      }
      const decoded = {
        kind: variationRule,
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
      } satisfies QuestionVariationRule;
      return decoded;
    }
    default:
      throw new DecodeError(`${path}.kind`, "a known Question Variation Rule");
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

export function decodeQuestionGradingRule(
  value: unknown,
  path: string,
  strict = false,
): QuestionGradingRule {
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
      } satisfies QuestionGradingRule;
      return decoded;
    }
    case "ungraded":
      if (strict) {
        requireOnlyFields(record, path, ["mode"]);
      }
      return { mode };
    default:
      throw new DecodeError(`${path}.mode`, "a known Question Grading Rule");
  }
}

/** Strict compact projection for an instructor-owned, unversioned workspace draft. */
export function decodeDraftQuestionSummary(
  value: unknown,
  path = "response",
): DraftQuestionSummary {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "draftQuestion",
    "workspace",
    "authoringWorkspace",
    "title",
    "questionBackend",
  ]);
  return {
    draftQuestion: decodeDraftQuestionReference(
      field(record, "draftQuestion", path),
      `${path}.draftQuestion`,
    ),
    workspace: decodeIdentifier(field(record, "workspace", path), `${path}.workspace`),
    authoringWorkspace: decodeAuthoringWorkspaceReference(
      field(record, "authoringWorkspace", path),
      `${path}.authoringWorkspace`,
    ),
    title: decodeEnvelopeTitle(field(record, "title", path), `${path}.title`),
    questionBackend: decodeStringEnum(
      field(record, "questionBackend", path),
      `${path}.questionBackend`,
      QUESTION_BACKENDS,
    ),
  } satisfies DraftQuestionSummary;
}

export function decodeDraftQuestionPage(
  value: unknown,
  path = "response",
): {
  readonly items: ReadonlyArray<DraftQuestionSummary>;
  readonly nextCursor: string | null;
} {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["items", "nextCursor"]);
  const items = decodeBoundedArray(
    field(record, "items", path),
    `${path}.items`,
    MAX_CURSOR_PAGE_ITEMS,
    decodeDraftQuestionSummary,
  );
  const nextCursor = decodeNullable(
    field(record, "nextCursor", path),
    `${path}.nextCursor`,
    decodeCursor,
  );
  return { items, nextCursor };
}

const PUBLICATION_FIELDS = [
  "questionBackend",
  "title",
  "prompt",
  "response",
  "questionAttemptLimit",
  "questionAttemptTimeLimit",
  "questionVariationRule",
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

/** Exact validation-only 422 body. Capability failures use a distinct violations shape. */
export function decodeQuestionPublicationValidationUnavailable(
  value: unknown,
  path = "response",
): QuestionPublicationValidationUnavailable {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["error"]);
  return {
    kind: "questionPublicationValidationUnavailable",
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

function decodeQuestionPublicationReviewSummary(
  value: unknown,
  path: string,
): QuestionPublicationReviewSummary {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "questionBackend",
    "title",
    "prompt",
    "response",
    "questionAttemptLimit",
    "questionAttemptTimeLimit",
    "questionVariationRule",
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
      "imathasQuestionBackend",
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
  const questionVariationRule = decodeRecord(
    field(record, "questionVariationRule", path),
    `${path}.questionVariationRule`,
  );
  requireOnlyFields(questionVariationRule, `${path}.questionVariationRule`, ["kind"]);
  const metadata = decodeRecord(field(record, "metadata", path), `${path}.metadata`);
  requireOnlyFields(metadata, `${path}.metadata`, [
    "questionDescription",
    "tags",
    "classifications",
    "questionLicense",
    "questionCitation",
    "language",
  ]);
  return {
    questionBackend: decodeStringEnum(
      field(record, "questionBackend", path),
      `${path}.questionBackend`,
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
    questionVariationRule: {
      kind: decodeStringEnum(
        field(questionVariationRule, "kind", `${path}.questionVariationRule`),
        `${path}.questionVariationRule.kind`,
        ["static", "seeded"],
      ),
    },
    metadata: {
      questionDescription: decodeNonemptyString(
        field(metadata, "questionDescription", `${path}.metadata`),
        `${path}.metadata.questionDescription`,
      ),
      tags: decodeBoundedArray(
        field(metadata, "tags", `${path}.metadata`),
        `${path}.metadata.tags`,
        MAX_PUBLICATION_SEMANTIC_ENTRIES,
        decodeString,
      ),
      classifications: decodeBoundedArray(
        field(metadata, "classifications", `${path}.metadata`),
        `${path}.metadata.classifications`,
        MAX_PUBLICATION_SEMANTIC_ENTRIES,
        (classification, classificationPath) =>
          decodeQuestionClassification(classification, classificationPath, true),
      ),
      questionLicense: decodeNullable(
        field(metadata, "questionLicense", `${path}.metadata`),
        `${path}.metadata.questionLicense`,
        decodeQuestionLicense,
      ),
      questionCitation: decodeNullable(
        field(metadata, "questionCitation", `${path}.metadata`),
        `${path}.metadata.questionCitation`,
        decodeQuestionCitation,
      ),
      language: decodeNonemptyString(
        field(metadata, "language", `${path}.metadata`),
        `${path}.metadata.language`,
      ),
    },
  };
}

export function decodeQuestionPublicationReview(
  value: unknown,
  path = "response",
): QuestionPublicationReview {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "draftQuestionRevisionNumber",
    "baseQuestion",
    "current",
    "changed",
  ]);
  const draftQuestionRevisionNumber = decodePositiveInteger(
    field(record, "draftQuestionRevisionNumber", path),
    `${path}.draftQuestionRevisionNumber`,
  );
  const baseQuestion = decodeStringEnum(
    field(record, "baseQuestion", path),
    `${path}.baseQuestion`,
    ["newQuestion"] as const,
  );
  const current = decodeQuestionPublicationReviewSummary(
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
    (baseQuestion === "newQuestion" && changed.length !== 0)
  ) {
    throw new DecodeError(`${path}.changed`, "unique baseline-consistent semantic fields");
  }
  return {
    draftQuestionRevisionNumber,
    revision: `"${draftQuestionRevisionNumber}"`,
    baseQuestion,
    current,
    changed,
  };
}
