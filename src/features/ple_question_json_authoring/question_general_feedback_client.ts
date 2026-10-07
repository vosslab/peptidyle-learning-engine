import type { DraftQuestionRouteId } from "../../navigation/public_route";
import {
  ifMatchHeaderForPositiveNumber,
  numberFromQuotedPositiveHeader,
} from "../../api/http_client/conditional_request";
import { ApiProtocolError } from "../../api/http_client/error";
import { decodeQuestionCitation } from "../../api/decoders/shared";
import { decodeString } from "../../api/decoder";
import type { QuestionLicense } from "../../../generated/api/QuestionLicense";
import type { QuestionType } from "../../../generated/api/QuestionType";
import type { PleQuestionJsonRecordMetadata } from "./question_json_defaults";

const MAX_RESPONSE_BYTES = 4 * 1024 * 1024;
const SAFE_ORIGIN = "https://ple-question-general-feedback.invalid";

export type PleQuestionManagedSupport = {
  readonly generalFeedback: string | null;
  readonly hint: string | null;
  readonly workedSolution: string | null;
};

export type PleQuestionGeneralFeedbackRead = PleQuestionManagedSupport & {
  readonly metadata: PleQuestionJsonRecordMetadata;
  readonly questionType: QuestionType | null;
  readonly authors: ReadonlyArray<string>;
  readonly draftQuestionEditNumber: string;
};

/** Empty authored support is absence. A non-empty value is stored as entered. ASVS 2.2.1. */
export function optionalPleManagedSupportText(value: string): string | null {
  return value.trim() === "" ? null : value;
}

export type PleQuestionGeneralFeedbackSave = {
  readonly draftQuestionEditNumber: string;
};

export type PleQuestionGeneralFeedbackFetch = (
  input: RequestInfo | URL,
  init?: RequestInit,
) => Promise<Response>;

export interface PleQuestionGeneralFeedbackClientConfig {
  readonly fetch?: PleQuestionGeneralFeedbackFetch;
  /** Same-origin path prefix only; never an external origin. */
  readonly basePath?: string;
}

export class PleQuestionGeneralFeedbackRequestError extends Error {
  public readonly status: number;
  public readonly path: string;

  public constructor(status: number, path: string) {
    super(`Question general feedback request ${path} failed with status ${status}`);
    this.status = status;
    this.path = path;
  }
}

export class PleQuestionGeneralFeedbackConflictError extends PleQuestionGeneralFeedbackRequestError {
  declare public readonly status: 409 | 412 | 428;

  public constructor(status: 409 | 412 | 428, path: string) {
    super(status, path);
  }
}

export class PleQuestionGeneralFeedbackProtocolError extends Error {}

export interface PleQuestionGeneralFeedbackClient {
  load(draftQuestion: DraftQuestionRouteId): Promise<PleQuestionGeneralFeedbackRead>;
  save(
    draftQuestion: DraftQuestionRouteId,
    metadata: PleQuestionJsonRecordMetadata,
    support: PleQuestionManagedSupport,
    expectedDraftQuestionEditNumber: string,
    questionType?: QuestionType | null,
  ): Promise<PleQuestionGeneralFeedbackSave>;
}

const QUESTION_TYPES = [
  "multipleChoice",
  "multipleAnswer",
  "fillInBlank",
  "multipleFillInBlank",
  "numeric",
  "matching",
  "ordering",
  "hotspot",
] as const satisfies ReadonlyArray<QuestionType>;

export function questionTypeUpdateForBackend(
  backend: "ple" | "webwork",
  selectedQuestionType: QuestionType | null,
): QuestionType | null | undefined {
  return backend === "webwork" ? selectedQuestionType : undefined;
}

function browserFetch(input: RequestInfo | URL, init?: RequestInit): Promise<Response> {
  return globalThis.fetch(input, init);
}

function hasAsciiControl(value: string): boolean {
  for (const character of value) {
    const codePoint = character.codePointAt(0);
    if (codePoint !== undefined && (codePoint <= 0x1f || codePoint === 0x7f)) return true;
  }
  return false;
}

function normalizeBasePath(value: string | undefined): string {
  if (value === undefined || value === "" || value === "/") return "";
  if (
    !value.startsWith("/") ||
    value.startsWith("//") ||
    value.includes("\\") ||
    hasAsciiControl(value) ||
    value.includes("?") ||
    value.includes("#")
  ) {
    throw new Error("Question general feedback basePath must be a stable same-origin path");
  }
  const normalized = value.replace(/\/+$/, "");
  const resolved = new URL(normalized, SAFE_ORIGIN);
  if (
    resolved.origin !== SAFE_ORIGIN ||
    resolved.pathname !== normalized ||
    resolved.search !== "" ||
    resolved.hash !== ""
  ) {
    throw new Error("Question general feedback basePath must be a stable same-origin path");
  }
  return normalized;
}

function metadataPath(draftQuestion: DraftQuestionRouteId): string {
  return `/api/authoring/drafts/${encodeURIComponent(draftQuestion)}/metadata`;
}

function sameOriginPath(basePath: string, path: string): string {
  const requestPath = `${basePath}${path}`;
  const resolved = new URL(requestPath, SAFE_ORIGIN);
  if (resolved.origin !== SAFE_ORIGIN || resolved.pathname !== requestPath) {
    throw new PleQuestionGeneralFeedbackProtocolError(
      "Question general feedback request path escaped its same-origin base",
    );
  }
  return requestPath;
}

function draftQuestionEditNumberFromResponse(response: Response, path: string): string {
  try {
    return numberFromQuotedPositiveHeader(response, path, "Draft Question Edit Number");
  } catch (error: unknown) {
    throw new PleQuestionGeneralFeedbackProtocolError(
      error instanceof ApiProtocolError
        ? error.message
        : `Question general feedback response ${path} must include one Draft Question Edit Number`,
    );
  }
}

function ifMatchDraftQuestionEditNumber(value: string, path: string): string {
  try {
    return ifMatchHeaderForPositiveNumber(value, path, "Draft Question Edit Number");
  } catch (error: unknown) {
    throw new PleQuestionGeneralFeedbackProtocolError(
      error instanceof ApiProtocolError
        ? error.message
        : "Draft Question Edit Number must be a positive integer",
    );
  }
}

function requireJson(response: Response, path: string): void {
  const mediaType = response.headers.get("content-type")?.split(";", 1)[0]?.trim().toLowerCase();
  if (mediaType !== "application/json") {
    throw new PleQuestionGeneralFeedbackProtocolError(
      `Question general feedback response ${path} must use application/json`,
    );
  }
}

async function boundedJson(response: Response, path: string): Promise<unknown> {
  const text = await response.text();
  if (text.length === 0 || new TextEncoder().encode(text).length > MAX_RESPONSE_BYTES) {
    throw new PleQuestionGeneralFeedbackProtocolError(
      `Question general feedback response ${path} must contain a bounded body`,
    );
  }
  try {
    return JSON.parse(text);
  } catch {
    throw new PleQuestionGeneralFeedbackProtocolError(
      `Question general feedback response ${path} is not valid JSON`,
    );
  }
}

function optionalSupportText(value: unknown): string | null | undefined {
  if (value === null) return null;
  if (typeof value === "string") return value;
  return undefined;
}

function decodeQuestionType(value: unknown): QuestionType | null | undefined {
  if (value === null) return null;
  if (typeof value !== "string") return undefined;
  return QUESTION_TYPES.find((questionType) => questionType === value);
}

function decodeCitation(value: unknown): string | null | undefined {
  if (value === null) return null;
  try {
    return decodeQuestionCitation(value, "Question metadata.questionCitation");
  } catch {
    return undefined;
  }
}

function isQuestionLicense(value: unknown): value is QuestionLicense | null {
  return value === null || value === "CC0-1.0" || value === "CC-BY-4.0" || value === "CC-BY-SA-4.0";
}

function decodeRecordMetadata(value: unknown, path: string): PleQuestionJsonRecordMetadata {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new PleQuestionGeneralFeedbackProtocolError(`Question metadata ${path} is invalid`);
  }
  const record = value as Record<string, unknown>;
  const questionCitation = decodeCitation(record.questionCitation);
  const questionLicense = record.questionLicense;
  if (
    Object.keys(record).length !== 6 ||
    typeof record.questionTitle !== "string" ||
    record.questionTitle.length > 512 ||
    typeof record.questionDescription !== "string" ||
    record.questionDescription.length > 4_000 ||
    !Array.isArray(record.tags) ||
    !record.tags.every((tag) => typeof tag === "string") ||
    !isQuestionLicense(questionLicense) ||
    questionCitation === undefined ||
    !(record.language === null || typeof record.language === "string")
  ) {
    throw new PleQuestionGeneralFeedbackProtocolError(`Question metadata ${path} is invalid`);
  }
  return {
    questionTitle: record.questionTitle,
    questionDescription: record.questionDescription,
    tags: record.tags,
    questionLicense,
    questionCitation,
    language: record.language,
  };
}

function decodeMetadata(
  value: unknown,
  path: string,
): PleQuestionManagedSupport & {
  metadata: PleQuestionJsonRecordMetadata;
  questionType: QuestionType | null;
  authors: ReadonlyArray<string>;
} {
  // ASVS 15.3.1: expose only the ordinary record metadata and support fields used by this editor.
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new PleQuestionGeneralFeedbackProtocolError(
      `Question general feedback response ${path} must contain generalFeedback, hint, and workedSolution`,
    );
  }
  const record = value as Record<string, unknown>;
  const metadata = decodeRecordMetadata(record.metadata, `${path}.metadata`);
  const generalFeedback = optionalSupportText(record.generalFeedback);
  const hint = optionalSupportText(record.hint);
  const workedSolution = optionalSupportText(record.workedSolution);
  const questionType = decodeQuestionType(record.questionType);
  const authors = Array.isArray(record.authors)
    ? record.authors.map((author, index) => decodeString(author, `${path}.authors[${index}]`))
    : undefined;
  if (
    Object.keys(record).length !== 6 ||
    generalFeedback === undefined ||
    hint === undefined ||
    workedSolution === undefined ||
    questionType === undefined ||
    authors === undefined ||
    authors.length > 16
  ) {
    throw new PleQuestionGeneralFeedbackProtocolError(
      `Question general feedback response ${path} must contain generalFeedback, hint, and workedSolution`,
    );
  }
  return { metadata, questionType, generalFeedback, hint, workedSolution, authors };
}

function requestInit(
  method: "GET" | "PUT",
  headers: Record<string, string>,
  body?: string,
): RequestInit {
  return { method, headers, body, credentials: "same-origin", cache: "no-store" };
}

/** Client for PLE-managed authored feedback, deliberately separate from backend interaction feedback. */
export function createPleQuestionGeneralFeedbackClient(
  config: PleQuestionGeneralFeedbackClientConfig = {},
): PleQuestionGeneralFeedbackClient {
  const fetchImplementation = config.fetch ?? browserFetch;
  const basePath = normalizeBasePath(config.basePath);

  async function load(
    draftQuestion: DraftQuestionRouteId,
  ): Promise<PleQuestionGeneralFeedbackRead> {
    const path = metadataPath(draftQuestion);
    const response = await fetchImplementation(
      sameOriginPath(basePath, path),
      requestInit("GET", { accept: "application/json" }),
    );
    if (response.status === 409 || response.status === 412 || response.status === 428) {
      throw new PleQuestionGeneralFeedbackConflictError(response.status, path);
    }
    if (!response.ok) throw new PleQuestionGeneralFeedbackRequestError(response.status, path);
    requireJson(response, path);
    return {
      ...decodeMetadata(await boundedJson(response, path), path),
      draftQuestionEditNumber: draftQuestionEditNumberFromResponse(response, path),
    };
  }

  async function save(
    draftQuestion: DraftQuestionRouteId,
    metadata: PleQuestionJsonRecordMetadata,
    support: PleQuestionManagedSupport,
    expectedDraftQuestionEditNumber: string,
    questionType?: QuestionType | null,
  ): Promise<PleQuestionGeneralFeedbackSave> {
    if (
      questionType !== undefined &&
      questionType !== null &&
      !QUESTION_TYPES.some((supportedType) => supportedType === questionType)
    ) {
      throw new PleQuestionGeneralFeedbackProtocolError(
        "Question Type must be one of the supported values",
      );
    }
    const path = metadataPath(draftQuestion);
    const response = await fetchImplementation(
      sameOriginPath(basePath, path),
      requestInit(
        "PUT",
        {
          accept: "application/json",
          "content-type": "application/json",
          "if-match": ifMatchDraftQuestionEditNumber(expectedDraftQuestionEditNumber, path),
        },
        // ASVS 15.3.3: send only the metadata, support, and explicitly supplied Type fields.
        JSON.stringify({
          metadata: {
            questionTitle: metadata.questionTitle,
            questionDescription: metadata.questionDescription,
            tags: [...metadata.tags],
            questionLicense: metadata.questionLicense,
            questionCitation: metadata.questionCitation,
            language: metadata.language,
          },
          generalFeedback: support.generalFeedback,
          hint: support.hint,
          workedSolution: support.workedSolution,
          ...(questionType === undefined ? {} : { questionType }),
        }),
      ),
    );
    if (response.status === 409 || response.status === 412 || response.status === 428) {
      throw new PleQuestionGeneralFeedbackConflictError(response.status, path);
    }
    if (!response.ok) throw new PleQuestionGeneralFeedbackRequestError(response.status, path);
    if (response.status !== 204) {
      throw new PleQuestionGeneralFeedbackProtocolError(
        `Question general feedback save ${path} must return no content`,
      );
    }
    return { draftQuestionEditNumber: draftQuestionEditNumberFromResponse(response, path) };
  }

  return { load, save };
}
