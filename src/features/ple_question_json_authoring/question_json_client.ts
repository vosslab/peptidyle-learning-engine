import type { QuestionSummary } from "../../../generated/api/QuestionSummary";
import type { PleQuestionJsonPublicationRequest } from "./question_json_repository";
import { decodeUuid } from "../../api/decoder";
import type { DraftQuestionRouteId } from "../../navigation/public_route";
import { decodeQuestionLineageView, isAvailablePleQuestionSummary } from "../../api/decoders";
import { isQuestionAuthorship } from "../../api/question_authorship";
import { validateCanonicalQuestionIdSyntax } from "../../question_id";
import { PLE_QUESTION_JSON_MEDIA_TYPE, type PleQuestionJsonDocument } from "./question_json_source";
import { parsePleQuestionJsonSource, serializePleQuestionJsonSource } from "./question_json_codec";
import {
  ifMatchHeaderForPositiveNumber,
  numberFromResponseEtag,
} from "../../api/http_client/conditional_request";
import { ApiProtocolError } from "../../api/http_client/error";

const MAX_RESPONSE_BYTES = 4 * 1024 * 1024;
const SAFE_ORIGIN = "https://ple-question-json.invalid";
function publicationUuid(value: unknown, path: string): string {
  const uuid = decodeUuid(value, path);
  if (uuid !== uuid.toLowerCase())
    throw new PleQuestionJsonProtocolError("Publication requires canonical lowercase UUIDs");
  return uuid;
}

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
  declare public readonly status: 409 | 412 | 428;

  public constructor(status: 409 | 412 | 428, path: string) {
    super(status, path);
  }
}

export class PleQuestionJsonProtocolError extends Error {}

export type PleQuestionJsonRead = {
  readonly source: PleQuestionJsonDocument;
  readonly draftQuestionEditNumber: string;
};

export type PleQuestionJsonSave = {
  readonly draftQuestionEditNumber: string;
};

export interface PleQuestionJsonClient {
  uploadAsset(
    draftQuestion: DraftQuestionRouteId,
    image: Blob,
    expectedDraftQuestionEditNumber: string,
    signal?: AbortSignal,
  ): Promise<PleQuestionJsonAssetDescriptor>;
  assetPreviewPath(draftQuestion: DraftQuestionRouteId, asset: string): string;
  load(draftQuestion: DraftQuestionRouteId): Promise<PleQuestionJsonRead>;
  save(
    draftQuestion: DraftQuestionRouteId,
    source: PleQuestionJsonDocument,
    expectedDraftQuestionEditNumber?: string,
  ): Promise<PleQuestionJsonSave>;
  publish(
    draftQuestion: DraftQuestionRouteId,
    request: PleQuestionJsonPublicationRequest,
    expectedDraftQuestionEditNumber: string,
  ): Promise<QuestionSummary>;
}

export type PleQuestionJsonAssetDescriptor = {
  readonly questionAsset: string;
  readonly checksum: string;
  readonly mediaType: "image/png" | "image/jpeg" | "image/webp";
  readonly intrinsicWidth: number;
  readonly intrinsicHeight: number;
};

/** ASVS 2.2.1: closed measured server facts, never caller-provided source or URLs. */
export function decodePleQuestionJsonAssetDescriptor(
  value: unknown,
): PleQuestionJsonAssetDescriptor {
  if (typeof value !== "object" || value === null || Array.isArray(value))
    throw new PleQuestionJsonProtocolError("The image upload returned an invalid descriptor.");
  const record = value as Record<string, unknown>;
  const keys = ["questionAsset", "checksum", "mediaType", "intrinsicWidth", "intrinsicHeight"];
  if (
    Object.keys(record).length !== keys.length ||
    keys.some((key) => !Object.prototype.hasOwnProperty.call(record, key))
  )
    throw new PleQuestionJsonProtocolError(
      "The image upload returned unexpected descriptor fields.",
    );
  const questionAsset = publicationUuid(record.questionAsset, "asset.questionAsset");
  const checksum = record.checksum;
  const mediaType = record.mediaType;
  const intrinsicWidth = record.intrinsicWidth;
  const intrinsicHeight = record.intrinsicHeight;
  if (
    typeof checksum !== "string" ||
    !/^[a-f0-9]{64}$/u.test(checksum) ||
    (mediaType !== "image/png" && mediaType !== "image/jpeg" && mediaType !== "image/webp") ||
    typeof intrinsicWidth !== "number" ||
    !Number.isSafeInteger(intrinsicWidth) ||
    intrinsicWidth < 1 ||
    typeof intrinsicHeight !== "number" ||
    !Number.isSafeInteger(intrinsicHeight) ||
    intrinsicHeight < 1 ||
    intrinsicWidth * intrinsicHeight > 20_000_000
  )
    throw new PleQuestionJsonProtocolError(
      "The image upload returned invalid measured image facts.",
    );
  return { questionAsset, checksum, mediaType, intrinsicWidth, intrinsicHeight };
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

function sourcePath(draftQuestion: DraftQuestionRouteId): string {
  return `/api/authoring/drafts/${encodedId(draftQuestion)}/source`;
}

function publishPath(draftQuestion: DraftQuestionRouteId): string {
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

function draftQuestionEditNumberFromResponse(response: Response, path: string): string {
  try {
    return numberFromResponseEtag(response, path, "Draft Question Edit Number");
  } catch (error: unknown) {
    throw new PleQuestionJsonProtocolError(
      error instanceof ApiProtocolError
        ? error.message
        : `PLE Question JSON response ${path} must include one Draft Question Edit Number`,
    );
  }
}

function ifMatchDraftQuestionEditNumber(value: string, path: string): string {
  try {
    return ifMatchHeaderForPositiveNumber(value, path, "Draft Question Edit Number");
  } catch (error: unknown) {
    throw new PleQuestionJsonProtocolError(
      error instanceof ApiProtocolError
        ? error.message
        : "Draft Question Edit Number must be a positive integer",
    );
  }
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

  function assetPreviewPath(draftQuestion: DraftQuestionRouteId, asset: string): string {
    const canonicalAsset = publicationUuid(asset, "asset.questionAsset");
    return sameOriginPath(
      basePath,
      `/api/authoring/drafts/${encodedId(draftQuestion)}/assets/${encodedId(canonicalAsset)}`,
    );
  }

  async function uploadAsset(
    draftQuestion: DraftQuestionRouteId,
    image: Blob,
    expectedDraftQuestionEditNumber: string,
    signal?: AbortSignal,
  ): Promise<PleQuestionJsonAssetDescriptor> {
    if (image.size < 1 || image.size > 8 * 1024 * 1024)
      throw new PleQuestionJsonProtocolError("Choose an image no larger than 8 MiB.");
    if (!["image/png", "image/jpeg", "image/webp"].includes(image.type))
      throw new PleQuestionJsonProtocolError(
        "Choose a PNG, JPEG, or WebP image. SVG is not yet supported.",
      );
    const path = `/api/authoring/drafts/${encodedId(draftQuestion)}/assets`;
    // ASVS 2.2.2: browser checks aid usability; the server verifies the actual raster bytes.
    const response = await fetchImplementation(sameOriginPath(basePath, path), {
      method: "POST",
      credentials: "same-origin",
      cache: "no-store",
      signal,
      body: image,
      headers: {
        accept: "application/json",
        "content-type": image.type,
        "if-match": ifMatchDraftQuestionEditNumber(expectedDraftQuestionEditNumber, path),
      },
    });
    if (response.status === 409 || response.status === 412 || response.status === 428)
      throw new PleQuestionJsonConflictError(response.status, path);
    if (!response.ok) throw new PleQuestionJsonRequestError(response.status, path);
    if (response.status !== 201)
      throw new PleQuestionJsonProtocolError("The image upload must return a created asset.");
    requireJson(response, path);
    return decodePleQuestionJsonAssetDescriptor(
      decodeJson(await boundedText(response, path), path),
    );
  }

  async function load(draftQuestion: DraftQuestionRouteId): Promise<PleQuestionJsonRead> {
    const path = sourcePath(draftQuestion);
    const requestPath = sameOriginPath(basePath, path);
    const response = await fetchImplementation(
      requestPath,
      requestInit("GET", {
        accept: PLE_QUESTION_JSON_MEDIA_TYPE,
      }),
    );
    if (response.status === 409 || response.status === 412 || response.status === 428)
      throw new PleQuestionJsonConflictError(response.status, path);
    if (!response.ok) throw new PleQuestionJsonRequestError(response.status, path);
    if (!isFlatMediaType(response)) {
      throw new PleQuestionJsonProtocolError(
        `PLE Question JSON response ${path} must use ${PLE_QUESTION_JSON_MEDIA_TYPE}`,
      );
    }
    const text = await boundedText(response, path);
    return {
      source: parsePleQuestionJsonSource(text),
      draftQuestionEditNumber: draftQuestionEditNumberFromResponse(response, path),
    };
  }

  async function save(
    draftQuestion: DraftQuestionRouteId,
    source: PleQuestionJsonDocument,
    expectedDraftQuestionEditNumber?: string,
  ): Promise<PleQuestionJsonSave> {
    const path = sourcePath(draftQuestion);
    const requestPath = sameOriginPath(basePath, path);
    const headers: Record<string, string> = {
      accept: "application/json",
      "content-type": PLE_QUESTION_JSON_MEDIA_TYPE,
    };
    if (expectedDraftQuestionEditNumber !== undefined)
      headers["if-match"] = ifMatchDraftQuestionEditNumber(expectedDraftQuestionEditNumber, path);
    const response = await fetchImplementation(
      requestPath,
      requestInit("PUT", headers, serializePleQuestionJsonSource(source)),
    );
    if (response.status === 409 || response.status === 412 || response.status === 428)
      throw new PleQuestionJsonConflictError(response.status, path);
    if (!response.ok) throw new PleQuestionJsonRequestError(response.status, path);
    if (response.status !== 204) {
      throw new PleQuestionJsonProtocolError(
        `PLE Question JSON save ${path} must return no content`,
      );
    }
    return { draftQuestionEditNumber: draftQuestionEditNumberFromResponse(response, path) };
  }

  async function publish(
    draftQuestion: DraftQuestionRouteId,
    request: PleQuestionJsonPublicationRequest,
    expectedDraftQuestionEditNumber: string,
  ): Promise<QuestionSummary> {
    if (!isQuestionAuthorship(request.authorship)) {
      throw new PleQuestionJsonProtocolError(
        "PLE Question JSON publication requires one to sixteen reviewed Question Authors",
      );
    }
    const path = publishPath(draftQuestion);
    // ASVS 2.2.1-2: validate identities; the server validates the resulting hierarchy.
    const disciplineUuid = publicationUuid(request.disciplineUuid, "publication.disciplineUuid");
    const subjectUuid = publicationUuid(request.subjectUuid, "publication.subjectUuid");
    const topicUuid =
      request.topicUuid === null
        ? null
        : publicationUuid(request.topicUuid, "publication.topicUuid");
    const subtopicUuid =
      request.subtopicUuid === null
        ? null
        : publicationUuid(request.subtopicUuid, "publication.subtopicUuid");
    const requestPath = sameOriginPath(basePath, path);
    const response = await fetchImplementation(
      requestPath,
      requestInit(
        "POST",
        {
          accept: "application/json",
          "content-type": "application/json",
          "if-match": ifMatchDraftQuestionEditNumber(expectedDraftQuestionEditNumber, path),
        },
        JSON.stringify({
          authors: request.authorship.authors.map((author) => author.displayName),
          disciplineUuid,
          subjectUuid,
          topicUuid,
          subtopicUuid,
        }),
      ),
    );
    if (response.status === 409 || response.status === 412 || response.status === 428)
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
    const { summary } = decodeQuestionLineageView(
      decodeJson(await boundedText(summaryResponse, summaryPath), summaryPath),
      summaryPath,
    );
    if (!isAvailablePleQuestionSummary(summary)) {
      throw new PleQuestionJsonProtocolError(
        "PLE Question JSON publication response must be an available PLE Question Library summary",
      );
    }
    return summary;
  }

  return { load, save, publish, uploadAsset, assetPreviewPath };
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
  const canonicalQuestionId = validateCanonicalQuestionIdSyntax(questionId);
  if (canonicalQuestionId === null || canonicalQuestionId !== questionId) {
    throw new PleQuestionJsonProtocolError(
      `PLE Question JSON publication ${path} must return a canonical Question ID`,
    );
  }
  return canonicalQuestionId;
}
