// qti_profile_import_client.ts - same-origin transport for private QTI profile imports.

import type { WorkspaceId } from "../../../generated/api/WorkspaceId";
import type { WorkspaceImportId } from "../../../generated/api/WorkspaceImportId";
import { decodeDraftQuestionDefinition } from "../../api/decoders";
import type {
  QtiProfileAcknowledgement,
  QtiProfileConversionResult,
  QtiProfileImportResponse,
} from "./qti_profile_import_contract";
import { decodeQtiProfileImportResponse } from "./qti_profile_import_decoder";

const SAFE_ORIGIN = "https://qti-profile-import.invalid";
const MAX_ARCHIVE_BYTES = 32 * 1_024 * 1_024;
const MAX_REPORT_BYTES = 16 * 1_024 * 1_024;

export type QtiProfileImportFetch = (
  input: RequestInfo | URL,
  init?: RequestInit,
) => Promise<Response>;

export interface QtiProfileImportClientConfig {
  readonly fetch?: QtiProfileImportFetch;
  /** Same-origin path prefix only; origins, queries, and fragments are rejected. */
  readonly basePath?: string;
}

export interface QtiProfileImportClient {
  upload(
    workspace: WorkspaceId,
    importId: WorkspaceImportId,
    archive: Blob,
  ): Promise<QtiProfileImportResponse>;
  report(workspace: WorkspaceId, importId: WorkspaceImportId): Promise<QtiProfileImportResponse>;
  convert(
    workspace: WorkspaceId,
    importId: WorkspaceImportId,
    sourceIdentifier: string,
    acknowledgement: QtiProfileAcknowledgement,
    draftRevision: string,
  ): Promise<QtiProfileConversionResult>;
}

export class QtiProfileImportRequestError extends Error {
  public readonly status: number;
  public readonly path: string;

  public constructor(status: number, path: string) {
    super(`QTI import request ${path} failed with status ${status}`);
    this.name = "QtiProfileImportRequestError";
    this.status = status;
    this.path = path;
  }
}

export class QtiProfileImportConflictError extends QtiProfileImportRequestError {
  declare public readonly status: 409 | 428;

  public constructor(status: 409 | 428, path: string) {
    super(status, path);
    this.name = "QtiProfileImportConflictError";
  }
}

export class QtiProfileImportProtocolError extends Error {
  public constructor(message: string) {
    super(message);
    this.name = "QtiProfileImportProtocolError";
  }
}

export class QtiProfileImportArchiveError extends Error {
  public constructor(message: string) {
    super(message);
    this.name = "QtiProfileImportArchiveError";
  }
}

function browserFetch(input: RequestInfo | URL, init?: RequestInit): Promise<Response> {
  return globalThis.fetch(input, init);
}

function hasAsciiControl(value: string): boolean {
  for (const character of value) {
    const point = character.codePointAt(0);
    if (point !== undefined && (point <= 0x1f || point === 0x7f)) return true;
  }
  return false;
}

function normalizeBasePath(value: string | undefined): string {
  if (value === undefined || value === "" || value === "/") return "";
  if (
    !value.startsWith("/") ||
    value.startsWith("//") ||
    value.includes("\\") ||
    value.includes("?") ||
    value.includes("#") ||
    hasAsciiControl(value)
  ) {
    throw new QtiProfileImportProtocolError("QTI import basePath must be a same-origin path");
  }
  const normalized = value.replace(/\/+$/u, "");
  const resolved = new URL(normalized, SAFE_ORIGIN);
  if (
    resolved.origin !== SAFE_ORIGIN ||
    resolved.pathname !== normalized ||
    resolved.search !== "" ||
    resolved.hash !== ""
  ) {
    throw new QtiProfileImportProtocolError("QTI import basePath escaped the same origin");
  }
  return normalized;
}

function requestPath(basePath: string, path: string): string {
  const candidate = `${basePath}${path}`;
  const resolved = new URL(candidate, SAFE_ORIGIN);
  if (resolved.origin !== SAFE_ORIGIN || resolved.pathname !== candidate) {
    throw new QtiProfileImportProtocolError("QTI import request path escaped the same origin");
  }
  return candidate;
}

function importPath(workspace: WorkspaceId, importId: WorkspaceImportId): string {
  return `/api/workspaces/${encodeURIComponent(workspace)}/qti-imports/${encodeURIComponent(importId)}`;
}

function conversionPath(
  workspace: WorkspaceId,
  importId: WorkspaceImportId,
  sourceIdentifier: string,
): string {
  return `${importPath(workspace, importId)}/items/${encodeURIComponent(sourceIdentifier)}/convert-flat`;
}

function requireJson(response: Response, path: string): void {
  const mediaType = response.headers.get("content-type")?.split(";", 1)[0]?.trim().toLowerCase();
  if (mediaType !== "application/json") {
    throw new QtiProfileImportProtocolError(
      `QTI import response ${path} must use application/json`,
    );
  }
}

function requireNoStore(response: Response, path: string): void {
  const directives =
    response.headers
      .get("cache-control")
      ?.split(",")
      .map((value) => value.trim().toLowerCase()) ?? [];
  if (!directives.includes("no-store")) {
    throw new QtiProfileImportProtocolError(`QTI import response ${path} must be no-store`);
  }
}

async function boundedJson(response: Response, path: string): Promise<unknown> {
  requireJson(response, path);
  requireNoStore(response, path);
  const text = await response.text();
  if (text.length === 0 || new TextEncoder().encode(text).length > MAX_REPORT_BYTES) {
    throw new QtiProfileImportProtocolError(`QTI import response ${path} must be bounded JSON`);
  }
  try {
    return JSON.parse(text);
  } catch {
    throw new QtiProfileImportProtocolError(`QTI import response ${path} is not valid JSON`);
  }
}

function validRevision(revision: string): string {
  if (!/^"[1-9][0-9]*"$/u.test(revision)) {
    throw new QtiProfileImportProtocolError("QTI conversion needs one strong numeric ETag");
  }
  if (BigInt(revision.slice(1, -1)) > 9_223_372_036_854_775_807n) {
    throw new QtiProfileImportProtocolError("QTI conversion ETag is out of range");
  }
  return revision;
}

function responseRevision(response: Response, path: string): string {
  const revision = response.headers.get("etag");
  if (revision === null) {
    throw new QtiProfileImportProtocolError(`QTI conversion response ${path} is missing its ETag`);
  }
  return validRevision(revision);
}

function validAcknowledgement(value: string, name: string): string {
  if (!/^[0-9a-f]{64}$/u.test(value)) {
    throw new QtiProfileImportProtocolError(`QTI conversion ${name} is invalid`);
  }
  return value;
}

function conflict(response: Response, path: string): never {
  requireNoStore(response, path);
  if (response.status === 409 || response.status === 428) {
    throw new QtiProfileImportConflictError(response.status, path);
  }
  throw new QtiProfileImportRequestError(response.status, path);
}

async function importResponse(
  response: Response,
  path: string,
  importId: WorkspaceImportId,
): Promise<QtiProfileImportResponse> {
  if (![200, 202, 422].includes(response.status)) conflict(response, path);
  const decoded = decodeQtiProfileImportResponse(await boundedJson(response, path));
  if (decoded.importId !== importId) {
    throw new QtiProfileImportProtocolError("QTI import response identity does not match its path");
  }
  const statusMatches =
    (response.status === 200 && decoded.state === "ready") ||
    (response.status === 202 && (decoded.state === "queued" || decoded.state === "processing")) ||
    (response.status === 422 &&
      (decoded.state === "failed" || decoded.state === "unsupportedProfile"));
  if (!statusMatches) {
    throw new QtiProfileImportProtocolError("QTI import response state does not match its status");
  }
  return decoded;
}

/** Builds the private QTI transport without adding QTI types to the global browser client. */
export function createQtiProfileImportClient(
  config: QtiProfileImportClientConfig = {},
): QtiProfileImportClient {
  const fetchImplementation = config.fetch ?? browserFetch;
  const basePath = normalizeBasePath(config.basePath);

  async function upload(
    workspace: WorkspaceId,
    importId: WorkspaceImportId,
    archive: Blob,
  ): Promise<QtiProfileImportResponse> {
    if (archive.size === 0) throw new QtiProfileImportArchiveError("QTI archive is empty");
    if (archive.size > MAX_ARCHIVE_BYTES) {
      throw new QtiProfileImportArchiveError("QTI archive exceeds the 32 MiB limit");
    }
    const path = importPath(workspace, importId);
    const response = await fetchImplementation(requestPath(basePath, path), {
      method: "PUT",
      headers: { accept: "application/json", "content-type": "application/zip" },
      body: archive,
      credentials: "same-origin",
      cache: "no-store",
    });
    return await importResponse(response, path, importId);
  }

  async function report(
    workspace: WorkspaceId,
    importId: WorkspaceImportId,
  ): Promise<QtiProfileImportResponse> {
    const path = importPath(workspace, importId);
    const response = await fetchImplementation(requestPath(basePath, path), {
      method: "GET",
      headers: { accept: "application/json" },
      credentials: "same-origin",
      cache: "no-store",
    });
    return await importResponse(response, path, importId);
  }

  async function convert(
    workspace: WorkspaceId,
    importId: WorkspaceImportId,
    sourceIdentifier: string,
    acknowledgement: QtiProfileAcknowledgement,
    draftRevision: string,
  ): Promise<QtiProfileConversionResult> {
    if (sourceIdentifier.length === 0) {
      throw new QtiProfileImportProtocolError("QTI conversion item identity is empty");
    }
    const path = conversionPath(workspace, importId, sourceIdentifier);
    const response = await fetchImplementation(requestPath(basePath, path), {
      method: "POST",
      headers: {
        accept: "application/json",
        "content-type": "application/json",
        "if-match": validRevision(draftRevision),
      },
      body: JSON.stringify({
        reportRevision: validAcknowledgement(acknowledgement.reportRevision, "report revision"),
        reviewToken: validAcknowledgement(acknowledgement.reviewToken, "review token"),
      }),
      credentials: "same-origin",
      cache: "no-store",
    });
    if (!response.ok) conflict(response, path);
    const value = await boundedJson(response, path);
    const draft = decodeDraftQuestionDefinition(value, "response");
    if (draft.workspace !== workspace) {
      throw new QtiProfileImportProtocolError(
        "QTI conversion response workspace does not match its path",
      );
    }
    return { draft, revision: responseRevision(response, path) };
  }

  return { upload, report, convert };
}
