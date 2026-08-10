import type { AssignmentId } from "../../../generated/api/AssignmentId";
import type { AssignmentRun } from "../../../generated/api/AssignmentRun";
import type { CourseAppearance } from "../../../generated/api/CourseAppearance";
import type { CourseAppearanceUpdate } from "../../../generated/api/CourseAppearanceUpdate";
import type { CourseBannerCandidateReceipt } from "../../../generated/api/CourseBannerCandidateReceipt";
import type { CourseId } from "../../../generated/api/CourseId";
import type { QuestionAttemptId } from "../../../generated/api/QuestionAttemptId";
import type { StudentResponse } from "../../../generated/api/StudentResponse";
import type { WorkspaceId } from "../../../generated/api/WorkspaceId";
import type { PublicationScope } from "../../../generated/api/PublicationScope";
import type { ApiClient } from "../client";
import type {
  AssignmentEditorDetail,
  FeedbackReleaseResponse,
  PublicationResult,
  PublicationValidationResponse,
  PrefetchedNextQuestion,
  WorkspaceDraftDetail,
} from "../contracts";
import {
  decodeAssignmentCapabilityViolations,
  decodeAssignmentEditorDetail,
  decodeAssignmentEditorInput,
  decodeAssignmentRun,
  decodeCapabilityViolations,
  decodeCourseAppearance,
  decodeCourseBannerCandidateReceipt,
  decodeDraftQuestionDefinition,
  decodeFeedbackReleaseResponse,
  decodePrefetchedNextQuestion,
  decodePublicationReadinessFailure,
  decodePublicationResult,
  decodePublicationValidationFailure,
  decodePublicationValidationReport,
  decodeResponseFormatReport,
  decodeStudentResponse,
  decodeSubmissionReceipt,
  decodeTimerVerdict,
} from "../decoders";
import {
  ApiProtocolError,
  ApiRequestError,
  AssignmentConflictError,
  AssignmentValidationError,
  CourseAppearanceConflictError,
  CourseAppearanceFileError,
  PublicationValidationError,
  WorkspaceConflictError,
} from "./error";
import {
  MAX_RESPONSE_CHARACTERS,
  boundedResponseJson,
  decodeJson,
  requireNoStore,
  responseContentType,
} from "./response";

/** Fetch-compatible dependency injected by tests or a non-browser host. */
export type ApiFetch = (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>;

/** Configuration that cannot redirect credentials to another origin. */
export interface HttpApiClientConfig {
  readonly fetch?: ApiFetch;
  readonly basePath?: string;
}
export interface RequestOptions {
  readonly method?: "GET" | "POST" | "PUT" | "DELETE";
  readonly body?: unknown;
  readonly headers?: Readonly<Record<string, string>>;
}
export function normalizeBasePath(value: string | undefined): string {
  if (value === undefined || value === "" || value === "/") return "";
  if (
    !value.startsWith("/") ||
    value.startsWith("//") ||
    value.includes("?") ||
    value.includes("#")
  )
    throw new Error("API basePath must be a same-origin path without query or fragment");
  return value.replace(/\/+$/, "");
}
export function browserFetch(input: RequestInfo | URL, init?: RequestInit): Promise<Response> {
  return globalThis.fetch(input, init);
}
export function requestPath(basePath: string, path: string): string {
  return `${basePath}${path}`;
}
export function cursorPath(path: string, cursor: string | undefined): string {
  return cursor === undefined ? path : `${path}?${new URLSearchParams({ cursor }).toString()}`;
}
export function encodedId(value: string): string {
  return encodeURIComponent(value);
}
export async function requestJson<T>(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  decoder: (value: unknown, path?: string) => T,
  options: RequestOptions = {},
): Promise<T> {
  const headers: Record<string, string> = { accept: "application/json", ...options.headers };
  const body = options.body === undefined ? undefined : JSON.stringify(options.body);
  if (body !== undefined) headers["content-type"] = "application/json";
  const response = await fetchImplementation(requestPath(basePath, path), {
    method: options.method ?? "GET",
    headers,
    body,
    credentials: "same-origin",
    cache: "no-store",
  });
  if (!response.ok) throw new ApiRequestError(response.status, path);
  responseContentType(response, path);
  const text = await response.text();
  if (text.length === 0 || text.length > MAX_RESPONSE_CHARACTERS)
    throw new ApiProtocolError(
      `API response ${path} must contain 1 to ${MAX_RESPONSE_CHARACTERS} JSON characters`,
    );
  return decoder(decodeJson(text, path), "response");
}

function workspacePath(workspace: WorkspaceId): string {
  return `/api/workspaces/${encodedId(workspace)}`;
}
function validRevision(value: string): boolean {
  return /^"[1-9][0-9]*"$/u.test(value) && BigInt(value.slice(1, -1)) <= 9_223_372_036_854_775_807n;
}
function workspaceRevision(response: Response, path: string): string {
  const value = response.headers.get("etag");
  if (value === null || !/^"[0-9]+"$/u.test(value))
    throw new ApiProtocolError(`API response ${path} must include one strong numeric ETag`);
  return value;
}
function appearancePath(courseId: CourseId): string {
  return `/api/courses/${encodedId(courseId)}/appearance`;
}
function strongAppearanceRevision(value: string): string {
  if (!/^[1-9][0-9]*$/u.test(value) || BigInt(value) > 9_223_372_036_854_775_807n)
    throw new ApiProtocolError("Course appearance needs a canonical positive revision");
  return `"${value}"`;
}
function assignmentPath(courseId: CourseId, assignmentId?: AssignmentId): string {
  const course = encodedId(courseId);
  return assignmentId === undefined
    ? `/api/courses/${course}/assignments`
    : `/api/courses/${course}/assignments/${encodedId(assignmentId)}`;
}

async function workspaceDraft(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  options: RequestOptions = {},
): Promise<WorkspaceDraftDetail> {
  const headers: Record<string, string> = { accept: "application/json", ...options.headers };
  const body = options.body === undefined ? undefined : JSON.stringify(options.body);
  if (body !== undefined) headers["content-type"] = "application/json";
  const response = await fetchImplementation(requestPath(basePath, path), {
    method: options.method ?? "GET",
    headers,
    body,
    credentials: "same-origin",
    cache: "no-store",
  });
  if (response.status === 409 || response.status === 428)
    throw new WorkspaceConflictError(response.status, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  responseContentType(response, path);
  const text = await response.text();
  if (text.length === 0 || text.length > MAX_RESPONSE_CHARACTERS)
    throw new ApiProtocolError(
      `API response ${path} must contain 1 to ${MAX_RESPONSE_CHARACTERS} JSON characters`,
    );
  return {
    draft: decodeDraftQuestionDefinition(decodeJson(text, path), "response"),
    revision: workspaceRevision(response, path),
  };
}

export async function requestAssignmentEditor(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  expected: { readonly assignmentId?: AssignmentId; readonly courseId?: CourseId },
  options: RequestOptions = {},
): Promise<AssignmentEditorDetail> {
  const headers: Record<string, string> = { accept: "application/json", ...options.headers };
  const body = options.body === undefined ? undefined : JSON.stringify(options.body);
  if (body !== undefined) headers["content-type"] = "application/json";
  const response = await fetchImplementation(requestPath(basePath, path), {
    method: options.method ?? "GET",
    headers,
    body,
    credentials: "same-origin",
    cache: "no-store",
  });
  if (response.status === 409 || response.status === 428)
    throw new AssignmentConflictError(response.status, path);
  responseContentType(response, path);
  const text = await response.text();
  if (text.length === 0 || text.length > MAX_RESPONSE_CHARACTERS)
    throw new ApiProtocolError(`API response ${path} must contain a bounded JSON body`);
  const value = decodeJson(text, path);
  if (
    response.status === 422 &&
    typeof value === "object" &&
    value !== null &&
    !Array.isArray(value) &&
    "error" in value &&
    Reflect.get(value, "error") === "assignment configuration is not supported"
  )
    throw new AssignmentValidationError(
      path,
      decodeAssignmentCapabilityViolations(value, "response"),
    );
  if (!response.ok) throw new ApiRequestError(response.status, path);
  const detail = decodeAssignmentEditorDetail(value, "response");
  if (expected.assignmentId !== undefined && detail.id !== expected.assignmentId)
    throw new ApiProtocolError(
      "assignment editor response does not match the requested assignment",
    );
  if (expected.courseId !== undefined && detail.courseId !== expected.courseId)
    throw new ApiProtocolError("assignment editor response does not match the requested course");
  const revision = response.headers.get("etag");
  if (revision === null || !validRevision(revision))
    throw new ApiProtocolError(
      `API response ${path} must include one positive strong numeric ETag`,
    );
  return { ...detail, revision };
}

export function createRequestClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<
  ApiClient,
  | "saveWorkspaceDraft"
  | "deleteWorkspaceDraft"
  | "validateWorkspacePublication"
  | "publishWorkspace"
  | "uploadCourseBannerCandidate"
  | "saveCourseAppearance"
  | "createAssignment"
  | "saveAssignment"
  | "startRun"
  | "prefetchNextQuestion"
  | "submitResponse"
  | "releaseAttemptFeedback"
  | "validateResponseFormatOnServer"
  | "timerVerdictOnServer"
  | "validateAssignmentConfigOnServer"
> {
  return {
    saveWorkspaceDraft: (
      workspace,
      draft,
      revision,
    ): ReturnType<ApiClient["saveWorkspaceDraft"]> => {
      if (draft.workspace !== workspace)
        return Promise.reject(
          new ApiProtocolError("workspace save path does not match draft body"),
        );
      if (revision !== undefined && !/^"[0-9]+"$/u.test(revision))
        return Promise.reject(
          new ApiProtocolError("workspace revision must be one strong numeric ETag"),
        );
      return workspaceDraft(fetchImplementation, basePath, workspacePath(workspace), {
        method: "PUT",
        body: draft,
        headers: revision === undefined ? {} : { "if-match": revision },
      });
    },
    deleteWorkspaceDraft: async (workspace, revision): Promise<void> => {
      const path = workspacePath(workspace);
      if (!/^"[0-9]+"$/u.test(revision))
        throw new ApiProtocolError("workspace revision must be one strong numeric ETag");
      const response = await fetchImplementation(requestPath(basePath, path), {
        method: "DELETE",
        headers: { accept: "application/json", "if-match": revision },
        credentials: "same-origin",
        cache: "no-store",
      });
      if (response.status === 409 || response.status === 428)
        throw new WorkspaceConflictError(response.status, path);
      if (!response.ok) throw new ApiRequestError(response.status, path);
      if (response.status !== 204) throw new ApiProtocolError(`API response ${path} must be 204`);
    },
    validateWorkspacePublication: async (workspace): Promise<PublicationValidationResponse> => {
      const path = `${workspacePath(workspace)}/publication-validation`;
      const response = await fetchImplementation(requestPath(basePath, path), {
        method: "POST",
        headers: { accept: "application/json" },
        credentials: "same-origin",
        cache: "no-store",
      });
      responseContentType(response, path);
      const text = await response.text();
      if (text.length === 0 || text.length > MAX_RESPONSE_CHARACTERS)
        throw new ApiProtocolError(`API response ${path} must contain a bounded JSON body`);
      const value = decodeJson(text, path);
      if (response.status === 422) return decodePublicationReadinessFailure(value, "response");
      if (!response.ok) throw new ApiRequestError(response.status, path);
      const report = decodePublicationValidationReport(value, "response");
      return {
        kind: "capabilityReport",
        revision: workspaceRevision(response, path),
        violations: report.violations,
      };
    },
    publishWorkspace: async (
      workspace,
      scope: PublicationScope,
      revision: string,
    ): Promise<PublicationResult> => {
      if (scope !== "institution" && scope !== "public")
        throw new ApiProtocolError("publication scope must be institution or public");
      if (!validRevision(revision))
        throw new ApiProtocolError("publication revision must be one positive strong numeric ETag");
      const path = `/api/problems/${encodedId(workspace)}/publish`;
      const response = await fetchImplementation(requestPath(basePath, path), {
        method: "POST",
        headers: {
          accept: "application/json",
          "content-type": "application/json",
          "if-match": revision,
        },
        body: JSON.stringify({ scope }),
        credentials: "same-origin",
        cache: "no-store",
      });
      responseContentType(response, path);
      const text = await response.text();
      if (text.length === 0 || text.length > MAX_RESPONSE_CHARACTERS)
        throw new ApiProtocolError(`API response ${path} must contain a bounded JSON body`);
      const value = decodeJson(text, path);
      if (response.status === 409 || response.status === 428)
        throw new WorkspaceConflictError(response.status, path);
      if (response.status === 422) {
        const failure = decodePublicationValidationFailure(value, "response");
        throw new PublicationValidationError(path, failure.message, failure.violations);
      }
      if (!response.ok) throw new ApiRequestError(response.status, path);
      return decodePublicationResult(value, "response");
    },
    uploadCourseBannerCandidate: async (courseId, image): Promise<CourseBannerCandidateReceipt> => {
      if (image.size <= 0) throw new CourseAppearanceFileError("Course banner image is empty");
      if (image.size > 2 * 1_024 * 1_024)
        throw new CourseAppearanceFileError("Course banner image exceeds 2 MiB");
      if (!["image/jpeg", "image/png", "image/webp"].includes(image.type))
        throw new CourseAppearanceFileError("Course banner must be JPEG, PNG, or WebP");
      const path = `${appearancePath(courseId)}/banner-candidates`;
      const response = await fetchImplementation(requestPath(basePath, path), {
        method: "POST",
        headers: { accept: "application/json", "content-type": image.type },
        body: image,
        credentials: "same-origin",
        cache: "no-store",
      });
      if (response.status !== 201) {
        if (!response.ok) throw new ApiRequestError(response.status, path);
        throw new ApiProtocolError(`API response ${path} must use status 201`);
      }
      return decodeCourseBannerCandidateReceipt(await boundedResponseJson(response, path));
    },
    saveCourseAppearance: async (
      courseId,
      update: CourseAppearanceUpdate,
      revision: string,
    ): Promise<CourseAppearance> => {
      const path = appearancePath(courseId);
      const response = await fetchImplementation(requestPath(basePath, path), {
        method: "PUT",
        headers: {
          accept: "application/json",
          "content-type": "application/json",
          "if-match": strongAppearanceRevision(revision),
        },
        body: JSON.stringify(update),
        credentials: "same-origin",
        cache: "no-store",
      });
      requireNoStore(response, path);
      if (response.status === 412) throw new CourseAppearanceConflictError(path);
      if (!response.ok) throw new ApiRequestError(response.status, path);
      const appearance = decodeCourseAppearance(await boundedResponseJson(response, path));
      if (response.headers.get("etag") !== strongAppearanceRevision(appearance.revision))
        throw new ApiProtocolError(
          `API response ${path} ETag does not match its appearance revision`,
        );
      return appearance;
    },
    createAssignment: (courseId, input) =>
      requestAssignmentEditor(
        fetchImplementation,
        basePath,
        assignmentPath(courseId),
        { courseId },
        { method: "POST", body: decodeAssignmentEditorInput(input, "request") },
      ),
    saveAssignment: (
      courseId,
      assignmentId,
      input,
      revision,
    ): ReturnType<ApiClient["saveAssignment"]> => {
      if (!validRevision(revision))
        return Promise.reject(
          new ApiProtocolError("assignment revision must be one positive strong numeric ETag"),
        );
      return requestAssignmentEditor(
        fetchImplementation,
        basePath,
        assignmentPath(courseId, assignmentId),
        { courseId, assignmentId },
        {
          method: "PUT",
          body: decodeAssignmentEditorInput(input, "request"),
          headers: { "if-match": revision },
        },
      );
    },
    startRun: (assignmentId): Promise<AssignmentRun> =>
      requestJson(fetchImplementation, basePath, "/api/runs", decodeAssignmentRun, {
        method: "POST",
        body: { assignmentId },
      }),
    prefetchNextQuestion: async (attemptId, signal): Promise<PrefetchedNextQuestion | null> => {
      const path = `/api/attempts/${encodedId(attemptId)}/prefetch-next`;
      const response = await fetchImplementation(requestPath(basePath, path), {
        method: "POST",
        headers: { accept: "application/json" },
        credentials: "same-origin",
        cache: "no-store",
        signal,
      });
      if (response.status === 204) return null;
      if (!response.ok) throw new ApiRequestError(response.status, path);
      const decoded = decodePrefetchedNextQuestion(
        await boundedResponseJson(response, path),
        "response",
      );
      if (decoded.predecessor !== attemptId)
        throw new ApiProtocolError("Prefetched question predecessor does not match its request");
      return decoded;
    },
    submitResponse: async (
      attemptId: QuestionAttemptId,
      response: StudentResponse,
      idempotencyKey: string,
    ): ReturnType<ApiClient["submitResponse"]> => {
      const decoded = decodeStudentResponse(response, "request.response");
      const path =
        decoded.kind === "externalTool"
          ? `/api/attempts/${encodedId(attemptId)}/external-tool/launch/submission`
          : `/api/submissions/${encodedId(attemptId)}`;
      const receipt = await requestJson(
        fetchImplementation,
        basePath,
        path,
        decodeSubmissionReceipt,
        {
          method: "POST",
          headers: { "idempotency-key": idempotencyKey },
          body: { response: decoded },
        },
      );
      if (receipt.attempt.id !== attemptId)
        throw new ApiProtocolError("Submission receipt attempt does not match its request");
      if (
        receipt.nextIssued !== null &&
        (receipt.nextIssued.run !== receipt.attempt.run ||
          receipt.nextIssued.id === receipt.attempt.id)
      )
        throw new ApiProtocolError("Submission receipt next attempt is not bound to its response");
      return receipt;
    },
    releaseAttemptFeedback: (attemptId): Promise<FeedbackReleaseResponse> =>
      requestJson(
        fetchImplementation,
        basePath,
        `/api/attempts/${encodedId(attemptId)}/feedback-release`,
        decodeFeedbackReleaseResponse,
        { method: "POST" },
      ),
    validateResponseFormatOnServer: (definition, response) =>
      requestJson(
        fetchImplementation,
        basePath,
        "/api/validation/response-format",
        decodeResponseFormatReport,
        { method: "POST", body: { definition, response } },
      ),
    timerVerdictOnServer: (evaluation) =>
      requestJson(fetchImplementation, basePath, "/api/validation/timer", decodeTimerVerdict, {
        method: "POST",
        body: evaluation,
      }),
    validateAssignmentConfigOnServer: (validationConfig) =>
      requestJson(
        fetchImplementation,
        basePath,
        "/api/validation/assignment-capabilities",
        decodeCapabilityViolations,
        { method: "POST", body: validationConfig },
      ),
  };
}
