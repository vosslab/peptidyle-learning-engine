import type { AssignmentId } from "../../../generated/api/AssignmentId";
import type { AssignmentAttempt } from "../../../generated/api/AssignmentAttempt";
import type { CourseId } from "../../../generated/api/CourseId";
import type { QuestionAttemptId } from "../../../generated/api/QuestionAttemptId";
import type { ApiClient } from "../client";
import type {
  AssignmentEditorDetail,
  AssignmentContentInput,
  AssignmentCreateInput,
  StudentFeedbackReleaseResponse,
  InstructorStudentView,
} from "../contracts";
import {
  decodeAssignmentContentInput,
  decodeInstructorStudentView,
  decodeAssignmentAttempt,
  decodeCapabilityViolations,
  decodeStudentFeedbackReleaseResponse,
  decodeStudentResponseFormatCheck,
  decodeQuestionAttemptTimingDecision,
} from "../decoders";
import { decodeAssignmentEditorDetail } from "../decoders/assignment_workspace";
import { ApiProtocolError, ApiRequestError, AssignmentConflictError } from "./error";
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

/**
 * Same-origin transport dispatch for every browser API operation.
 *
 * Mutation routes receive the browser's same-origin request context here; feature
 * clients add only their closed body and strong revision headers through
 * `RequestOptions`. Keeping dispatch in one owner prevents a feature-specific
 * fetch path from drifting from the deployed cookie and cache boundary.
 */
export async function requestSameOrigin(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  options: RequestOptions = {},
): Promise<Response> {
  const headers: Record<string, string> = { accept: "application/json", ...options.headers };
  const body = options.body === undefined ? undefined : JSON.stringify(options.body);
  if (body !== undefined) headers["content-type"] = "application/json";
  return fetchImplementation(requestPath(basePath, path), {
    method: options.method ?? "GET",
    headers,
    body,
    credentials: "same-origin",
    cache: "no-store",
  });
}

export async function requestJson<T>(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  decoder: (value: unknown, path?: string) => T,
  options: RequestOptions = {},
): Promise<T> {
  const response = await requestSameOrigin(fetchImplementation, basePath, path, options);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  responseContentType(response, path);
  const text = await response.text();
  if (text.length === 0 || text.length > MAX_RESPONSE_CHARACTERS)
    throw new ApiProtocolError(
      `API response ${path} must contain 1 to ${MAX_RESPONSE_CHARACTERS} JSON characters`,
    );
  return decoder(decodeJson(text, path), "response");
}

function validRevision(value: string): boolean {
  return /^"[1-9][0-9]*"$/u.test(value) && BigInt(value.slice(1, -1)) <= 9_223_372_036_854_775_807n;
}

/** Converts the transport ETag into the exact Assignment edit precondition. */
function assignmentEditPrecondition(assignmentEditEtag: string): string {
  if (!validRevision(assignmentEditEtag))
    throw new ApiProtocolError("assignment edit number must be one positive strong numeric ETag");
  return assignmentEditEtag.slice(1, -1);
}
function assignmentPath(courseId: CourseId, assignmentId?: AssignmentId): string {
  const course = encodedId(courseId);
  return assignmentId === undefined
    ? `/api/courses/${course}/assignments`
    : `/api/courses/${course}/assignments/${encodedId(assignmentId)}`;
}

export function studentAttemptPath(
  courseId: CourseId,
  assignmentId: AssignmentId,
  attemptId: QuestionAttemptId,
): string {
  return `${assignmentPath(courseId, assignmentId)}/attempts/${encodedId(attemptId)}`;
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
  requireNoStore(response, path);
  if (response.status === 409 || response.status === 412 || response.status === 428)
    throw new AssignmentConflictError(response.status, path);
  responseContentType(response, path);
  const text = await response.text();
  if (text.length === 0 || text.length > MAX_RESPONSE_CHARACTERS)
    throw new ApiProtocolError(`API response ${path} must contain a bounded JSON body`);
  const value = decodeJson(text, path);
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
  | "createAssignment"
  | "getAssignmentWorkspace"
  | "saveAssignmentContent"
  | "getInstructorStudentView"
  | "startAssignmentAttempt"
  | "releaseStudentFeedback"
  | "validateResponseFormatOnServer"
  | "questionAttemptTimingDecisionOnServer"
  | "validateAssignmentConfigOnServer"
> {
  return {
    getAssignmentWorkspace: (courseId, assignmentId) =>
      requestAssignmentEditor(
        fetchImplementation,
        basePath,
        assignmentPath(courseId, assignmentId),
        { courseId, assignmentId },
      ),
    createAssignment: (
      courseId,
      input: AssignmentCreateInput,
    ): ReturnType<ApiClient["createAssignment"]> => {
      if (typeof input.title !== "string" || input.title.trim().length === 0)
        return Promise.reject(new ApiProtocolError("Assignment needs a nonempty title"));
      return requestAssignmentEditor(
        fetchImplementation,
        basePath,
        assignmentPath(courseId),
        { courseId },
        { method: "POST", body: { title: input.title } },
      );
    },
    saveAssignmentContent: (
      courseId,
      assignmentId,
      _assignmentReference,
      input: AssignmentContentInput,
      assignmentEtag,
    ): ReturnType<ApiClient["saveAssignmentContent"]> => {
      const baseEditNumber = assignmentEditPrecondition(assignmentEtag);
      return requestAssignmentEditor(
        fetchImplementation,
        basePath,
        `${assignmentPath(courseId, assignmentId)}/content`,
        { courseId, assignmentId },
        {
          method: "PUT",
          body: { ...decodeAssignmentContentInput(input, "request"), baseEditNumber },
          headers: { "if-match": assignmentEtag },
        },
      );
    },
    getInstructorStudentView: async (courseId, assignmentId): Promise<InstructorStudentView> => {
      const path = `${assignmentPath(courseId, assignmentId)}/student-view`;
      const response = await fetchImplementation(requestPath(basePath, path), {
        method: "GET",
        headers: { accept: "application/json" },
        credentials: "same-origin",
        cache: "no-store",
      });
      requireNoStore(response, path);
      if (!response.ok) throw new ApiRequestError(response.status, path);
      return decodeInstructorStudentView(await boundedResponseJson(response, path), "response");
    },
    startAssignmentAttempt: (courseId, assignmentId): Promise<AssignmentAttempt> =>
      requestJson(
        fetchImplementation,
        basePath,
        `${assignmentPath(courseId, assignmentId)}/assignment-attempts`,
        decodeAssignmentAttempt,
        {
          method: "POST",
        },
      ),
    releaseStudentFeedback: (attemptId): Promise<StudentFeedbackReleaseResponse> =>
      requestJson(
        fetchImplementation,
        basePath,
        `/api/attempts/${encodedId(attemptId)}/student-feedback-release`,
        decodeStudentFeedbackReleaseResponse,
        { method: "POST" },
      ),
    validateResponseFormatOnServer: (responseFormat, response) =>
      requestJson(
        fetchImplementation,
        basePath,
        "/api/validation/response-format",
        decodeStudentResponseFormatCheck,
        { method: "POST", body: { responseFormat, response } },
      ),
    questionAttemptTimingDecisionOnServer: (evaluation) =>
      requestJson(
        fetchImplementation,
        basePath,
        "/api/validation/question-attempt-timing",
        decodeQuestionAttemptTimingDecision,
        {
          method: "POST",
          body: evaluation,
        },
      ),
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
