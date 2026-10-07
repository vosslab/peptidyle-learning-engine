import type { QuestionBackend } from "../../generated/api/QuestionBackend";
import type { QuestionFormat } from "../../generated/api/QuestionFormat";
import { ApiProtocolError, ApiRequestError } from "./http_client/error";
import { createDefaultPleQuestionJsonDraft } from "../features/ple_question_json_authoring/question_json_defaults";
import { serializePleQuestionJsonSource } from "../features/ple_question_json_authoring/question_json_codec";
import { parseDraftQuestionId, type DraftQuestionRouteId } from "../navigation/public_route";

const DRAFTS_PATH = "/api/authoring/drafts";
const MAX_WEBWORK_PG_PATH_BYTES = 1_024;

export type QuestionDraftCreationFormat = Extract<
  QuestionFormat,
  "pleQuestionJson" | "webworkPg" | "webworkPgml"
>;

export const QUESTION_DRAFT_CREATION_FORMATS: ReadonlyArray<{
  readonly format: QuestionDraftCreationFormat;
  readonly label: string;
}> = [
  { format: "pleQuestionJson", label: "Native JSON" },
  { format: "webworkPg", label: "WebWork PG" },
  { format: "webworkPgml", label: "WebWork PGML" },
];

export function isQuestionDraftCreationFormat(value: string): value is QuestionDraftCreationFormat {
  return QUESTION_DRAFT_CREATION_FORMATS.some((item) => item.format === value);
}

export type QuestionDraftCreationInput = {
  readonly format: QuestionDraftCreationFormat;
  readonly source?: string;
  readonly webworkPgPath?: string;
};

type NativeQuestionDraftRequest = {
  readonly metadata: ReturnType<typeof createDefaultPleQuestionJsonDraft>["metadata"];
  readonly questionBackend: Extract<QuestionBackend, "ple">;
  readonly questionFormat: Extract<QuestionFormat, "pleQuestionJson">;
  readonly webworkPgPath: null;
  readonly source: string;
};

type WebworkQuestionDraftRequest = {
  readonly metadata: ReturnType<typeof createDefaultPleQuestionJsonDraft>["metadata"];
  readonly questionBackend: Extract<QuestionBackend, "webwork">;
  readonly questionFormat: Extract<QuestionFormat, "webworkPg" | "webworkPgml">;
  readonly webworkPgPath: string;
  readonly source: string;
};

export type QuestionDraftCreationRequest = NativeQuestionDraftRequest | WebworkQuestionDraftRequest;

export type CreatedQuestionDraft = {
  readonly draftQuestionId: DraftQuestionRouteId;
  readonly draftQuestionEditNumber: string;
};

export type QuestionDraftCreationFetch = (
  input: RequestInfo | URL,
  init?: RequestInit,
) => Promise<Response>;

export type QuestionDraftCreationClientConfig = {
  readonly fetch?: QuestionDraftCreationFetch;
};

export interface QuestionDraftCreationClient {
  createDraft(input: QuestionDraftCreationInput): Promise<CreatedQuestionDraft>;
}

/** Matches the registered relative WebWork path bound to a Draft at creation. */
export function isAllowedWebworkPgPath(value: string): boolean {
  return (
    value.length > 0 &&
    new TextEncoder().encode(value).length <= MAX_WEBWORK_PG_PATH_BYTES &&
    !value.startsWith("/") &&
    !value.includes("\\") &&
    !value.includes("\0") &&
    !value.split("/").some((part) => part.length === 0 || part === "." || part === "..")
  );
}

/** Builds an explicit, immutable source binding for the ordinary New Draft flow. */
export function questionDraftCreationRequest(
  input: QuestionDraftCreationInput,
): QuestionDraftCreationRequest {
  const defaults = createDefaultPleQuestionJsonDraft();
  if (input.format === "pleQuestionJson") {
    if (input.webworkPgPath !== undefined && input.webworkPgPath !== "") {
      throw new ApiProtocolError("Native JSON Drafts do not use a WebWork PG path");
    }
    return {
      metadata: defaults.metadata,
      questionBackend: "ple",
      questionFormat: "pleQuestionJson",
      webworkPgPath: null,
      source: input.source ?? serializePleQuestionJsonSource(defaults.source),
    };
  }

  const webworkPgPath = input.webworkPgPath;
  if (webworkPgPath === undefined || !isAllowedWebworkPgPath(webworkPgPath)) {
    throw new ApiProtocolError("Enter an allowed relative WebWork PG path for this Draft");
  }
  return {
    metadata: defaults.metadata,
    questionBackend: "webwork",
    questionFormat: input.format,
    webworkPgPath,
    source: input.source ?? "",
  };
}

function isPositiveEditNumber(value: unknown): value is string {
  if (typeof value !== "string" || !/^[1-9][0-9]*$/u.test(value)) return false;
  return BigInt(value) <= 9_223_372_036_854_775_807n;
}

function decodeCreatedQuestionDraft(value: unknown): CreatedQuestionDraft {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new ApiProtocolError("Draft creation returned an invalid response");
  }
  const record = value as Record<string, unknown>;
  if (
    Object.keys(record).length !== 2 ||
    typeof record.draftQuestionId !== "string" ||
    !isPositiveEditNumber(record.draftQuestionEditNumber)
  ) {
    throw new ApiProtocolError("Draft creation returned an invalid response");
  }
  const draftQuestionId = parseDraftQuestionId(record.draftQuestionId);
  if (draftQuestionId === null) {
    throw new ApiProtocolError("Draft creation returned an invalid Draft Question ID");
  }
  return { draftQuestionId, draftQuestionEditNumber: record.draftQuestionEditNumber };
}

export function createQuestionDraftCreationClient(
  config: QuestionDraftCreationClientConfig = {},
): QuestionDraftCreationClient {
  const fetchImplementation = config.fetch ?? globalThis.fetch;

  async function createDraft(input: QuestionDraftCreationInput): Promise<CreatedQuestionDraft> {
    const body = questionDraftCreationRequest(input);
    const response = await fetchImplementation(DRAFTS_PATH, {
      method: "POST",
      headers: {
        accept: "application/json",
        "content-type": "application/json",
      },
      body: JSON.stringify(body),
      credentials: "same-origin",
      cache: "no-store",
    });
    if (!response.ok) throw new ApiRequestError(response.status, DRAFTS_PATH);
    return decodeCreatedQuestionDraft((await response.json()) as unknown);
  }

  return { createDraft };
}
