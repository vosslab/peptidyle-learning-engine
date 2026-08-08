// http_client.ts - same-origin fetch transport for the Rust API.

import type { AssignmentId } from "../../generated/api/AssignmentId";
import type { AssignmentRun } from "../../generated/api/AssignmentRun";
import type { CourseId } from "../../generated/api/CourseId";
import type { CatalogProblemDetail } from "../../generated/api/CatalogProblemDetail";
import type { CatalogSearchPage } from "../../generated/api/CatalogSearchPage";
import type { CatalogSearchQuery } from "../../generated/api/CatalogSearchQuery";
import type { GradebookSummaryRow } from "../../generated/api/GradebookSummaryRow";
import type { EnrollmentId } from "../../generated/api/EnrollmentId";
import type { ProblemId } from "../../generated/api/ProblemId";
import type { QuestionAttempt } from "../../generated/api/QuestionAttempt";
import type { QuestionAttemptId } from "../../generated/api/QuestionAttemptId";
import type { RunId } from "../../generated/api/RunId";
import type { StudentResponse } from "../../generated/api/StudentResponse";
import type { VersionId } from "../../generated/api/VersionId";
import type { DraftQuestionDefinition } from "../../generated/api/DraftQuestionDefinition";
import type { WorkspaceId } from "../../generated/api/WorkspaceId";
import type { PublicationScope } from "../../generated/api/PublicationScope";
import type { ApiClient } from "./client";
import { catalogSearchPath } from "./catalog_query";
import type {
  AssignmentCapabilityViolation,
  AssignmentEditorDetail,
  AssignmentEditorInput,
  CursorPage,
  EnrollmentView,
  ExternalToolLaunch,
  PublicationValidationResponse,
  PublicationViolation,
  RunScreenData,
  RunSummaryResponse,
  PrefetchedNextQuestion,
} from "./contracts";
import type { Decoder } from "./decoder";
import {
  decodeAssignmentPage,
  decodeAssignmentCapabilityViolations,
  decodeAssignmentEditorDetail,
  decodeAssignmentEditorInput,
  decodeAssignmentRun,
  decodeAssignmentSummary,
  decodeAttemptPage,
  decodeAuthSession,
  decodeCapabilityViolations,
  decodeCatalogProblemDetail,
  decodeCatalogPage,
  decodeCatalogSearchPage,
  decodeCoursePage,
  decodeCourseSummary,
  decodeEnrollmentView,
  decodeFeedbackReleaseResponse,
  decodeExternalToolLaunch,
  decodeGradebookPage,
  decodeQuestionAttempt,
  decodeQuestionDefinition,
  decodeQuestionEnvelope,
  decodePrefetchedNextQuestion,
  decodeResponseFormatReport,
  decodeRunPage,
  decodeStudentResponse,
  decodeRunSummaryResponse,
  decodeStudentAssignmentSummary,
  decodeDraftQuestionDefinition,
  decodeWorkspaceDraftPage,
  decodePublicationDiff,
  decodePublicationReadinessFailure,
  decodePublicationResult,
  decodePublicationValidationFailure,
  decodePublicationValidationReport,
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

/** An optimistic workspace save lost its strong revision race; the editor must retain local edits. */
export class WorkspaceConflictError extends ApiRequestError {
  public constructor(status: 409 | 428, path: string) {
    super(status, path);
    this.name = "WorkspaceConflictError";
  }
}

/** A revisioned assignment save lost its server-side compare-and-swap race. */
export class AssignmentConflictError extends ApiRequestError {
  public constructor(status: 409 | 428, path: string) {
    super(status, path);
    this.name = "AssignmentConflictError";
  }
}

/** A complete, authoritative assignment capability report from a 422 response. */
export class AssignmentValidationError extends ApiRequestError {
  public readonly violations: ReadonlyArray<AssignmentCapabilityViolation>;

  public constructor(path: string, violations: ReadonlyArray<AssignmentCapabilityViolation>) {
    super(422, path);
    this.name = "AssignmentValidationError";
    this.violations = violations;
  }
}

/** A publish 422 that retains every server-reported capability violation. */
export class PublicationValidationError extends ApiRequestError {
  public readonly messageForAuthor: string;
  public readonly violations: ReadonlyArray<PublicationViolation>;

  public constructor(
    path: string,
    messageForAuthor: string,
    violations: ReadonlyArray<PublicationViolation>,
  ) {
    super(422, path);
    this.name = "PublicationValidationError";
    this.messageForAuthor = messageForAuthor;
    this.violations = violations;
  }
}

interface RequestOptions {
  readonly method?: "GET" | "POST" | "PUT" | "DELETE";
  readonly body?: unknown;
  readonly headers?: Readonly<Record<string, string>>;
}

interface ExpectedAssignmentIdentity {
  readonly assignmentId?: AssignmentId;
  readonly courseId?: CourseId;
}

function workspacePath(workspace: WorkspaceId): string {
  return `/api/workspaces/${encodedId(workspace)}`;
}

function workspaceRevision(response: Response, path: string): string {
  const revision = response.headers.get("etag");
  if (revision === null || !/^"[0-9]+"$/u.test(revision)) {
    throw new ApiProtocolError(`API response ${path} must include one strong numeric ETag`);
  }
  return revision;
}

function assignmentRevision(response: Response, path: string): string {
  const revision = response.headers.get("etag");
  if (revision === null || !validWorkspaceRevision(revision)) {
    throw new ApiProtocolError(
      `API response ${path} must include one positive strong numeric ETag`,
    );
  }
  return revision;
}

function validWorkspaceRevision(value: string): boolean {
  if (!/^"[1-9][0-9]*"$/u.test(value)) return false;
  return BigInt(value.slice(1, -1)) <= 9_223_372_036_854_775_807n;
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

function gradebookPath(
  courseId: CourseId,
  cursor: string | undefined,
  pageSize: number | undefined,
): string {
  if (pageSize !== undefined && (!Number.isSafeInteger(pageSize) || pageSize <= 0)) {
    throw new Error("gradebook pageSize must be a positive safe integer");
  }
  const query = new URLSearchParams();
  if (cursor !== undefined) {
    query.set("cursor", cursor);
  }
  if (pageSize !== undefined) {
    query.set("pageSize", String(pageSize));
  }
  const suffix = query.size === 0 ? "" : `?${query.toString()}`;
  return `/api/courses/${encodedId(courseId)}/gradebook${suffix}`;
}

function encodedId(value: string): string {
  return encodeURIComponent(value);
}

async function catalogProblemDetail(
  fetchImplementation: ApiFetch,
  basePath: string,
  problemId: ProblemId,
  versionId: VersionId,
): Promise<CatalogProblemDetail> {
  const path = `/api/problems/${encodedId(problemId)}/versions/${encodedId(versionId)}/detail`;
  const detail = await requestJson(fetchImplementation, basePath, path, decodeCatalogProblemDetail);
  if (detail.summary.problem !== problemId || detail.summary.version !== versionId) {
    throw new ApiProtocolError(
      "Catalog detail identity does not match its requested immutable version",
    );
  }
  return detail;
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

async function requestWorkspaceDraft(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  options: RequestOptions = {},
): Promise<{ readonly draft: DraftQuestionDefinition; readonly revision: string }> {
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
  if (response.status === 409 || response.status === 428) {
    throw new WorkspaceConflictError(response.status, path);
  }
  if (!response.ok) throw new ApiRequestError(response.status, path);
  responseContentType(response, path);
  const text = await response.text();
  if (text.length === 0 || text.length > MAX_RESPONSE_CHARACTERS) {
    throw new ApiProtocolError(
      `API response ${path} must contain 1 to ${MAX_RESPONSE_CHARACTERS} JSON characters`,
    );
  }
  return {
    draft: decodeDraftQuestionDefinition(decodeJson(text, path), "response"),
    revision: workspaceRevision(response, path),
  };
}

function assignmentPath(courseId: CourseId, assignmentId?: AssignmentId): string {
  const course = encodedId(courseId);
  return assignmentId === undefined
    ? `/api/courses/${course}/assignments`
    : `/api/courses/${course}/assignments/${encodedId(assignmentId)}`;
}

function assignmentValidationFailure(
  value: unknown,
  path: string,
): ReadonlyArray<AssignmentCapabilityViolation> | undefined {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return undefined;
  if (
    !("error" in value) ||
    Reflect.get(value, "error") !== "assignment configuration is not supported"
  ) {
    return undefined;
  }
  return decodeAssignmentCapabilityViolations(value, path);
}

async function requestAssignmentEditor(
  fetchImplementation: ApiFetch,
  basePath: string,
  path: string,
  expected: ExpectedAssignmentIdentity,
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
  if (response.status === 409 || response.status === 428) {
    throw new AssignmentConflictError(response.status, path);
  }
  responseContentType(response, path);
  const text = await response.text();
  if (text.length === 0 || text.length > MAX_RESPONSE_CHARACTERS) {
    throw new ApiProtocolError(`API response ${path} must contain a bounded JSON body`);
  }
  const value = decodeJson(text, path);
  if (response.status === 422) {
    const violations = assignmentValidationFailure(value, "response");
    if (violations !== undefined) throw new AssignmentValidationError(path, violations);
  }
  if (!response.ok) throw new ApiRequestError(response.status, path);
  const detail = decodeAssignmentEditorDetail(value, "response");
  if (expected.assignmentId !== undefined && detail.id !== expected.assignmentId) {
    throw new ApiProtocolError(
      "assignment editor response does not match the requested assignment",
    );
  }
  if (expected.courseId !== undefined && detail.courseId !== expected.courseId) {
    throw new ApiProtocolError("assignment editor response does not match the requested course");
  }
  return { ...detail, revision: assignmentRevision(response, path) };
}

async function requestPublicationValidation(
  fetchImplementation: ApiFetch,
  basePath: string,
  workspace: WorkspaceId,
): Promise<PublicationValidationResponse> {
  const path = `${workspacePath(workspace)}/publication-validation`;
  const response = await fetchImplementation(requestPath(basePath, path), {
    method: "POST",
    headers: { accept: "application/json" },
    credentials: "same-origin",
    cache: "no-store",
  });
  responseContentType(response, path);
  const text = await response.text();
  if (text.length === 0 || text.length > MAX_RESPONSE_CHARACTERS) {
    throw new ApiProtocolError(`API response ${path} must contain a bounded JSON body`);
  }
  const value = decodeJson(text, path);
  if (response.status === 422) {
    const failure = decodePublicationReadinessFailure(value, "response");
    return failure;
  }
  if (!response.ok) throw new ApiRequestError(response.status, path);
  const report = decodePublicationValidationReport(value, "response");
  return {
    kind: "capabilityReport",
    revision: workspaceRevision(response, path),
    violations: report.violations,
  };
}

async function requestPublicationDiff(
  fetchImplementation: ApiFetch,
  basePath: string,
  workspace: WorkspaceId,
): Promise<import("./contracts").PublicationDiff> {
  const path = `${workspacePath(workspace)}/publication-diff`;
  const response = await fetchImplementation(requestPath(basePath, path), {
    headers: { accept: "application/json" },
    credentials: "same-origin",
    cache: "no-store",
  });
  if (!response.ok) throw new ApiRequestError(response.status, path);
  responseContentType(response, path);
  const text = await response.text();
  const diff = decodePublicationDiff(decodeJson(text, path), "response");
  const revision = workspaceRevision(response, path);
  if (revision !== diff.revision) {
    throw new ApiProtocolError("Publication diff ETag does not match its draftRevision");
  }
  return diff;
}

async function requestPublication(
  fetchImplementation: ApiFetch,
  basePath: string,
  workspace: WorkspaceId,
  scope: PublicationScope,
  revision: string,
): Promise<import("./contracts").PublicationResult> {
  if (scope !== "institution" && scope !== "public") {
    throw new ApiProtocolError("publication scope must be institution or public");
  }
  if (!validWorkspaceRevision(revision)) {
    throw new ApiProtocolError("publication revision must be one positive strong numeric ETag");
  }
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
  if (text.length === 0 || text.length > MAX_RESPONSE_CHARACTERS) {
    throw new ApiProtocolError(`API response ${path} must contain a bounded JSON body`);
  }
  const value = decodeJson(text, path);
  if (response.status === 409 || response.status === 428) {
    throw new WorkspaceConflictError(response.status, path);
  }
  if (response.status === 422) {
    const failure = decodePublicationValidationFailure(value, "response");
    throw new PublicationValidationError(path, failure.message, failure.violations);
  }
  if (!response.ok) throw new ApiRequestError(response.status, path);
  return decodePublicationResult(value, "response");
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
    screen.issuedQuestion.version !== screen.attempt.questionVersion ||
    screen.issuedQuestion.seed !== screen.attempt.seed
  ) {
    throw new ApiProtocolError("Run screen issued question does not match its attempt");
  }
}

function verifyRunEnrollment(run: AssignmentRun, enrollment: EnrollmentView): void {
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
}

/** Creates a strict same-origin transport for the implemented Rust routes. */
export function createHttpApiClient(config: HttpApiClientConfig = {}): ApiClient {
  const fetchImplementation = config.fetch ?? browserFetch;
  const basePath = normalizeBasePath(config.basePath);

  const client: ApiClient = {
    getSession: () =>
      requestJson(fetchImplementation, basePath, "/api/auth/session", decodeAuthSession),
    listWorkspaceDrafts: (cursor?: string) =>
      requestJson(
        fetchImplementation,
        basePath,
        cursorPath("/api/workspaces", cursor),
        decodeWorkspaceDraftPage,
      ),
    getWorkspaceDraft: (workspace: WorkspaceId) =>
      requestWorkspaceDraft(fetchImplementation, basePath, workspacePath(workspace)),
    saveWorkspaceDraft: (
      workspace: WorkspaceId,
      draft: DraftQuestionDefinition,
      revision?: string,
    ) => {
      if (draft.workspace !== workspace) {
        return Promise.reject(
          new ApiProtocolError("workspace save path does not match draft body"),
        );
      }
      if (revision !== undefined && !/^"[0-9]+"$/u.test(revision)) {
        return Promise.reject(
          new ApiProtocolError("workspace revision must be one strong numeric ETag"),
        );
      }
      return requestWorkspaceDraft(fetchImplementation, basePath, workspacePath(workspace), {
        method: "PUT",
        body: draft,
        headers: revision === undefined ? {} : { "if-match": revision },
      });
    },
    deleteWorkspaceDraft: async (workspace: WorkspaceId, revision: string): Promise<void> => {
      const path = workspacePath(workspace);
      if (!/^"[0-9]+"$/u.test(revision)) {
        throw new ApiProtocolError("workspace revision must be one strong numeric ETag");
      }
      const response = await fetchImplementation(requestPath(basePath, path), {
        method: "DELETE",
        headers: { accept: "application/json", "if-match": revision },
        credentials: "same-origin",
        cache: "no-store",
      });
      if (response.status === 409 || response.status === 428) {
        throw new WorkspaceConflictError(response.status, path);
      }
      if (!response.ok) throw new ApiRequestError(response.status, path);
      if (response.status !== 204) throw new ApiProtocolError(`API response ${path} must be 204`);
    },
    validateWorkspacePublication: (workspace: WorkspaceId) =>
      requestPublicationValidation(fetchImplementation, basePath, workspace),
    getWorkspacePublicationDiff: (workspace: WorkspaceId) =>
      requestPublicationDiff(fetchImplementation, basePath, workspace),
    publishWorkspace: (workspace: WorkspaceId, scope: PublicationScope, revision: string) =>
      requestPublication(fetchImplementation, basePath, workspace, scope, revision),
    listProblems: (cursor?: string) =>
      requestJson(
        fetchImplementation,
        basePath,
        cursorPath("/api/problems", cursor),
        decodeCatalogPage,
      ),
    searchCatalog: (query: CatalogSearchQuery): Promise<CatalogSearchPage> =>
      requestJson(fetchImplementation, basePath, catalogSearchPath(query), decodeCatalogSearchPage),
    getCatalogProblemDetail: (problemId: ProblemId, versionId: VersionId) =>
      catalogProblemDetail(fetchImplementation, basePath, problemId, versionId),
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
    listGradebook: (
      courseId: CourseId,
      cursor?: string,
      pageSize?: number,
    ): Promise<CursorPage<GradebookSummaryRow>> =>
      requestJson(
        fetchImplementation,
        basePath,
        gradebookPath(courseId, cursor, pageSize),
        decodeGradebookPage,
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
    getAssignmentEditor: (assignmentId: AssignmentId) =>
      requestAssignmentEditor(
        fetchImplementation,
        basePath,
        `/api/assignments/${encodedId(assignmentId)}`,
        { assignmentId },
      ),
    createAssignment: (courseId: CourseId, input: AssignmentEditorInput) => {
      const body = decodeAssignmentEditorInput(input, "request");
      return requestAssignmentEditor(
        fetchImplementation,
        basePath,
        assignmentPath(courseId),
        { courseId },
        { method: "POST", body },
      );
    },
    saveAssignment: (
      courseId: CourseId,
      assignmentId: AssignmentId,
      input: AssignmentEditorInput,
      revision: string,
    ) => {
      if (!validWorkspaceRevision(revision)) {
        return Promise.reject(
          new ApiProtocolError("assignment revision must be one positive strong numeric ETag"),
        );
      }
      const body = decodeAssignmentEditorInput(input, "request");
      return requestAssignmentEditor(
        fetchImplementation,
        basePath,
        assignmentPath(courseId, assignmentId),
        { courseId, assignmentId },
        { method: "PUT", body, headers: { "if-match": revision } },
      );
    },
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
    getRunSummary: (
      runId: RunId,
      cursor?: string,
      pageSize?: number,
    ): Promise<RunSummaryResponse> => {
      if (cursor !== undefined && (cursor.length === 0 || cursor.length > 512)) {
        return Promise.reject(
          new ApiProtocolError("run summary cursor must be 1 through 512 characters"),
        );
      }
      if (
        pageSize !== undefined &&
        (!Number.isSafeInteger(pageSize) || pageSize <= 0 || pageSize > 100)
      ) {
        return Promise.reject(new ApiProtocolError("run summary pageSize must be 1 through 100"));
      }
      const query = new URLSearchParams();
      if (cursor !== undefined) query.set("cursor", cursor);
      if (pageSize !== undefined) query.set("pageSize", String(pageSize));
      const suffix = query.size === 0 ? "" : `?${query.toString()}`;
      return requestJson(
        fetchImplementation,
        basePath,
        `/api/runs/${encodedId(runId)}/summary${suffix}`,
        decodeRunSummaryResponse,
      );
    },
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
    getIssuedQuestion: (attemptId: QuestionAttemptId) =>
      requestJson(
        fetchImplementation,
        basePath,
        `/api/attempts/${encodedId(attemptId)}/question`,
        decodeQuestionEnvelope,
      ),
    prefetchNextQuestion: async (
      attemptId: QuestionAttemptId,
      signal?: AbortSignal,
    ): Promise<PrefetchedNextQuestion | null> => {
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
      responseContentType(response, path);
      const text = await response.text();
      if (text.length === 0 || text.length > MAX_RESPONSE_CHARACTERS) {
        throw new ApiProtocolError(
          `API response ${path} must contain 1 to ${MAX_RESPONSE_CHARACTERS} JSON characters`,
        );
      }
      const decoded = decodePrefetchedNextQuestion(decodeJson(text, path), "response");
      if (decoded.predecessor !== attemptId) {
        throw new ApiProtocolError("Prefetched question predecessor does not match its request");
      }
      return decoded;
    },
    getExternalToolLaunch: (attemptId: QuestionAttemptId): Promise<ExternalToolLaunch> =>
      requestJson(
        fetchImplementation,
        basePath,
        `/api/attempts/${encodedId(attemptId)}/external-tool-launch`,
        decodeExternalToolLaunch,
      ),
    submitResponse: async (
      attemptId: QuestionAttemptId,
      response: StudentResponse,
      idempotencyKey: string,
    ) => {
      const decodedResponse = decodeStudentResponse(response, "request.response");
      const path =
        decodedResponse.kind === "externalTool"
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
          body: { response: decodedResponse },
        },
      );
      if (receipt.attempt.id !== attemptId) {
        throw new ApiProtocolError("Submission receipt attempt does not match its request");
      }
      if (
        receipt.nextIssued !== null &&
        (receipt.nextIssued.run !== receipt.attempt.run ||
          receipt.nextIssued.id === receipt.attempt.id)
      ) {
        throw new ApiProtocolError("Submission receipt next attempt is not bound to its response");
      }
      return receipt;
    },
    releaseAttemptFeedback: (attemptId: QuestionAttemptId) =>
      requestJson(
        fetchImplementation,
        basePath,
        `/api/attempts/${encodedId(attemptId)}/feedback-release`,
        decodeFeedbackReleaseResponse,
        { method: "POST" },
      ),
    getSummary: (enrollmentId: EnrollmentId) =>
      requestJson(
        fetchImplementation,
        basePath,
        `/api/grading/summaries/${encodedId(enrollmentId)}`,
        decodeStudentAssignmentSummary,
      ),
    getRunScreen: async (runId: RunId): Promise<RunScreenData> => {
      const run = await client.getRun(runId);
      const [enrollment, initialAttempt] = await Promise.all([
        client.getEnrollment(run.enrollment),
        activeAttempt(client, runId).then(
          (attempt) => ({ kind: "attempt" as const, attempt }),
          (error: unknown) => ({ kind: "error" as const, error }),
        ),
      ]);
      verifyRunEnrollment(run, enrollment);
      let attempt: QuestionAttempt;
      if (initialAttempt.kind === "attempt") {
        attempt = initialAttempt.attempt;
      } else {
        const error = initialAttempt.error;
        const noActiveAttempt =
          error instanceof ApiProtocolError &&
          error.message === `Run ${runId} has no active question attempt`;
        if (!noActiveAttempt || run.completedAt !== null) throw error;
        // A crash can commit a submission before its successor is issued. Ask
        // the authoritative start/resume endpoint once, then require that it
        // healed this exact route rather than sending the learner elsewhere.
        const resumed = await client.startRun(enrollment.enrollment.assignment);
        if (
          resumed.id !== runId ||
          resumed.tenant !== run.tenant ||
          resumed.enrollment !== run.enrollment
        ) {
          throw new ApiProtocolError("Run screen recovery did not resume the requested run");
        }
        attempt = await activeAttempt(client, runId);
      }
      const assignment = await client.getAssignment(enrollment.enrollment.assignment);
      if (assignment.id !== enrollment.enrollment.assignment) {
        throw new ApiProtocolError("Run screen assignment does not match its enrollment");
      }
      const [course, issuedQuestion] = await Promise.all([
        client.getCourse(assignment.courseId),
        client.getIssuedQuestion(attempt.id),
      ]);
      const screen: RunScreenData = { course, assignment, run, attempt, issuedQuestion };
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
