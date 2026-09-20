import type { AssessmentId } from "../../../generated/api/AssessmentId";
import type { AssessmentAttempt } from "../../../generated/api/AssessmentAttempt";
import type { CourseInstanceId } from "../../../generated/api/CourseInstanceId";
import type { QuestionAttemptId } from "../../../generated/api/QuestionAttemptId";
import type { ApiClient } from "../client";
import type {
  AssessmentEditorDetail,
  AssessmentContentInput,
  AssessmentCreateInput,
  StudentFeedbackReleaseResponse,
} from "../contracts";
import {
  decodeAssessmentContentInput,
  decodeAssessmentAttempt,
  decodeCapabilityViolations,
  decodeStudentFeedbackReleaseResponse,
  decodeStudentResponseFormatCheck,
  decodeQuestionAttemptTimingDecision,
} from "../decoders";
import { decodeAssessmentEditorDetail } from "../decoders/assessment_workspace";
import { ApiProtocolError, ApiRequestError, AssessmentConflictError } from "./error";
import {
  MAX_RESPONSE_CHARACTERS,
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

/** Converts the transport ETag into the exact Assessment edit precondition. */
function assessmentEditPrecondition(assessmentEditEtag: string): string {
  if (!validRevision(assessmentEditEtag))
    throw new ApiProtocolError("assessment edit number must be one positive strong numeric ETag");
  return assessmentEditEtag.slice(1, -1);
}
function assessmentPath(courseId: CourseInstanceId, assessmentId?: AssessmentId): string {
  const course = encodedId(courseId);
  return assessmentId === undefined
    ? `/api/courses/${course}/assessments`
    : `/api/courses/${course}/assessments/${encodedId(assessmentId)}`;
}

export function studentAttemptPath(
  courseId: CourseInstanceId,
  assessmentId: AssessmentId,
  attemptId: QuestionAttemptId,
): string {
  return `${assessmentPath(courseId, assessmentId)}/attempts/${encodedId(attemptId)}`;
}

export async function requestAssessmentEditor(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  expected: { readonly assessmentId?: AssessmentId; readonly courseId?: CourseInstanceId },
  options: RequestOptions = {},
): Promise<AssessmentEditorDetail> {
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
    throw new AssessmentConflictError(response.status, path);
  responseContentType(response, path);
  const text = await response.text();
  if (text.length === 0 || text.length > MAX_RESPONSE_CHARACTERS)
    throw new ApiProtocolError(`API response ${path} must contain a bounded JSON body`);
  const value = decodeJson(text, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  const detail = decodeAssessmentEditorDetail(value, "response");
  if (expected.assessmentId !== undefined && detail.id !== expected.assessmentId)
    throw new ApiProtocolError(
      "assessment editor response does not match the requested assessment",
    );
  if (expected.courseId !== undefined && detail.courseId !== expected.courseId)
    throw new ApiProtocolError("assessment editor response does not match the requested course");
  const etag = response.headers.get("etag");
  if (etag === null || !validRevision(etag))
    throw new ApiProtocolError(
      `API response ${path} must include one positive strong numeric ETag`,
    );
  return { ...detail, etag };
}

export function createRequestClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<
  ApiClient,
  | "createAssessment"
  | "getAssessmentWorkspace"
  | "saveAssessmentContent"
  | "startAssessmentAttempt"
  | "releaseStudentFeedback"
  | "validateResponseFormatOnServer"
  | "questionAttemptTimingDecisionOnServer"
  | "validateAssessmentConfigOnServer"
> {
  return {
    getAssessmentWorkspace: (courseId, assessmentId) =>
      requestAssessmentEditor(
        fetchImplementation,
        basePath,
        assessmentPath(courseId, assessmentId),
        { courseId, assessmentId },
      ),
    createAssessment: (
      courseId,
      input: AssessmentCreateInput,
    ): ReturnType<ApiClient["createAssessment"]> => {
      if (typeof input.title !== "string" || input.title.trim().length === 0)
        return Promise.reject(new ApiProtocolError("Assessment needs a nonempty title"));
      return requestAssessmentEditor(
        fetchImplementation,
        basePath,
        assessmentPath(courseId),
        { courseId },
        { method: "POST", body: { title: input.title } },
      );
    },
    saveAssessmentContent: (
      courseId,
      assessmentId,
      input: AssessmentContentInput,
      assessmentEtag,
    ): ReturnType<ApiClient["saveAssessmentContent"]> => {
      const baseEditNumber = assessmentEditPrecondition(assessmentEtag);
      return requestAssessmentEditor(
        fetchImplementation,
        basePath,
        `${assessmentPath(courseId, assessmentId)}/content`,
        { courseId, assessmentId },
        {
          method: "PUT",
          body: { ...decodeAssessmentContentInput(input, "request"), baseEditNumber },
          headers: { "if-match": assessmentEtag },
        },
      );
    },
    startAssessmentAttempt: (courseId, assessmentId): Promise<AssessmentAttempt> =>
      requestJson(
        fetchImplementation,
        basePath,
        `${assessmentPath(courseId, assessmentId)}/assessment-attempts`,
        decodeAssessmentAttempt,
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
    validateAssessmentConfigOnServer: (validationConfig) =>
      requestJson(
        fetchImplementation,
        basePath,
        "/api/validation/assessment-capabilities",
        decodeCapabilityViolations,
        { method: "POST", body: validationConfig },
      ),
  };
}
