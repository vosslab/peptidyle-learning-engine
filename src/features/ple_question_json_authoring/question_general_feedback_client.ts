import type { DraftQuestionReference } from "../../../generated/api/DraftQuestionReference";

const MAX_RESPONSE_BYTES = 4 * 1024 * 1024;
const SAFE_ORIGIN = "https://ple-question-general-feedback.invalid";

export type PleQuestionGeneralFeedbackRead = {
  readonly generalFeedback: string | null;
  readonly revision: string;
};

export type PleQuestionGeneralFeedbackSave = {
  readonly revision: string;
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
  load(draftQuestion: DraftQuestionReference): Promise<PleQuestionGeneralFeedbackRead>;
  save(
    draftQuestion: DraftQuestionReference,
    generalFeedback: string | null,
    revision: string,
  ): Promise<PleQuestionGeneralFeedbackSave>;
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

function metadataPath(draftQuestion: DraftQuestionReference): string {
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

function validRevision(value: string): string {
  if (!/^"[1-9][0-9]*"$/u.test(value)) {
    throw new PleQuestionGeneralFeedbackProtocolError(
      "Question general feedback revision must be one positive strong numeric ETag",
    );
  }
  if (BigInt(value.slice(1, -1)) > 9_223_372_036_854_775_807n) {
    throw new PleQuestionGeneralFeedbackProtocolError(
      "Question general feedback revision must fit in a signed 64-bit integer",
    );
  }
  return value;
}

function strongRevision(response: Response, path: string): string {
  const revision = response.headers.get("etag");
  if (revision === null) {
    throw new PleQuestionGeneralFeedbackProtocolError(
      `Question general feedback response ${path} must include one strong numeric ETag`,
    );
  }
  return validRevision(revision);
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

function decodeMetadata(value: unknown, path: string): string | null {
  if (
    typeof value !== "object" ||
    value === null ||
    Array.isArray(value) ||
    Object.keys(value).length !== 1 ||
    !("generalFeedback" in value) ||
    ((value as Record<string, unknown>).generalFeedback !== null &&
      typeof (value as Record<string, unknown>).generalFeedback !== "string")
  ) {
    throw new PleQuestionGeneralFeedbackProtocolError(
      `Question general feedback response ${path} must contain only generalFeedback`,
    );
  }
  return (value as { readonly generalFeedback: string | null }).generalFeedback;
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
    draftQuestion: DraftQuestionReference,
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
      generalFeedback: decodeMetadata(await boundedJson(response, path), path),
      revision: strongRevision(response, path),
    };
  }

  async function save(
    draftQuestion: DraftQuestionReference,
    generalFeedback: string | null,
    revision: string,
  ): Promise<PleQuestionGeneralFeedbackSave> {
    const path = metadataPath(draftQuestion);
    const response = await fetchImplementation(
      sameOriginPath(basePath, path),
      requestInit(
        "PUT",
        {
          accept: "application/json",
          "content-type": "application/json",
          "if-match": validRevision(revision),
        },
        JSON.stringify({ generalFeedback }),
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
    return { revision: strongRevision(response, path) };
  }

  return { load, save };
}
