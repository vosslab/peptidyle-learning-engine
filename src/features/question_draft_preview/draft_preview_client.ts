import type { QuestionContentBlock } from "../../../generated/api/QuestionContentBlock";
import type { QuestionResponseFormat } from "../../../generated/api/QuestionResponseFormat";
import type { StudentResponse } from "../../../generated/api/StudentResponse";
import type { PublishedQuestionRevisionTuple } from "../../../generated/api/PublishedQuestionRevisionTuple";
import type { DraftQuestionRouteId } from "../../navigation/public_route";
import {
  ifMatchHeaderForPositiveNumber,
  numberFromQuotedPositiveHeader,
} from "../../api/http_client/conditional_request";
import { ApiProtocolError, ApiRequestError } from "../../api/http_client/error";
import { requestSameOrigin } from "../../api/http_client/request";
import { decodePublishedQuestionRevisionTuple } from "../../api/decoders/shared";

const MAX_SOURCE_BYTES = 4 * 1024 * 1024;
const MAX_RESPONSE_BYTES = 128 * 1024;
const SAFE_ORIGIN = "https://ple-question-draft-preview.invalid";
const NATIVE_MEDIA_TYPE = "application/vnd.peptidyle.question+json";
const WEBWORK_MEDIA_TYPE = "text/x-wework-pg";

export type DraftSourceBinding =
  | {
      readonly backend: "ple";
      readonly format: "pleQuestionJson";
      readonly mediaType: typeof NATIVE_MEDIA_TYPE;
      readonly webworkPgPath: null;
    }
  | {
      readonly backend: "webwork";
      readonly format: "webworkPg" | "webworkPgml";
      readonly mediaType: typeof WEBWORK_MEDIA_TYPE;
      readonly webworkPgPath: string;
    };

export type DraftSourceRead = {
  readonly source: string;
  readonly draftQuestionEditNumber: string;
  readonly binding: DraftSourceBinding;
};

export type NativeDraftPreview = {
  readonly questionTitle: string;
  readonly prompt: ReadonlyArray<QuestionContentBlock>;
  readonly response: QuestionResponseFormat;
};

export type DraftTestResult =
  | { readonly kind: "evaluated"; readonly correct: boolean; readonly normalizedCredit: number }
  | { readonly kind: "ungraded" };

export type DraftPreviewFetch = (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>;

export interface DraftPreviewClientConfig {
  readonly fetch?: DraftPreviewFetch;
  readonly basePath?: string;
}

export interface DraftQuestionPreviewClient {
  loadSource(draftQuestion: DraftQuestionRouteId): Promise<DraftSourceRead>;
  saveSource(
    draftQuestion: DraftQuestionRouteId,
    binding: DraftSourceBinding,
    source: string,
    expectedDraftQuestionEditNumber: string,
  ): Promise<string>;
  loadNativePreview(
    draftQuestion: DraftQuestionRouteId,
    draftQuestionEditNumber: string,
  ): Promise<NativeDraftPreview>;
  webworkPreviewPath(
    draftQuestion: DraftQuestionRouteId,
    draftQuestionEditNumber: string,
    seed: number,
  ): string;
  test(
    draftQuestion: DraftQuestionRouteId,
    draftQuestionEditNumber: string,
    response: StudentResponse,
    seed?: number,
  ): Promise<DraftTestResult>;
  publishRevision(
    draftQuestion: DraftQuestionRouteId,
    parent: PublishedQuestionRevisionTuple,
    reasonForEdit: string,
    expectedDraftQuestionEditNumber: string,
  ): Promise<PublishedQuestionRevisionTuple>;
  webworkResponse(pairs: ReadonlyArray<readonly [string, string]>): StudentResponse;
}

function encodedId(value: string): string {
  return encodeURIComponent(value);
}

function normalizeBasePath(value: string | undefined): string {
  if (value === undefined || value === "" || value === "/") return "";
  if (
    !value.startsWith("/") ||
    value.startsWith("//") ||
    value.includes("\\") ||
    value.includes("?") ||
    value.includes("#")
  )
    throw new Error("Draft preview basePath must be a same-origin path without query or fragment");
  const normalized = value.replace(/\/+$/u, "");
  const resolved = new URL(normalized, SAFE_ORIGIN);
  if (
    resolved.origin !== SAFE_ORIGIN ||
    resolved.pathname !== normalized ||
    resolved.search !== "" ||
    resolved.hash !== ""
  )
    throw new Error("Draft preview basePath must be a stable same-origin path");
  return normalized;
}

function sourcePath(draftQuestion: DraftQuestionRouteId): string {
  return `/api/authoring/drafts/${encodedId(draftQuestion)}/source`;
}

function previewPath(draftQuestion: DraftQuestionRouteId): string {
  return `/api/authoring/drafts/${encodedId(draftQuestion)}/preview`;
}

function testPath(draftQuestion: DraftQuestionRouteId): string {
  return `/api/authoring/drafts/${encodedId(draftQuestion)}/test`;
}

function publishRevisionPath(draftQuestion: DraftQuestionRouteId): string {
  return `/api/authoring/drafts/${encodedId(draftQuestion)}/publish-revision`;
}

function responseEditNumber(response: Response, path: string): string {
  try {
    return numberFromQuotedPositiveHeader(response, path, "Draft Question Edit Number");
  } catch (error: unknown) {
    throw new ApiProtocolError(
      error instanceof Error ? error.message : `Draft response ${path} omitted its Edit Number`,
    );
  }
}

function sourceBinding(response: Response, path: string): DraftSourceBinding {
  const backend = response.headers.get("x-ple-question-backend");
  const format = response.headers.get("x-ple-question-format");
  const pathHeader = response.headers.get("x-ple-webwork-pg-path");
  const mediaType = response.headers.get("content-type")?.split(";", 1)[0]?.trim().toLowerCase();
  if (
    backend === "ple" &&
    format === "pleQuestionJson" &&
    mediaType === NATIVE_MEDIA_TYPE &&
    pathHeader === ""
  ) {
    return { backend, format, mediaType, webworkPgPath: null };
  }
  if (
    backend === "webwork" &&
    (format === "webworkPg" || format === "webworkPgml") &&
    mediaType === WEBWORK_MEDIA_TYPE &&
    pathHeader !== null &&
    pathHeader.length > 0
  ) {
    let webworkPgPath: string;
    try {
      webworkPgPath = decodeURIComponent(pathHeader);
    } catch {
      throw new ApiProtocolError(`Draft source binding ${path} has an invalid WebWork path`);
    }
    if (
      webworkPgPath.startsWith("/") ||
      webworkPgPath.includes("\\") ||
      webworkPgPath.includes("\0") ||
      webworkPgPath.split("/").some((part) => part === "" || part === "." || part === "..")
    )
      throw new ApiProtocolError(`Draft source binding ${path} has an invalid WebWork path`);
    return { backend, format, mediaType, webworkPgPath };
  }
  throw new ApiProtocolError(`Draft source binding ${path} is unsupported or incoherent`);
}

async function boundedText(response: Response, path: string, allowEmpty: boolean): Promise<string> {
  const bytes = new Uint8Array(await response.arrayBuffer());
  if (bytes.length > MAX_SOURCE_BYTES || (!allowEmpty && bytes.length === 0))
    throw new ApiProtocolError(`Draft source ${path} is empty or exceeds its supported size`);
  try {
    return new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  } catch {
    throw new ApiProtocolError(`Draft source ${path} is not valid UTF-8 text`);
  }
}

function decodeNativePreview(value: unknown): NativeDraftPreview {
  if (typeof value !== "object" || value === null || Array.isArray(value))
    throw new ApiProtocolError("Draft preview returned an invalid record");
  const record = value as Record<string, unknown>;
  if (
    Object.keys(record).length !== 3 ||
    typeof record.questionTitle !== "string" ||
    !Array.isArray(record.prompt) ||
    typeof record.response !== "object" ||
    record.response === null
  )
    throw new ApiProtocolError("Draft preview returned an invalid Native presentation");
  return record as NativeDraftPreview;
}

function decodeTestResult(value: unknown): DraftTestResult {
  if (typeof value !== "object" || value === null || Array.isArray(value))
    throw new ApiProtocolError("Draft test returned an invalid result");
  const record = value as Record<string, unknown>;
  if (record.kind === "ungraded" && Object.keys(record).length === 1) return { kind: "ungraded" };
  if (
    record.kind === "evaluated" &&
    Object.keys(record).length === 3 &&
    typeof record.correct === "boolean" &&
    typeof record.normalizedCredit === "number" &&
    Number.isFinite(record.normalizedCredit) &&
    record.normalizedCredit >= 0 &&
    record.normalizedCredit <= 1
  )
    return {
      kind: "evaluated",
      correct: record.correct,
      normalizedCredit: record.normalizedCredit,
    };
  throw new ApiProtocolError("Draft test returned an invalid grading result");
}

function encodeBase64Utf8(value: string): string {
  const bytes = new TextEncoder().encode(value);
  let binary = "";
  for (let offset = 0; offset < bytes.length; offset += 0x8000) {
    binary += String.fromCharCode(...bytes.subarray(offset, offset + 0x8000));
  }
  return btoa(binary);
}

export function createDraftQuestionPreviewClient(
  config: DraftPreviewClientConfig = {},
): DraftQuestionPreviewClient {
  const fetchImplementation = config.fetch ?? globalThis.fetch;
  const basePath = normalizeBasePath(config.basePath);

  async function loadSource(draftQuestion: DraftQuestionRouteId): Promise<DraftSourceRead> {
    const path = sourcePath(draftQuestion);
    const requestPath = `${basePath}${path}`;
    const response = await fetchImplementation(requestPath, {
      method: "GET",
      credentials: "same-origin",
      cache: "no-store",
      headers: { accept: "*/*" },
    });
    if (!response.ok) throw new ApiRequestError(response.status, requestPath);
    const binding = sourceBinding(response, path);
    const source = await boundedText(response, path, true);
    const draftQuestionEditNumber = responseEditNumber(response, path);
    return { source, draftQuestionEditNumber, binding };
  }

  async function saveSource(
    draftQuestion: DraftQuestionRouteId,
    binding: DraftSourceBinding,
    source: string,
    expectedDraftQuestionEditNumber: string,
  ): Promise<string> {
    const path = sourcePath(draftQuestion);
    const requestPath = `${basePath}${path}`;
    if (new TextEncoder().encode(source).length > MAX_SOURCE_BYTES)
      throw new ApiProtocolError(`Draft source ${path} exceeds its supported size`);
    const response = await fetchImplementation(requestPath, {
      method: "PUT",
      credentials: "same-origin",
      cache: "no-store",
      headers: {
        accept: "application/json",
        "content-type": binding.mediaType,
        "if-match": ifMatchHeaderForPositiveNumber(
          expectedDraftQuestionEditNumber,
          path,
          "Draft Question Edit Number",
        ),
      },
      body: source,
    });
    if (response.status === 409 || response.status === 412 || response.status === 428)
      throw new ApiRequestError(response.status, requestPath);
    if (!response.ok) throw new ApiRequestError(response.status, requestPath);
    if (response.status !== 204)
      throw new ApiProtocolError(`Draft source save ${path} must return no content`);
    return responseEditNumber(response, path);
  }

  async function loadNativePreview(
    draftQuestion: DraftQuestionRouteId,
    draftQuestionEditNumber: string,
  ): Promise<NativeDraftPreview> {
    const path = previewPath(draftQuestion);
    const query = new URLSearchParams({ draftQuestionEditNumber });
    const requestPath = `${path}?${query.toString()}`;
    const response = await requestSameOrigin(fetchImplementation, basePath, requestPath, {
      method: "GET",
      headers: { accept: "application/json" },
    });
    if (response.status === 409 || response.status === 412 || response.status === 428)
      throw new ApiRequestError(response.status, path);
    if (!response.ok) throw new ApiRequestError(response.status, path);
    if (
      response.headers.get("content-type")?.split(";", 1)[0]?.trim().toLowerCase() !==
      "application/json"
    )
      throw new ApiProtocolError(`Draft Native preview ${path} must return JSON`);
    const bytes = new Uint8Array(await response.arrayBuffer());
    if (bytes.length > MAX_RESPONSE_BYTES)
      throw new ApiProtocolError(`Draft preview ${path} is too large`);
    return decodeNativePreview(JSON.parse(new TextDecoder().decode(bytes)) as unknown);
  }

  function webworkPreviewPath(
    draftQuestion: DraftQuestionRouteId,
    draftQuestionEditNumber: string,
    seed: number,
  ): string {
    if (!Number.isSafeInteger(seed) || seed < 0 || seed > 4_294_967_295)
      throw new ApiProtocolError("Draft preview seed is outside its supported range");
    const path = previewPath(draftQuestion);
    const query = new URLSearchParams({
      draftQuestionEditNumber,
      seed: String(seed),
      draftTest: "true",
    });
    return `${basePath}${path}?${query.toString()}`;
  }

  async function test(
    draftQuestion: DraftQuestionRouteId,
    draftQuestionEditNumber: string,
    responseValue: StudentResponse,
    seed?: number,
  ): Promise<DraftTestResult> {
    if (seed !== undefined && (!Number.isSafeInteger(seed) || seed < 0 || seed > 4_294_967_295))
      throw new ApiProtocolError("Draft test seed is outside its supported range");
    const path = testPath(draftQuestion);
    const requestPath = `${basePath}${path}`;
    const response = await requestSameOrigin(fetchImplementation, basePath, path, {
      method: "POST",
      headers: {
        accept: "application/json",
        "content-type": "application/json",
        "if-match": ifMatchHeaderForPositiveNumber(
          draftQuestionEditNumber,
          path,
          "Draft Question Edit Number",
        ),
      },
      body: { ...(seed === undefined ? {} : { seed }), response: responseValue },
    });
    if (response.status === 409 || response.status === 412 || response.status === 428)
      throw new ApiRequestError(response.status, requestPath);
    if (!response.ok) throw new ApiRequestError(response.status, requestPath);
    const bytes = new Uint8Array(await response.arrayBuffer());
    if (bytes.length > MAX_RESPONSE_BYTES)
      throw new ApiProtocolError(`Draft test ${path} is too large`);
    return decodeTestResult(JSON.parse(new TextDecoder().decode(bytes)) as unknown);
  }

  async function publishRevision(
    draftQuestion: DraftQuestionRouteId,
    parent: PublishedQuestionRevisionTuple,
    reasonForEdit: string,
    expectedDraftQuestionEditNumber: string,
  ): Promise<PublishedQuestionRevisionTuple> {
    const path = publishRevisionPath(draftQuestion);
    const response = await requestSameOrigin(fetchImplementation, basePath, path, {
      method: "POST",
      headers: {
        accept: "application/json",
        "content-type": "application/json",
        "if-match": ifMatchHeaderForPositiveNumber(
          expectedDraftQuestionEditNumber,
          path,
          "Draft Question Edit Number",
        ),
      },
      body: {
        questionId: parent.publishedQuestionId,
        parentRevisionNumber: parent.revisionNumber,
        reasonForEdit,
      },
    });
    if (response.status === 409 || response.status === 412 || response.status === 428)
      throw new ApiRequestError(response.status, path);
    if (!response.ok) throw new ApiRequestError(response.status, path);
    if (
      response.headers.get("content-type")?.split(";", 1)[0]?.trim().toLowerCase() !==
      "application/json"
    )
      throw new ApiProtocolError(`Question Revision response ${path} must return JSON`);
    const bytes = new Uint8Array(await response.arrayBuffer());
    if (bytes.length > MAX_RESPONSE_BYTES)
      throw new ApiProtocolError(`Question Revision response ${path} is too large`);
    const value: unknown = JSON.parse(new TextDecoder().decode(bytes));
    if (typeof value !== "object" || value === null || Array.isArray(value))
      throw new ApiProtocolError(`Question Revision response ${path} is invalid`);
    const record = value as Record<string, unknown>;
    if (
      Object.keys(record).length !== 1 ||
      !Object.prototype.hasOwnProperty.call(record, "publishedQuestionRevisionTuple")
    )
      throw new ApiProtocolError(`Question Revision response ${path} has unexpected fields`);
    return decodePublishedQuestionRevisionTuple(
      record.publishedQuestionRevisionTuple,
      `${path}.publishedQuestionRevisionTuple`,
    );
  }

  function webworkResponse(pairs: ReadonlyArray<readonly [string, string]>): StudentResponse {
    if (
      pairs.length > 1024 ||
      pairs.some(([name, value]) => name.length > 1024 || value.length > 65536)
    )
      throw new ApiProtocolError("Draft WebWork response exceeds its supported size");
    return { kind: "backendOwned", payload: encodeBase64Utf8(JSON.stringify(pairs)) };
  }

  return {
    loadSource,
    saveSource,
    loadNativePreview,
    webworkPreviewPath,
    test,
    publishRevision,
    webworkResponse,
  };
}

export function draftPreviewSeed(): number {
  return crypto.getRandomValues(new Uint32Array(1))[0] ?? 0;
}
