// flat_question_asset_client.ts - protected browser boundary for immutable author image assets.

import type { WorkspaceId } from "../../../generated/api/WorkspaceId";
import {
  DecodeError,
  decodeArray,
  decodeField,
  decodePositiveInteger,
  decodeRecord,
  decodeString,
} from "../../api/decoder";

const MAX_RESPONSE_CHARACTERS = 4 * 1024 * 1024;
const SAFE_ORIGIN = "https://flat-question.invalid";
const ALLOWED_MEDIA_TYPES = ["image/jpeg", "image/png", "image/webp"] as const;

export type FlatQuestionAssetMediaType = (typeof ALLOWED_MEDIA_TYPES)[number];

/** Browser-safe immutable descriptor; object storage facts never cross this boundary. */
export interface FlatQuestionAssetDescriptor {
  readonly assetId: string;
  readonly contentChecksum: string;
  readonly displayLabel: string;
  readonly mediaType: FlatQuestionAssetMediaType;
  readonly intrinsicWidth: number;
  readonly intrinsicHeight: number;
}

export interface FlatQuestionAssetUpload {
  /** Opaque original image bytes; the server sniffs and measures them. */
  readonly image: Blob;
  /** Human-facing image name, not a storage path or key. */
  readonly displayLabel: string;
  /** Author-provided instructional origin retained server-side. */
  readonly provenance: string;
}

/** Fetch-compatible dependency for browser code and deterministic Node tests. */
export type FlatQuestionAssetFetch = (
  input: RequestInfo | URL,
  init?: RequestInit,
) => Promise<Response>;

export interface FlatQuestionAssetClientConfig {
  readonly fetch?: FlatQuestionAssetFetch;
  /** Same-origin path prefix only; never an external origin. */
  readonly basePath?: string;
}

export interface FlatQuestionAssetClient {
  list(workspace: WorkspaceId): Promise<ReadonlyArray<FlatQuestionAssetDescriptor>>;
  upload(
    workspace: WorkspaceId,
    upload: FlatQuestionAssetUpload,
  ): Promise<FlatQuestionAssetDescriptor>;
}

export class FlatQuestionAssetRequestError extends Error {
  public readonly status: number;
  public readonly path: string;

  public constructor(status: number, path: string) {
    super(`Flat-question asset request ${path} failed with status ${status}`);
    this.name = "FlatQuestionAssetRequestError";
    this.status = status;
    this.path = path;
  }
}

export class FlatQuestionAssetProtocolError extends Error {
  public constructor(message: string) {
    super(message);
    this.name = "FlatQuestionAssetProtocolError";
  }
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
    value.includes("?") ||
    value.includes("#") ||
    hasAsciiControl(value)
  ) {
    throw new FlatQuestionAssetProtocolError(
      "Flat-question asset basePath must be a stable same-origin path",
    );
  }
  const normalized = value.replace(/\/+$/u, "");
  const resolved = new URL(normalized, SAFE_ORIGIN);
  if (resolved.origin !== SAFE_ORIGIN || resolved.pathname !== normalized) {
    throw new FlatQuestionAssetProtocolError(
      "Flat-question asset basePath must be a stable same-origin path",
    );
  }
  return normalized;
}

function assetPath(workspace: WorkspaceId): string {
  return `/api/workspaces/${encodeURIComponent(workspace)}/flat-question-assets`;
}

function requestPath(basePath: string, path: string): string {
  const requestPath = `${basePath}${path}`;
  const resolved = new URL(requestPath, SAFE_ORIGIN);
  if (resolved.origin !== SAFE_ORIGIN || resolved.pathname !== requestPath) {
    throw new FlatQuestionAssetProtocolError(
      "Flat-question asset request escaped its same-origin base",
    );
  }
  return requestPath;
}

function requireNoStore(response: Response, path: string): void {
  if (response.headers.get("cache-control")?.toLowerCase() !== "no-store") {
    throw new FlatQuestionAssetProtocolError(
      `Flat-question asset response ${path} must use Cache-Control: no-store`,
    );
  }
}

function requireJson(response: Response, path: string): void {
  const mediaType = response.headers.get("content-type")?.split(";", 1)[0]?.trim().toLowerCase();
  if (mediaType !== "application/json") {
    throw new FlatQuestionAssetProtocolError(
      `Flat-question asset response ${path} must use application/json`,
    );
  }
}

async function responseJson(response: Response, path: string): Promise<unknown> {
  requireNoStore(response, path);
  requireJson(response, path);
  const text = await response.text();
  if (text.length === 0 || text.length > MAX_RESPONSE_CHARACTERS) {
    throw new FlatQuestionAssetProtocolError(
      `Flat-question asset response ${path} must contain a bounded JSON body`,
    );
  }
  try {
    return JSON.parse(text) as unknown;
  } catch {
    throw new FlatQuestionAssetProtocolError(
      `Flat-question asset response ${path} must be valid JSON`,
    );
  }
}

function requireOnlyFields(record: Record<string, unknown>, path: string): void {
  const allowed = [
    "assetId",
    "contentChecksum",
    "displayLabel",
    "mediaType",
    "intrinsicWidth",
    "intrinsicHeight",
  ];
  for (const key of Object.keys(record)) {
    if (!allowed.includes(key)) {
      throw new DecodeError(`${path}.${key}`, "a field allowed by the image asset contract");
    }
  }
}

function decodeCanonicalUuid(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  if (!/^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/u.test(decoded)) {
    throw new DecodeError(path, "a lowercase canonical UUID");
  }
  return decoded;
}

function decodeSafeLabel(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  if (decoded.trim() === "" || decoded !== decoded.trim() || hasAsciiControl(decoded)) {
    throw new DecodeError(path, "a trimmed, nonempty image label without control characters");
  }
  return decoded;
}

function decodeMediaType(value: unknown, path: string): FlatQuestionAssetMediaType {
  const decoded = decodeString(value, path);
  if (!isAllowedMediaType(decoded)) {
    throw new DecodeError(path, "an allowed still-image media type");
  }
  return decoded;
}

function isAllowedMediaType(value: string): value is FlatQuestionAssetMediaType {
  return ALLOWED_MEDIA_TYPES.some((candidate) => candidate === value);
}

function decodeDimension(value: unknown, path: string): number {
  const decoded = decodePositiveInteger(value, path);
  if (decoded > 4_294_967_295) throw new DecodeError(path, "a positive u32 image dimension");
  return decoded;
}

/** Strictly decodes the safe browser projection and rejects private storage-field drift. */
export function decodeFlatQuestionAssetDescriptor(
  value: unknown,
  path = "response",
): FlatQuestionAssetDescriptor {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path);
  const descriptor: FlatQuestionAssetDescriptor = {
    assetId: decodeCanonicalUuid(decodeField(record, "assetId", path), `${path}.assetId`),
    contentChecksum: decodeString(
      decodeField(record, "contentChecksum", path),
      `${path}.contentChecksum`,
    ),
    displayLabel: decodeSafeLabel(
      decodeField(record, "displayLabel", path),
      `${path}.displayLabel`,
    ),
    mediaType: decodeMediaType(decodeField(record, "mediaType", path), `${path}.mediaType`),
    intrinsicWidth: decodeDimension(
      decodeField(record, "intrinsicWidth", path),
      `${path}.intrinsicWidth`,
    ),
    intrinsicHeight: decodeDimension(
      decodeField(record, "intrinsicHeight", path),
      `${path}.intrinsicHeight`,
    ),
  };
  if (!/^[0-9a-f]{64}$/u.test(descriptor.contentChecksum)) {
    throw new DecodeError(`${path}.contentChecksum`, "a lowercase SHA-256 checksum");
  }
  return descriptor;
}

export function decodeFlatQuestionAssetList(
  value: unknown,
  path = "response",
): ReadonlyArray<FlatQuestionAssetDescriptor> {
  return decodeArray(value, path, decodeFlatQuestionAssetDescriptor);
}

function safeHeaderValue(value: string, name: string): string {
  const normalized = value.trim();
  if (normalized === "" || normalized !== value || hasAsciiControl(value)) {
    throw new FlatQuestionAssetProtocolError(
      `Flat-question asset ${name} must be trimmed, nonempty, and free of control characters`,
    );
  }
  return normalized;
}

function uploadMediaType(image: Blob): FlatQuestionAssetMediaType {
  const mediaType = image.type.toLowerCase();
  if (!isAllowedMediaType(mediaType)) {
    throw new FlatQuestionAssetProtocolError("Choose a PNG, JPEG, or WebP image to upload");
  }
  return mediaType;
}

/** Creates the isolated author-image transport; it neither derives nor transmits storage metadata. */
export function createFlatQuestionAssetClient(
  config: FlatQuestionAssetClientConfig = {},
): FlatQuestionAssetClient {
  const fetchImplementation = config.fetch ?? browserFetch;
  const basePath = normalizeBasePath(config.basePath);
  return {
    async list(workspace: WorkspaceId): Promise<ReadonlyArray<FlatQuestionAssetDescriptor>> {
      const path = assetPath(workspace);
      const response = await fetchImplementation(requestPath(basePath, path), {
        method: "GET",
        headers: { accept: "application/json" },
        credentials: "same-origin",
        cache: "no-store",
      });
      if (!response.ok) throw new FlatQuestionAssetRequestError(response.status, path);
      const value = await responseJson(response, path);
      return decodeFlatQuestionAssetList(value);
    },
    async upload(
      workspace: WorkspaceId,
      upload: FlatQuestionAssetUpload,
    ): Promise<FlatQuestionAssetDescriptor> {
      if (upload.image.size <= 0) {
        throw new FlatQuestionAssetProtocolError("Choose an image to upload");
      }
      const mediaType = uploadMediaType(upload.image);
      const label = safeHeaderValue(upload.displayLabel, "image label");
      const provenance = safeHeaderValue(upload.provenance, "image provenance");
      const path = assetPath(workspace);
      const response = await fetchImplementation(requestPath(basePath, path), {
        method: "POST",
        headers: {
          accept: "application/json",
          "content-type": mediaType,
          "x-ple-asset-label": label,
          "x-ple-asset-provenance": provenance,
        },
        body: upload.image,
        credentials: "same-origin",
        cache: "no-store",
      });
      if (response.status !== 201) {
        if (!response.ok) throw new FlatQuestionAssetRequestError(response.status, path);
        throw new FlatQuestionAssetProtocolError(
          `Flat-question asset upload ${path} must return 201`,
        );
      }
      const value = await responseJson(response, path);
      return decodeFlatQuestionAssetDescriptor(value);
    },
  };
}
