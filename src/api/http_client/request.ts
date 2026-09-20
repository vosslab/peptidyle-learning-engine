import type { AssessmentId } from "../../../generated/api/AssessmentId";
import type { CourseInstanceId } from "../../../generated/api/CourseInstanceId";
import type { QuestionAttemptId } from "../../../generated/api/QuestionAttemptId";
import type { ApiClient } from "../client";
import type { StudentFeedbackReleaseResponse } from "../contracts";
import {
  decodeCapabilityViolations,
  decodeStudentFeedbackReleaseResponse,
  decodeStudentResponseFormatCheck,
  decodeQuestionAttemptTimingDecision,
} from "../decoders";
import { ApiProtocolError, ApiRequestError } from "./error";
import { MAX_RESPONSE_CHARACTERS, decodeJson, responseContentType } from "./response";

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

export function studentAttemptPath(
  courseId: CourseInstanceId,
  assessmentId: AssessmentId,
  attemptId: QuestionAttemptId,
): string {
  return `/api/course-instances/${encodedId(courseId)}/assessments/${encodedId(assessmentId)}/attempts/${encodedId(attemptId)}`;
}

export function createRequestClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<
  ApiClient,
  | "releaseStudentFeedback"
  | "validateResponseFormatOnServer"
  | "questionAttemptTimingDecisionOnServer"
  | "validateAssessmentConfigOnServer"
> {
  return {
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
