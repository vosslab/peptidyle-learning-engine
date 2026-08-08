// http_client.ts - same-origin fetch transport for the Rust API.

import type { AssignmentId } from "../../generated/api/AssignmentId";
import type { CourseId } from "../../generated/api/CourseId";
import type { EnrollmentId } from "../../generated/api/EnrollmentId";
import type { ProblemId } from "../../generated/api/ProblemId";
import type { QuestionAttempt } from "../../generated/api/QuestionAttempt";
import type { QuestionAttemptId } from "../../generated/api/QuestionAttemptId";
import type { RunId } from "../../generated/api/RunId";
import type { StudentResponse } from "../../generated/api/StudentResponse";
import type { VersionId } from "../../generated/api/VersionId";
import type { ApiClient } from "./client";
import type { RunScreenData } from "./contracts";
import type { Decoder } from "./decoder";
import {
  decodeAssignmentPage,
  decodeAssignmentRun,
  decodeAssignmentSummary,
  decodeAttemptPage,
  decodeAuthSession,
  decodeCapabilityViolations,
  decodeCatalogPage,
  decodeCoursePage,
  decodeCourseSummary,
  decodeEnrollmentView,
  decodeQuestionAttempt,
  decodeQuestionDefinition,
  decodeResponseFormatReport,
  decodeRunPage,
  decodeStudentAssignmentSummary,
  decodeSubmissionReceipt,
  decodeTaxonomyPage,
  decodeTimerVerdict,
} from "./decoders";

const MAX_RESPONSE_CHARACTERS = 4 * 1_024 * 1_024;

/** Fetch-compatible dependency injected by tests or a non-browser host. */
export type ApiFetch = (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>;

/** Configuration that cannot redirect credentials to another origin. */
export interface HttpApiClientConfig {
  readonly fetch?: ApiFetch;
  /** Same-origin path prefix, such as `/ple`; an origin or protocol is rejected. */
  readonly basePath?: string;
}

/** Non-successful HTTP result without echoing a potentially sensitive body. */
export class ApiRequestError extends Error {
  public readonly status: number;
  public readonly path: string;

  public constructor(status: number, path: string) {
    super(`API request ${path} failed with status ${status}`);
    this.name = "ApiRequestError";
    this.status = status;
    this.path = path;
  }
}

/** Successful HTTP response that violated the browser-safe API contract. */
export class ApiProtocolError extends Error {
  public constructor(message: string) {
    super(message);
    this.name = "ApiProtocolError";
  }
}

interface RequestOptions {
  readonly method?: "GET" | "POST";
  readonly body?: unknown;
  readonly headers?: Readonly<Record<string, string>>;
}

function normalizeBasePath(value: string | undefined): string {
  if (value === undefined || value === "" || value === "/") {
    return "";
  }
  if (
    !value.startsWith("/") ||
    value.startsWith("//") ||
    value.includes("?") ||
    value.includes("#")
  ) {
    throw new Error("API basePath must be a same-origin path without query or fragment");
  }
  return value.replace(/\/+$/, "");
}

function browserFetch(input: RequestInfo | URL, init?: RequestInit): Promise<Response> {
  return globalThis.fetch(input, init);
}

function requestPath(basePath: string, path: string): string {
  return `${basePath}${path}`;
}

function cursorPath(path: string, cursor: string | undefined): string {
  if (cursor === undefined) {
    return path;
  }
  const query = new URLSearchParams({ cursor });
  return `${path}?${query.toString()}`;
}

function encodedId(value: string): string {
  return encodeURIComponent(value);
}

function decodeJson(text: string, path: string): unknown {
  try {
    return JSON.parse(text);
  } catch (error: unknown) {
    const detail = error instanceof Error ? error.message : "invalid JSON";
    throw new ApiProtocolError(`API response ${path} is not valid JSON: ${detail}`);
  }
}

function responseContentType(response: Response, path: string): void {
  const contentType = response.headers.get("content-type");
  if (contentType === null || !contentType.toLowerCase().includes("application/json")) {
    throw new ApiProtocolError(`API response ${path} must use application/json`);
  }
}

async function requestJson<T>(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  decoder: Decoder<T>,
  options: RequestOptions = {},
): Promise<T> {
  const headers: Record<string, string> = {
    accept: "application/json",
    ...options.headers,
  };
  let body: string | undefined;
  if (options.body !== undefined) {
    headers["content-type"] = "application/json";
    body = JSON.stringify(options.body);
  }
  const response = await fetchImplementation(requestPath(basePath, path), {
    method: options.method ?? "GET",
    headers,
    body,
    credentials: "same-origin",
    cache: "no-store",
  });
  if (!response.ok) {
    throw new ApiRequestError(response.status, path);
  }
  responseContentType(response, path);
  const text = await response.text();
  if (text.length === 0 || text.length > MAX_RESPONSE_CHARACTERS) {
    throw new ApiProtocolError(
      `API response ${path} must contain 1 to ${MAX_RESPONSE_CHARACTERS} JSON characters`,
    );
  }
  return decoder(decodeJson(text, path), "response");
}

async function activeAttempt(client: ApiClient, runId: RunId): Promise<QuestionAttempt> {
  let cursor: string | undefined;
  const seenCursors = new Set<string>();
  while (true) {
    const page = await client.listAttempts(runId, cursor);
    const active = page.items.find((attempt) => attempt.response === null);
    if (active !== undefined) {
      return active;
    }
    if (page.nextCursor === null) {
      throw new ApiProtocolError(`Run ${runId} has no active question attempt`);
    }
    if (seenCursors.has(page.nextCursor)) {
      throw new ApiProtocolError(`Run ${runId} repeated an attempt cursor`);
    }
    seenCursors.add(page.nextCursor);
    cursor = page.nextCursor;
  }
}

function verifyRunScreen(screen: RunScreenData): void {
  if (screen.run.id !== screen.attempt.run) {
    throw new ApiProtocolError("Run screen attempt does not belong to the requested run");
  }
  if (screen.assignment.courseId !== screen.course.id) {
    throw new ApiProtocolError("Run screen assignment does not belong to its course");
  }
  if (
    screen.question.problem !== screen.attempt.problem ||
    screen.question.version !== screen.attempt.questionVersion
  ) {
    throw new ApiProtocolError("Run screen question does not match its issued attempt");
  }
}

/** Creates a strict same-origin transport for the implemented Rust routes. */
export function createHttpApiClient(config: HttpApiClientConfig = {}): ApiClient {
  const fetchImplementation = config.fetch ?? browserFetch;
  const basePath = normalizeBasePath(config.basePath);

  const client: ApiClient = {
    getSession: () =>
      requestJson(fetchImplementation, basePath, "/api/auth/session", decodeAuthSession),
    listProblems: (cursor?: string) =>
      requestJson(
        fetchImplementation,
        basePath,
        cursorPath("/api/problems", cursor),
        decodeCatalogPage,
      ),
    getProblemVersion: (problemId: ProblemId, versionId: VersionId) =>
      requestJson(
        fetchImplementation,
        basePath,
        `/api/problems/${encodedId(problemId)}/versions/${encodedId(versionId)}`,
        decodeQuestionDefinition,
      ),
    listTaxonomy: (cursor?: string) =>
      requestJson(
        fetchImplementation,
        basePath,
        cursorPath("/api/taxonomy", cursor),
        decodeTaxonomyPage,
      ),
    listCourses: (cursor?: string) =>
      requestJson(
        fetchImplementation,
        basePath,
        cursorPath("/api/courses", cursor),
        decodeCoursePage,
      ),
    getCourse: (courseId: CourseId) =>
      requestJson(
        fetchImplementation,
        basePath,
        `/api/courses/${encodedId(courseId)}`,
        decodeCourseSummary,
      ),
    listAssignments: (courseId: CourseId, cursor?: string) =>
      requestJson(
        fetchImplementation,
        basePath,
        cursorPath(`/api/courses/${encodedId(courseId)}/assignments`, cursor),
        decodeAssignmentPage,
      ),
    getAssignment: (assignmentId: AssignmentId) =>
      requestJson(
        fetchImplementation,
        basePath,
        `/api/assignments/${encodedId(assignmentId)}`,
        decodeAssignmentSummary,
      ),
    getEnrollment: (enrollmentId: EnrollmentId) =>
      requestJson(
        fetchImplementation,
        basePath,
        `/api/enrollments/${encodedId(enrollmentId)}`,
        decodeEnrollmentView,
      ),
    listRuns: (enrollmentId: EnrollmentId, cursor?: string) =>
      requestJson(
        fetchImplementation,
        basePath,
        cursorPath(`/api/enrollments/${encodedId(enrollmentId)}/runs`, cursor),
        decodeRunPage,
      ),
    startRun: (assignmentId: AssignmentId) =>
      requestJson(fetchImplementation, basePath, "/api/runs", decodeAssignmentRun, {
        method: "POST",
        body: { assignmentId },
      }),
    getRun: (runId: RunId) =>
      requestJson(
        fetchImplementation,
        basePath,
        `/api/runs/${encodedId(runId)}`,
        decodeAssignmentRun,
      ),
    listAttempts: (runId: RunId, cursor?: string) =>
      requestJson(
        fetchImplementation,
        basePath,
        cursorPath(`/api/runs/${encodedId(runId)}/attempts`, cursor),
        decodeAttemptPage,
      ),
    getAttempt: (attemptId: QuestionAttemptId) =>
      requestJson(
        fetchImplementation,
        basePath,
        `/api/attempts/${encodedId(attemptId)}`,
        decodeQuestionAttempt,
      ),
    submitResponse: (
      attemptId: QuestionAttemptId,
      response: StudentResponse,
      idempotencyKey: string,
    ) =>
      requestJson(
        fetchImplementation,
        basePath,
        `/api/submissions/${encodedId(attemptId)}`,
        decodeSubmissionReceipt,
        {
          method: "POST",
          headers: { "idempotency-key": idempotencyKey },
          body: { response },
        },
      ),
    getSummary: (enrollmentId: EnrollmentId) =>
      requestJson(
        fetchImplementation,
        basePath,
        `/api/grading/summaries/${encodedId(enrollmentId)}`,
        decodeStudentAssignmentSummary,
      ),
    getRunScreen: async (runId: RunId): Promise<RunScreenData> => {
      const [run, attempt] = await Promise.all([
        client.getRun(runId),
        activeAttempt(client, runId),
      ]);
      const enrollment = await client.getEnrollment(run.enrollment);
      if (
        enrollment.enrollment.id !== run.enrollment ||
        enrollment.summary.enrollment !== enrollment.enrollment.id
      ) {
        throw new ApiProtocolError("Run screen enrollment records are inconsistent");
      }
      if (
        run.tenant !== enrollment.enrollment.tenant ||
        enrollment.summary.tenant !== enrollment.enrollment.tenant
      ) {
        throw new ApiProtocolError("Run screen enrollment records cross tenant boundaries");
      }
      const assignment = await client.getAssignment(enrollment.enrollment.assignment);
      if (assignment.id !== enrollment.enrollment.assignment) {
        throw new ApiProtocolError("Run screen assignment does not match its enrollment");
      }
      const [course, question] = await Promise.all([
        client.getCourse(assignment.courseId),
        client.getProblemVersion(attempt.problem, attempt.questionVersion),
      ]);
      const screen: RunScreenData = { course, assignment, run, attempt, question };
      if (attempt.tenant !== run.tenant) {
        throw new ApiProtocolError("Run screen attempt crosses a tenant boundary");
      }
      verifyRunScreen(screen);
      return screen;
    },
    assetUrl: (assetId) => requestPath(basePath, `/api/assets/${encodedId(assetId)}`),
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

  return client;
}
