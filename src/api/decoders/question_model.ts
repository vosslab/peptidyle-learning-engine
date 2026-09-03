// Authoring and publication decoders.

import type { QuestionAttemptLimit } from "../../../generated/api/QuestionAttemptLimit";
import type { QuestionContentBlock } from "../../../generated/api/QuestionContentBlock";
import type { DraftImathasQuestionBackendBinding } from "../../../generated/api/DraftImathasQuestionBackendBinding";
import type { QuestionGradingRule } from "../../../generated/api/QuestionGradingRule";
import type { ImathasQuestionBackendBinding } from "../../../generated/api/ImathasQuestionBackendBinding";
import type { QuestionBackend } from "../../../generated/api/QuestionBackend";
import type { QuestionRevision } from "../../../generated/api/QuestionRevision";
import type { DraftQuestionContent } from "../../../generated/api/DraftQuestionContent";
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
  decodeFiniteNumber,
  decodeNonemptyString,
  decodeNonnegativeInteger,
  decodeNullable,
  decodePositiveInteger,
  decodeRecord,
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
  decodeQuestionTitle,
  decodeIdentifier,
  decodeQuestionLicense,
  decodeQuestionCitation,
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

/** Strict key-free static PLE Question JSON Draft Question Preview. */
export function decodeKeyFreeDraftPreview(
  value: unknown,
  path = "wasmPreview",
): {
  workspace: string;
  title: string;
  prompt: ReadonlyArray<QuestionContentBlock>;
  response: QuestionResponseFormat;
} {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["workspace", "title", "prompt", "response"]);
  return {
    workspace: decodeIdentifier(field(record, "workspace", path), `${path}.workspace`),
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

function decodeQuestionBackend(value: unknown, path: string): QuestionBackend {
  return decodeStringEnum(value, path, QUESTION_BACKENDS);
}

function decodeDraftImathasQuestionBackendBinding(
  value: unknown,
  path: string,
): DraftImathasQuestionBackendBinding {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["deploymentReference", "itemReference"]);
  return {
    deploymentReference: decodeImathasDeploymentReference(
      field(record, "deploymentReference", path),
      `${path}.deploymentReference`,
    ),
    itemReference: decodeImathasItemReference(
      field(record, "itemReference", path),
      `${path}.itemReference`,
    ),
  } satisfies DraftImathasQuestionBackendBinding;
}

function decodeImathasQuestionBackendBinding(
  value: unknown,
  path: string,
): ImathasQuestionBackendBinding {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["deploymentReference", "itemReference", "profile"]);
  return {
    deploymentReference: decodeImathasDeploymentReference(
      field(record, "deploymentReference", path),
      `${path}.deploymentReference`,
    ),
    itemReference: decodeImathasItemReference(
      field(record, "itemReference", path),
      `${path}.itemReference`,
    ),
    profile: decodeImathasProfile(field(record, "profile", path), `${path}.profile`),
  } satisfies ImathasQuestionBackendBinding;
}

function questionBackendFieldsAreAllowed(
  questionBackend: QuestionBackend,
  webworkPgPath: string | null,
  qtiPackageItemIdentifier: string | null,
  workspaceImportId: string | null,
  imathasQuestionBackendBinding: ImathasQuestionBackendBinding | null,
  draftImathasQuestionBackendBinding: DraftImathasQuestionBackendBinding | null,
  path: string,
  draft: boolean,
): void {
  const invalid = (): never => {
    throw new DecodeError(path, "the exact fields permitted for its Question Backend");
  };
  switch (questionBackend) {
    case "ple":
      if (
        webworkPgPath !== null ||
        qtiPackageItemIdentifier !== null ||
        workspaceImportId !== null ||
        imathasQuestionBackendBinding !== null ||
        draftImathasQuestionBackendBinding !== null
      ) {
        invalid();
      }
      return;
    case "webwork":
      if (
        webworkPgPath === null ||
        qtiPackageItemIdentifier !== null ||
        workspaceImportId !== null ||
        imathasQuestionBackendBinding !== null ||
        draftImathasQuestionBackendBinding !== null
      ) {
        invalid();
      }
      return;
    case "qti":
      if (
        qtiPackageItemIdentifier === null ||
        (draft && workspaceImportId === null) ||
        (!draft && workspaceImportId !== null) ||
        webworkPgPath !== null ||
        imathasQuestionBackendBinding !== null ||
        draftImathasQuestionBackendBinding !== null
      ) {
        invalid();
      }
      return;
    case "imathas":
      if (
        webworkPgPath !== null ||
        qtiPackageItemIdentifier !== null ||
        workspaceImportId !== null ||
        (draft
          ? imathasQuestionBackendBinding !== null || draftImathasQuestionBackendBinding === null
          : imathasQuestionBackendBinding === null || draftImathasQuestionBackendBinding !== null)
      ) {
        invalid();
      }
      return;
  }
}

export function decodeQuestionRevisionBackendFields(
  record: Record<string, unknown>,
  path: string,
): Pick<
  QuestionRevision,
  "questionBackend" | "webworkPgPath" | "qtiPackageItemIdentifier" | "imathasQuestionBackendBinding"
> {
  const questionBackend = decodeQuestionBackend(
    field(record, "questionBackend", path),
    `${path}.questionBackend`,
  );
  const webworkPgPath = decodeNullable(
    field(record, "webworkPgPath", path),
    `${path}.webworkPgPath`,
    decodeNonemptyString,
  );
  const qtiPackageItemIdentifier = decodeNullable(
    field(record, "qtiPackageItemIdentifier", path),
    `${path}.qtiPackageItemIdentifier`,
    decodeNonemptyString,
  );
  const imathasQuestionBackendBinding = decodeNullable(
    field(record, "imathasQuestionBackendBinding", path),
    `${path}.imathasQuestionBackendBinding`,
    decodeImathasQuestionBackendBinding,
  );
  questionBackendFieldsAreAllowed(
    questionBackend,
    webworkPgPath,
    qtiPackageItemIdentifier,
    null,
    imathasQuestionBackendBinding,
    null,
    path,
    false,
  );
  return {
    questionBackend,
    webworkPgPath,
    qtiPackageItemIdentifier,
    imathasQuestionBackendBinding,
  };
}

export function decodeDraftQuestionBackendFields(
  record: Record<string, unknown>,
  path: string,
): Pick<
  DraftQuestionContent,
  | "questionBackend"
  | "webworkPgPath"
  | "qtiPackageItemIdentifier"
  | "workspaceImportId"
  | "draftImathasQuestionBackendBinding"
> {
  const questionBackend = decodeQuestionBackend(
    field(record, "questionBackend", path),
    `${path}.questionBackend`,
  );
  const webworkPgPath = decodeNullable(
    field(record, "webworkPgPath", path),
    `${path}.webworkPgPath`,
    decodeNonemptyString,
  );
  const qtiPackageItemIdentifier = decodeNullable(
    field(record, "qtiPackageItemIdentifier", path),
    `${path}.qtiPackageItemIdentifier`,
    decodeNonemptyString,
  );
  const workspaceImportId = decodeNullable(
    field(record, "workspaceImportId", path),
    `${path}.workspaceImportId`,
    decodeIdentifier,
  );
  const draftImathasQuestionBackendBinding = decodeNullable(
    field(record, "draftImathasQuestionBackendBinding", path),
    `${path}.draftImathasQuestionBackendBinding`,
    decodeDraftImathasQuestionBackendBinding,
  );
  questionBackendFieldsAreAllowed(
    questionBackend,
    webworkPgPath,
    qtiPackageItemIdentifier,
    workspaceImportId,
    null,
    draftImathasQuestionBackendBinding,
    path,
    true,
  );
  return {
    questionBackend,
    webworkPgPath,
    qtiPackageItemIdentifier,
    workspaceImportId,
    draftImathasQuestionBackendBinding,
  };
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

/** Strict compact Draft Question Summary for an instructor-owned, unversioned workspace draft. */
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
    title: decodeQuestionTitle(field(record, "title", path), `${path}.title`),
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
          title: decodeQuestionTitle(field(violation, "title", entryPath), `${entryPath}.title`),
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
  const metadata = decodeRecord(field(record, "metadata", path), `${path}.metadata`);
  requireOnlyFields(metadata, `${path}.metadata`, [
    "questionDescription",
    "tags",
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
    title: decodeQuestionTitle(field(record, "title", path), `${path}.title`),
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
    "draftQuestionEditNumber",
    "baseQuestion",
    "current",
    "changed",
  ]);
  const draftQuestionEditNumber = decodePositiveInteger(
    field(record, "draftQuestionEditNumber", path),
    `${path}.draftQuestionEditNumber`,
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
    draftQuestionEditNumber,
    revision: `"${draftQuestionEditNumber}"`,
    baseQuestion,
    current,
    changed,
  };
}
