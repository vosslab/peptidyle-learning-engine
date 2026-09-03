// editor_instructor_preview.ts - explicit, protected author-preview transport and DTO boundary.

import type { QuestionContentBlock } from "../../generated/api/QuestionContentBlock";
import type { QuestionBackend } from "../../generated/api/QuestionBackend";
import type { QuestionResponseFormat } from "../../generated/api/QuestionResponseFormat";
import type { WorkspaceId } from "../../generated/api/WorkspaceId";
import {
  DecodeError,
  decodeField,
  decodeNonemptyString,
  decodeRecord,
  decodeString,
  decodeStringEnum,
} from "../api/decoder";
import { decodeKeyFreeDraftPreview, decodeStudentFeedback } from "../api/decoders";

const QUESTION_BACKENDS: ReadonlyArray<QuestionBackend> = ["ple", "webwork", "qti", "imathas"];

const MAX_WORKSPACE_REVISION = 9_223_372_036_854_775_807n;

/** The safe presentation an instructor explicitly asks the server to derive. */
export interface InstructorPreviewPresentation {
  readonly title: string;
  readonly prompt: ReadonlyArray<QuestionContentBlock>;
  readonly response: QuestionResponseFormat;
  /** Display-ready blocks, not a reusable grading key or answer representation. */
  readonly questionAnswer: ReadonlyArray<QuestionContentBlock>;
  readonly questionAnswerExplanation?: ReadonlyArray<QuestionContentBlock>;
}

export type InstructorPreviewResult =
  | {
      readonly kind: "available";
      readonly revision: string;
      readonly presentation: InstructorPreviewPresentation;
    }
  | {
      readonly kind: "unavailable";
      readonly revision: string;
      readonly backend: QuestionBackend;
      readonly reason: string;
    };

type DecodedInstructorPreview =
  | { readonly kind: "available"; readonly presentation: InstructorPreviewPresentation }
  | { readonly kind: "unavailable"; readonly backend: QuestionBackend; readonly reason: string };

/** Narrow author-only capability injected into the workspace editor. */
export interface InstructorPreviewBoundary {
  readonly requestPresentation: (
    workspace: WorkspaceId,
    revision: string,
  ) => Promise<InstructorPreviewResult>;
}

/** Fetch-compatible dependency injected by tests or a non-browser host. */
export type InstructorPreviewFetch = (
  input: RequestInfo | URL,
  init?: RequestInit,
) => Promise<Response>;

export interface InstructorPreviewClientConfig {
  readonly fetch?: InstructorPreviewFetch;
}

/** A refused or failed protected preview request; its response body is intentionally not retained. */
export class InstructorPreviewRequestError extends Error {
  public readonly status: number;

  public constructor(status: number) {
    super(`Instructor preview request failed with status ${status}`);
    this.name = "InstructorPreviewRequestError";
    this.status = status;
  }
}

/** The protected route rejected the exact saved revision; the editor must offer reload recovery. */
export class InstructorPreviewConflictError extends InstructorPreviewRequestError {
  public constructor(status: 409 | 428) {
    super(status);
    this.name = "InstructorPreviewConflictError";
  }
}

function requireOnlyFields(
  record: Record<string, unknown>,
  path: string,
  allowed: ReadonlyArray<string>,
): void {
  for (const key of Object.keys(record)) {
    if (!allowed.includes(key)) {
      throw new DecodeError(`${path}.${key}`, "a field allowed by this response contract");
    }
  }
}

function decodePresentation(
  record: Record<string, unknown>,
  path: string,
  workspace: WorkspaceId,
): InstructorPreviewPresentation {
  requireOnlyFields(record, path, [
    "kind",
    "title",
    "prompt",
    "response",
    "questionAnswer",
    "questionAnswerExplanation",
  ]);
  const title = decodeNonemptyString(decodeField(record, "title", path), `${path}.title`);
  const prompt = decodeField(record, "prompt", path);
  const response = decodeField(record, "response", path);
  const safePreview = decodeKeyFreeDraftPreview(
    { workspace, title, prompt, response },
    `${path}.studentPresentation`,
  );
  const answer = decodeStudentFeedback(
    {
      questionAnswer: decodeField(record, "questionAnswer", path),
      ...("questionAnswerExplanation" in record
        ? {
            questionAnswerExplanation: decodeField(record, "questionAnswerExplanation", path),
          }
        : {}),
    },
    `${path}.authorPresentation`,
  );
  const questionAnswer = answer.questionAnswer;
  if (questionAnswer === undefined || questionAnswer.length === 0) {
    throw new DecodeError(`${path}.questionAnswer`, "a nonempty display-ready Question Answer");
  }
  const presentation: InstructorPreviewPresentation = {
    title: safePreview.title,
    prompt: safePreview.prompt,
    response: safePreview.response,
    questionAnswer,
    ...(answer.questionAnswerExplanation === undefined
      ? {}
      : { questionAnswerExplanation: answer.questionAnswerExplanation }),
  };
  return presentation;
}

/** Decodes the exact, server-redacted author-preview DTO before UI state can observe it. */
export function decodeInstructorPreview(
  value: unknown,
  workspace: WorkspaceId,
  path = "authorPreview",
): DecodedInstructorPreview {
  const record = decodeRecord(value, path);
  const kind = decodeString(decodeField(record, "kind", path), `${path}.kind`);
  switch (kind) {
    case "available": {
      const presentation = decodePresentation(record, path, workspace);
      return { kind, presentation };
    }
    case "unavailable": {
      requireOnlyFields(record, path, ["kind", "backend", "reason"]);
      return {
        kind,
        backend: decodeStringEnum(
          decodeField(record, "backend", path),
          `${path}.backend`,
          QUESTION_BACKENDS,
        ),
        reason: decodeNonemptyString(decodeField(record, "reason", path), `${path}.reason`),
      };
    }
    default:
      throw new DecodeError(`${path}.kind`, "available or unavailable");
  }
}

function authorPreviewPath(workspace: WorkspaceId): string {
  const encodedWorkspace = encodeURIComponent(workspace);
  return `/api/workspaces/${encodedWorkspace}/author-preview`;
}

function requireStrongRevision(value: string, message: string): void {
  if (!/^"[1-9][0-9]*"$/u.test(value)) {
    throw new Error(message);
  }
  const numericRevision = BigInt(value.slice(1, -1));
  if (numericRevision > MAX_WORKSPACE_REVISION) {
    throw new Error(message);
  }
}

function responseRevision(response: Response): string {
  const revision = response.headers.get("etag");
  if (revision === null) {
    throw new Error("Instructor preview response must include one strong numeric ETag");
  }
  requireStrongRevision(
    revision,
    "Instructor preview response must include one strong numeric ETag",
  );
  return revision;
}

function requireJson(response: Response): void {
  const contentType = response.headers.get("content-type");
  if (contentType === null || !contentType.toLowerCase().startsWith("application/json")) {
    throw new Error("Instructor preview response must be JSON");
  }
}

/**
 * Builds the one deliberately non-generic author-preview client. It only sends a request after an
 * instructor action, requests no persistent cache, and never accepts a caller-supplied URL.
 */
export function createInstructorPreviewClient(
  config: InstructorPreviewClientConfig = {},
): InstructorPreviewBoundary {
  const fetchImplementation = config.fetch ?? globalThis.fetch.bind(globalThis);
  return {
    requestPresentation: async (workspace, revision): Promise<InstructorPreviewResult> => {
      requireStrongRevision(
        revision,
        "Instructor preview revision must be one strong numeric ETag",
      );
      const path = authorPreviewPath(workspace);
      const response = await fetchImplementation(path, {
        headers: { accept: "application/json", "if-match": revision },
        credentials: "same-origin",
        cache: "no-store",
      });
      if (!response.ok) {
        if (response.status === 409 || response.status === 428) {
          throw new InstructorPreviewConflictError(response.status);
        }
        throw new InstructorPreviewRequestError(response.status);
      }
      requireJson(response);
      const receivedRevision = responseRevision(response);
      if (receivedRevision !== revision) {
        throw new Error("Instructor preview response revision does not match the saved draft");
      }
      const text = await response.text();
      let payload: unknown;
      try {
        payload = JSON.parse(text) as unknown;
      } catch {
        throw new Error("Instructor preview response was not valid JSON");
      }
      const decoded = decodeInstructorPreview(payload, workspace, "response");
      if (decoded.kind === "available") {
        return {
          kind: decoded.kind,
          revision: receivedRevision,
          presentation: decoded.presentation,
        };
      }
      return {
        kind: decoded.kind,
        revision: receivedRevision,
        backend: decoded.backend,
        reason: decoded.reason,
      };
    },
  };
}
