import type { QuestionSummary } from "../../../generated/api/QuestionSummary";
import type { QuestionAuthorship } from "../../../generated/api/QuestionAuthorship";
import type { DraftQuestionReference } from "../../../generated/api/DraftQuestionReference";
import { decodeQuestionSummary, isAvailablePleQuestionSummary } from "../../api/decoders";
import { isQuestionAuthorship } from "../../api/question_authorship";
import { PLE_QUESTION_JSON_MEDIA_TYPE, type PleQuestionJsonDocument } from "./question_json_source";
import { parsePleQuestionJsonSource, serializePleQuestionJsonSource } from "./question_json_codec";

const MAX_RESPONSE_BYTES = 4 * 1024 * 1024;
const SAFE_ORIGIN = "https://ple-question-json.invalid";

/** Fetch-compatible dependency for browser code and deterministic Node tests. */
export type PleQuestionJsonFetch = (
  input: RequestInfo | URL,
  init?: RequestInit,
) => Promise<Response>;

export interface PleQuestionJsonClientConfig {
  readonly fetch?: PleQuestionJsonFetch;
  /** Same-origin path prefix only; never an external origin. */
  readonly basePath?: string;
}

export class PleQuestionJsonRequestError extends Error {
  public readonly status: number;
  public readonly path: string;

  public constructor(status: number, path: string) {
    super(`PLE Question JSON request ${path} failed with status ${status}`);
    this.status = status;
    this.path = path;
  }
}

export class PleQuestionJsonConflictError extends PleQuestionJsonRequestError {
  declare public readonly status: 409 | 428;

  public constructor(status: 409 | 428, path: string) {
    super(status, path);
  }
}

export class PleQuestionJsonProtocolError extends Error {}

export type PleQuestionJsonRead = {
  readonly source: PleQuestionJsonDocument;
  readonly revision: string;
};

export type PleQuestionJsonSave = {
  readonly revision: string;
};

export interface PleQuestionJsonClient {
  load(draftQuestion: DraftQuestionReference): Promise<PleQuestionJsonRead>;
  save(
    draftQuestion: DraftQuestionReference,
    source: PleQuestionJsonDocument,
    revision?: string,
  ): Promise<PleQuestionJsonSave>;
  publish(
    draftQuestion: DraftQuestionReference,
    request: { readonly authorship: QuestionAuthorship },
    revision: string,
  ): Promise<QuestionSummary>;
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
    throw new Error(
      "PLE Question JSON basePath must be a same-origin path without query or fragment",
    );
  }
  const normalized = value.replace(/\/+$/, "");
  const resolved = new URL(normalized, SAFE_ORIGIN);
  if (
    resolved.origin !== SAFE_ORIGIN ||
    resolved.pathname !== normalized ||
    resolved.search !== "" ||
    resolved.hash !== ""
  ) {
    throw new Error("PLE Question JSON basePath must be a stable same-origin path");
  }
  return normalized;
}

function encodedId(value: string): string {
  return encodeURIComponent(value);
}

function sourcePath(draftQuestion: DraftQuestionReference): string {
  return `/api/authoring/drafts/${encodedId(draftQuestion)}/source`;
}

function publishPath(draftQuestion: DraftQuestionReference): string {
  return `/api/authoring/drafts/${encodedId(draftQuestion)}/publish`;
}

function publishedQuestionPath(questionId: string): string {
  return `/api/questions/by-id/${encodedId(questionId)}`;
}

/** Proves that every browser-relative request remains under the current origin and base path. */
function sameOriginPath(basePath: string, path: string): string {
  const requestPath = `${basePath}${path}`;
  const resolved = new URL(requestPath, SAFE_ORIGIN);
  if (resolved.origin !== SAFE_ORIGIN || resolved.pathname !== requestPath) {
    throw new PleQuestionJsonProtocolError(
      "PLE Question JSON request path escaped its same-origin base",
    );
  }
  return requestPath;
}

function strongRevision(response: Response, path: string): string {
  const revision = response.headers.get("etag");
  if (revision === null || !/^"[1-9][0-9]*"$/u.test(revision)) {
    throw new PleQuestionJsonProtocolError(
      `PLE Question JSON response ${path} must include one strong numeric ETag`,
    );
  }
  if (BigInt(revision.slice(1, -1)) > 9_223_372_036_854_775_807n) {
    throw new PleQuestionJsonProtocolError(
      `PLE Question JSON response ${path} includes an out-of-range ETag`,
    );
  }
  return revision;
}

function isFlatMediaType(response: Response): boolean {
  const contentType = response.headers.get("content-type");
  if (contentType === null) return false;
  const mediaType = contentType.split(";", 1)[0]?.trim().toLowerCase();
  return mediaType === PLE_QUESTION_JSON_MEDIA_TYPE;
}

function requireJson(response: Response, path: string): void {
  const contentType = response.headers.get("content-type");
  const mediaType = contentType?.split(";", 1)[0]?.trim().toLowerCase();
  if (mediaType !== "application/json") {
    throw new PleQuestionJsonProtocolError(
      `PLE Question JSON response ${path} must use application/json`,
    );
  }
}

async function boundedText(response: Response, path: string): Promise<string> {
  const text = await response.text();
  if (text.length === 0 || new TextEncoder().encode(text).length > MAX_RESPONSE_BYTES) {
    throw new PleQuestionJsonProtocolError(
      `PLE Question JSON response ${path} must contain a bounded body`,
    );
  }
  return text;
}

function decodeJson(text: string, path: string): unknown {
  try {
    return JSON.parse(text);
  } catch {
    throw new PleQuestionJsonProtocolError(`PLE Question JSON response ${path} is not valid JSON`);
  }
}

function requestInit(
  method: "GET" | "PUT" | "POST",
  headers: Record<string, string>,
  body?: string,
): RequestInit {
  return { method, headers, body, credentials: "same-origin", cache: "no-store" };
}

/** Client for the protected ple-question-json source, save, and publication endpoints. */
export function createPleQuestionJsonClient(
  config: PleQuestionJsonClientConfig = {},
): PleQuestionJsonClient {
  const fetchImplementation = config.fetch ?? browserFetch;
  const basePath = normalizeBasePath(config.basePath);

  async function load(draftQuestion: DraftQuestionReference): Promise<PleQuestionJsonRead> {
    const path = sourcePath(draftQuestion);
    const requestPath = sameOriginPath(basePath, path);
    const response = await fetchImplementation(
      requestPath,
      requestInit("GET", {
        accept: PLE_QUESTION_JSON_MEDIA_TYPE,
      }),
    );
    if (response.status === 409 || response.status === 428)
      throw new PleQuestionJsonConflictError(response.status, path);
    if (!response.ok) throw new PleQuestionJsonRequestError(response.status, path);
    if (!isFlatMediaType(response)) {
      throw new PleQuestionJsonProtocolError(
        `PLE Question JSON response ${path} must use ${PLE_QUESTION_JSON_MEDIA_TYPE}`,
      );
    }
    const text = await boundedText(response, path);
    return { source: parsePleQuestionJsonSource(text), revision: strongRevision(response, path) };
  }

  async function save(
    draftQuestion: DraftQuestionReference,
    source: PleQuestionJsonDocument,
    revision?: string,
  ): Promise<PleQuestionJsonSave> {
    const path = sourcePath(draftQuestion);
    const requestPath = sameOriginPath(basePath, path);
    const headers: Record<string, string> = {
      accept: "application/json",
      "content-type": PLE_QUESTION_JSON_MEDIA_TYPE,
    };
    if (revision !== undefined) headers["if-match"] = validRevision(revision);
    const response = await fetchImplementation(
      requestPath,
      requestInit("PUT", headers, serializePleQuestionJsonSource(source)),
    );
    if (response.status === 409 || response.status === 428)
      throw new PleQuestionJsonConflictError(response.status, path);
    if (!response.ok) throw new PleQuestionJsonRequestError(response.status, path);
    if (response.status !== 204) {
      throw new PleQuestionJsonProtocolError(
        `PLE Question JSON save ${path} must return no content`,
      );
    }
    return { revision: strongRevision(response, path) };
  }

  async function publish(
    draftQuestion: DraftQuestionReference,
    request: { readonly authorship: QuestionAuthorship },
    revision: string,
  ): Promise<QuestionSummary> {
    if (!isQuestionAuthorship(request.authorship)) {
      throw new PleQuestionJsonProtocolError(
        "PLE Question JSON publication requires one to sixteen reviewed Question Authors",
      );
    }
    const path = publishPath(draftQuestion);
    const requestPath = sameOriginPath(basePath, path);
    const response = await fetchImplementation(
      requestPath,
      requestInit(
        "POST",
        {
          accept: "application/json",
          "content-type": "application/json",
          "if-match": validRevision(revision),
        },
        JSON.stringify({
          authors: request.authorship.authors.map((author) => author.displayName),
        }),
      ),
    );
    if (response.status === 409 || response.status === 428)
      throw new PleQuestionJsonConflictError(response.status, path);
    if (!response.ok) throw new PleQuestionJsonRequestError(response.status, path);
    requireJson(response, path);
    const questionId = publishedQuestionId(
      decodeJson(await boundedText(response, path), path),
      path,
    );
    const summaryPath = publishedQuestionPath(questionId);
    const summaryResponse = await fetchImplementation(
      sameOriginPath(basePath, summaryPath),
      requestInit("GET", { accept: "application/json" }),
    );
    if (!summaryResponse.ok)
      throw new PleQuestionJsonRequestError(summaryResponse.status, summaryPath);
    requireJson(summaryResponse, summaryPath);
    const summary = decodeQuestionSummary(
      decodeJson(await boundedText(summaryResponse, summaryPath), summaryPath),
      summaryPath,
      true,
    );
    if (!isAvailablePleQuestionSummary(summary)) {
      throw new PleQuestionJsonProtocolError(
        "PLE Question JSON publication response must be an available PLE Question Library summary",
      );
    }
    return summary;
  }

  return { load, save, publish };
}

function publishedQuestionId(value: unknown, path: string): string {
  const questionId =
    typeof value === "object" && value !== null && !Array.isArray(value)
      ? (value as Record<string, unknown>).questionId
      : undefined;
  if (
    typeof value !== "object" ||
    value === null ||
    Array.isArray(value) ||
    Object.keys(value).length !== 1 ||
    typeof questionId !== "string"
  ) {
    throw new PleQuestionJsonProtocolError(
      `PLE Question JSON publication ${path} must return only a Question ID`,
    );
  }
  return questionId;
}

function validRevision(value: string): string {
  if (!/^"[1-9][0-9]*"$/u.test(value)) {
    throw new PleQuestionJsonProtocolError(
      "PLE Question JSON revision must be one positive strong numeric ETag",
    );
  }
  if (BigInt(value.slice(1, -1)) > 9_223_372_036_854_775_807n) {
    throw new PleQuestionJsonProtocolError(
      "PLE Question JSON revision must fit in a signed 64-bit integer",
    );
  }
  return value;
}
