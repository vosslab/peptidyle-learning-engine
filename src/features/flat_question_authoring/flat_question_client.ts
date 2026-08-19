import type { DraftQuestionDefinition } from "../../../generated/api/DraftQuestionDefinition";
import type { CatalogProblemSummary } from "../../../generated/api/CatalogProblemSummary";
import type { PublicationScope } from "../../../generated/api/PublicationScope";
import type { PublicByline } from "../../../generated/api/PublicByline";
import type { WorkspaceId } from "../../../generated/api/WorkspaceId";
import {
  decodeCatalogProblemSummary,
  decodeDraftQuestionDefinition,
  isPublishedNativeCatalogProblemSummary,
} from "../../api/decoders";
import { isPublicByline } from "../../api/public_byline";
import {
  FLAT_QUESTION_FILL_IN_FAMILY,
  FLAT_QUESTION_HOTSPOT_FAMILY,
  FLAT_QUESTION_MATCHING_FAMILY,
  FLAT_QUESTION_MEDIA_TYPE,
  FLAT_QUESTION_MULTI_FILL_IN_FAMILY,
  FLAT_QUESTION_MULTIPLE_ANSWER_FAMILY,
  FLAT_QUESTION_NUMERIC_FAMILY,
  FLAT_QUESTION_ORDERING_FAMILY,
  FLAT_QUESTION_SINGLE_CHOICE_FAMILY,
  type FlatQuestionSourceV2,
} from "./flat_question_source";
import { parseFlatQuestionSource, serializeFlatQuestionSource } from "./flat_question_codec";

const MAX_RESPONSE_BYTES = 4 * 1024 * 1024;
const SAFE_ORIGIN = "https://flat-question.invalid";

/** Fetch-compatible dependency for browser code and deterministic Node tests. */
export type FlatQuestionFetch = (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>;

export interface FlatQuestionClientConfig {
  readonly fetch?: FlatQuestionFetch;
  /** Same-origin path prefix only; never an external origin. */
  readonly basePath?: string;
}

export class FlatQuestionRequestError extends Error {
  public readonly status: number;
  public readonly path: string;

  public constructor(status: number, path: string) {
    super(`Flat-question request ${path} failed with status ${status}`);
    this.status = status;
    this.path = path;
  }
}

export class FlatQuestionConflictError extends FlatQuestionRequestError {
  declare public readonly status: 409 | 428;

  public constructor(status: 409 | 428, path: string) {
    super(status, path);
  }
}

export class FlatQuestionProtocolError extends Error {}

export type FlatQuestionRead = {
  readonly source: FlatQuestionSourceV2;
  readonly revision: string;
};

export type FlatQuestionSave = {
  readonly draft: DraftQuestionDefinition;
  readonly revision: string;
};

export interface FlatQuestionClient {
  load(workspace: WorkspaceId): Promise<FlatQuestionRead>;
  save(
    workspace: WorkspaceId,
    source: FlatQuestionSourceV2,
    revision?: string,
  ): Promise<FlatQuestionSave>;
  publish(
    workspace: WorkspaceId,
    request: { readonly scope: PublicationScope; readonly byline: PublicByline },
    revision: string,
  ): Promise<CatalogProblemSummary>;
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
    throw new Error("Flat-question basePath must be a same-origin path without query or fragment");
  }
  const normalized = value.replace(/\/+$/, "");
  const resolved = new URL(normalized, SAFE_ORIGIN);
  if (
    resolved.origin !== SAFE_ORIGIN ||
    resolved.pathname !== normalized ||
    resolved.search !== "" ||
    resolved.hash !== ""
  ) {
    throw new Error("Flat-question basePath must be a stable same-origin path");
  }
  return normalized;
}

function encodedId(value: string): string {
  return encodeURIComponent(value);
}

function sourcePath(workspace: WorkspaceId): string {
  return `/api/workspaces/${encodedId(workspace)}/flat-question`;
}

function publishPath(workspace: WorkspaceId): string {
  return `/api/problems/${encodedId(workspace)}/flat-question-publish`;
}

/** Proves that every browser-relative request remains under the current origin and base path. */
function sameOriginPath(basePath: string, path: string): string {
  const requestPath = `${basePath}${path}`;
  const resolved = new URL(requestPath, SAFE_ORIGIN);
  if (resolved.origin !== SAFE_ORIGIN || resolved.pathname !== requestPath) {
    throw new FlatQuestionProtocolError("Flat-question request path escaped its same-origin base");
  }
  return requestPath;
}

function strongRevision(response: Response, path: string): string {
  const revision = response.headers.get("etag");
  if (revision === null || !/^"[1-9][0-9]*"$/u.test(revision)) {
    throw new FlatQuestionProtocolError(
      `Flat-question response ${path} must include one strong numeric ETag`,
    );
  }
  if (BigInt(revision.slice(1, -1)) > 9_223_372_036_854_775_807n) {
    throw new FlatQuestionProtocolError(
      `Flat-question response ${path} includes an out-of-range ETag`,
    );
  }
  return revision;
}

function isFlatMediaType(response: Response): boolean {
  const contentType = response.headers.get("content-type");
  if (contentType === null) return false;
  const mediaType = contentType.split(";", 1)[0]?.trim().toLowerCase();
  return mediaType === FLAT_QUESTION_MEDIA_TYPE;
}

function requireJson(response: Response, path: string): void {
  const contentType = response.headers.get("content-type");
  const mediaType = contentType?.split(";", 1)[0]?.trim().toLowerCase();
  if (mediaType !== "application/json") {
    throw new FlatQuestionProtocolError(`Flat-question response ${path} must use application/json`);
  }
}

async function boundedText(response: Response, path: string): Promise<string> {
  const text = await response.text();
  if (text.length === 0 || new TextEncoder().encode(text).length > MAX_RESPONSE_BYTES) {
    throw new FlatQuestionProtocolError(
      `Flat-question response ${path} must contain a bounded body`,
    );
  }
  return text;
}

function decodeJson(text: string, path: string): unknown {
  try {
    return JSON.parse(text);
  } catch {
    throw new FlatQuestionProtocolError(`Flat-question response ${path} is not valid JSON`);
  }
}

function requireFlatNativeSource(
  source: DraftQuestionDefinition["source"],
  response: FlatQuestionSourceV2["response"] | undefined,
  responseKind: "save" | "publication",
): void {
  const expectedFamily = response === undefined ? undefined : familyForResponse(response);
  const expectedDescription = expectedFamily ?? supportedFamilyDescription();
  if (source.backend !== "native") {
    throw new FlatQuestionProtocolError(
      `Flat-question ${responseKind} response must use native ${expectedDescription}`,
    );
  }
  const acceptedFamily =
    expectedFamily === undefined
      ? isFlatQuestionFamily(source.family)
      : source.family === expectedFamily;
  if (!acceptedFamily) {
    throw new FlatQuestionProtocolError(
      `Flat-question ${responseKind} response must use native ${expectedDescription}`,
    );
  }
}

function familyForResponse(response: FlatQuestionSourceV2["response"]): string {
  switch (response.kind) {
    case "singleChoice":
      return FLAT_QUESTION_SINGLE_CHOICE_FAMILY;
    case "multipleAnswer":
      return FLAT_QUESTION_MULTIPLE_ANSWER_FAMILY;
    case "fillIn":
      return FLAT_QUESTION_FILL_IN_FAMILY;
    case "multiFillIn":
      return FLAT_QUESTION_MULTI_FILL_IN_FAMILY;
    case "numeric":
      return FLAT_QUESTION_NUMERIC_FAMILY;
    case "matching":
      return FLAT_QUESTION_MATCHING_FAMILY;
    case "ordering":
      return FLAT_QUESTION_ORDERING_FAMILY;
    case "hotspot":
      return FLAT_QUESTION_HOTSPOT_FAMILY;
  }
}

function isFlatQuestionFamily(family: string): boolean {
  return supportedFamilies().includes(family);
}

function supportedFamilies(): ReadonlyArray<string> {
  return [
    FLAT_QUESTION_SINGLE_CHOICE_FAMILY,
    FLAT_QUESTION_MULTIPLE_ANSWER_FAMILY,
    FLAT_QUESTION_FILL_IN_FAMILY,
    FLAT_QUESTION_MULTI_FILL_IN_FAMILY,
    FLAT_QUESTION_NUMERIC_FAMILY,
    FLAT_QUESTION_MATCHING_FAMILY,
    FLAT_QUESTION_ORDERING_FAMILY,
    FLAT_QUESTION_HOTSPOT_FAMILY,
  ];
}

function supportedFamilyDescription(): string {
  return supportedFamilies().join(" or ");
}

function requestInit(
  method: "GET" | "PUT" | "POST",
  headers: Record<string, string>,
  body?: string,
): RequestInit {
  return { method, headers, body, credentials: "same-origin", cache: "no-store" };
}

/** Client for the protected flat-question source, save, and publication endpoints. */
export function createFlatQuestionClient(
  config: FlatQuestionClientConfig = {},
): FlatQuestionClient {
  const fetchImplementation = config.fetch ?? browserFetch;
  const basePath = normalizeBasePath(config.basePath);

  async function load(workspace: WorkspaceId): Promise<FlatQuestionRead> {
    const path = sourcePath(workspace);
    const requestPath = sameOriginPath(basePath, path);
    const response = await fetchImplementation(
      requestPath,
      requestInit("GET", {
        accept: FLAT_QUESTION_MEDIA_TYPE,
      }),
    );
    if (response.status === 409 || response.status === 428)
      throw new FlatQuestionConflictError(response.status, path);
    if (!response.ok) throw new FlatQuestionRequestError(response.status, path);
    if (!isFlatMediaType(response)) {
      throw new FlatQuestionProtocolError(
        `Flat-question response ${path} must use ${FLAT_QUESTION_MEDIA_TYPE}`,
      );
    }
    const text = await boundedText(response, path);
    return { source: parseFlatQuestionSource(text), revision: strongRevision(response, path) };
  }

  async function save(
    workspace: WorkspaceId,
    source: FlatQuestionSourceV2,
    revision?: string,
  ): Promise<FlatQuestionSave> {
    const path = sourcePath(workspace);
    const requestPath = sameOriginPath(basePath, path);
    const headers: Record<string, string> = {
      accept: "application/json",
      "content-type": FLAT_QUESTION_MEDIA_TYPE,
    };
    if (revision !== undefined) headers["if-match"] = validRevision(revision);
    const response = await fetchImplementation(
      requestPath,
      requestInit("PUT", headers, serializeFlatQuestionSource(source)),
    );
    if (response.status === 409 || response.status === 428)
      throw new FlatQuestionConflictError(response.status, path);
    if (!response.ok) throw new FlatQuestionRequestError(response.status, path);
    requireJson(response, path);
    const text = await boundedText(response, path);
    const draft = decodeDraftQuestionDefinition(decodeJson(text, path));
    if (draft.workspace !== workspace) {
      throw new FlatQuestionProtocolError(
        "Flat-question save response does not match its workspace",
      );
    }
    requireFlatNativeSource(draft.source, source.response, "save");
    return { draft, revision: strongRevision(response, path) };
  }

  async function publish(
    workspace: WorkspaceId,
    request: { readonly scope: PublicationScope; readonly byline: PublicByline },
    revision: string,
  ): Promise<CatalogProblemSummary> {
    if (request.scope !== "institution" && request.scope !== "public") {
      throw new FlatQuestionProtocolError("Flat-question publication scope is invalid");
    }
    if (!isPublicByline(request.byline)) {
      throw new FlatQuestionProtocolError(
        "Flat-question publication requires one to sixteen reviewed author names",
      );
    }
    const path = publishPath(workspace);
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
        JSON.stringify(request),
      ),
    );
    if (response.status === 409 || response.status === 428)
      throw new FlatQuestionConflictError(response.status, path);
    if (!response.ok) throw new FlatQuestionRequestError(response.status, path);
    requireJson(response, path);
    const summary = decodeCatalogProblemSummary(
      decodeJson(await boundedText(response, path), path),
      path,
      true,
    );
    if (!isPublishedNativeCatalogProblemSummary(summary, request.scope)) {
      throw new FlatQuestionProtocolError(
        "Flat-question publication response must be a native published summary for its requested scope",
      );
    }
    return summary;
  }

  return { load, save, publish };
}

function validRevision(value: string): string {
  if (!/^"[1-9][0-9]*"$/u.test(value)) {
    throw new FlatQuestionProtocolError(
      "Flat-question revision must be one positive strong numeric ETag",
    );
  }
  if (BigInt(value.slice(1, -1)) > 9_223_372_036_854_775_807n) {
    throw new FlatQuestionProtocolError(
      "Flat-question revision must fit in a signed 64-bit integer",
    );
  }
  return value;
}
