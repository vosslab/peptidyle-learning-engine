import type { AssignmentId } from "../../../generated/api/AssignmentId";
import type { AssetId } from "../../../generated/api/AssetId";
import type { AssignmentRun } from "../../../generated/api/AssignmentRun";
import type { CatalogProblemDetail } from "../../../generated/api/CatalogProblemDetail";
import type { CatalogProblemSummary } from "../../../generated/api/CatalogProblemSummary";
import type { CatalogSearchPage } from "../../../generated/api/CatalogSearchPage";
import type { CatalogSearchQuery } from "../../../generated/api/CatalogSearchQuery";
import type { CourseAppearance } from "../../../generated/api/CourseAppearance";
import type { CourseId } from "../../../generated/api/CourseId";
import type { EnrollmentId } from "../../../generated/api/EnrollmentId";
import type { GradebookSummaryRow } from "../../../generated/api/GradebookSummaryRow";
import type { QuestionId } from "../../../generated/api/QuestionId";
import type { QuestionAttempt } from "../../../generated/api/QuestionAttempt";
import type { QuestionAttemptId } from "../../../generated/api/QuestionAttemptId";
import type { QuestionEnvelope } from "../../../generated/api/QuestionEnvelope";
import type { RunId } from "../../../generated/api/RunId";
import type { WorkspaceId } from "../../../generated/api/WorkspaceId";
import type { ApiClient } from "../client";
import { DecodeError, decodeNonemptyString, decodeRecord } from "../decoder";
import type {
  CursorPage,
  EnrollmentView,
  PublicationDiff,
  RunScreenData,
  RunSummaryResponse,
  WorkspaceDraftDetail,
} from "../contracts";
import { catalogProblemReferencePath, catalogSearchPath } from "../catalog_query";
import {
  decodeAssignmentPage,
  decodeAssignmentRun,
  decodeAssignmentSummaryWithTiming,
  decodeAttemptPage,
  decodeCatalogPage,
  decodeCatalogProblemDetail,
  decodeCatalogProblemSummary,
  decodeCatalogSearchPage,
  decodeCourseAppearance,
  decodeCoursePage,
  decodeCourseSummary,
  decodeDraftQuestionDefinition,
  decodeEnrollmentView,
  decodeExternalToolLaunch,
  decodeGradebookPage,
  decodeQuestionAttempt,
  decodeQuestionEnvelope,
  decodeIssuedPresentationEnvelope,
  decodeRunPage,
  decodeRunSummaryResponse,
  decodeStudentAssignmentSummary,
  decodeTaxonomyPage,
  decodeWorkspaceDraftPage,
  decodePublicationDiff,
  decodeNavigationResolution,
} from "../decoders";
import { ApiProtocolError, ApiRequestError } from "./error";
import {
  encodedId,
  cursorPath,
  requestAssignmentEditor,
  requestJson,
  requestPath,
  type ApiFetch,
} from "./request";

export const MAX_RESPONSE_CHARACTERS = 4 * 1_024 * 1_024;

function decodeProtectedAssetDelivery(value: unknown, path = "response"): string {
  const record = decodeRecord(value, path);
  const raw = decodeNonemptyString(record.url, `${path}.url`);
  let url: URL;
  try {
    url = new URL(raw);
  } catch (_error: unknown) {
    throw new DecodeError(`${path}.url`, "an absolute HTTP(S) URL");
  }
  if (
    (url.protocol !== "https:" && url.protocol !== "http:") ||
    url.username !== "" ||
    url.password !== ""
  )
    throw new DecodeError(`${path}.url`, "an absolute HTTP(S) URL");
  return url.href;
}

function issuedQuestionForAttempt(
  fetchImplementation: ApiFetch,
  basePath: string,
  attempt: QuestionAttempt,
): Promise<QuestionEnvelope> {
  const decoder =
    attempt.issuedCapability === "notApplicable"
      ? decodeQuestionEnvelope
      : decodeIssuedPresentationEnvelope;
  return requestJson(
    fetchImplementation,
    basePath,
    `/api/attempts/${encodedId(attempt.id)}/question`,
    decoder,
  );
}

export function requireNoStore(response: Response, path: string): void {
  const directives =
    response.headers
      .get("cache-control")
      ?.split(",")
      .map((directive) => directive.trim().toLowerCase()) ?? [];
  if (!directives.includes("no-store"))
    throw new ApiProtocolError(`API response ${path} must be no-store`);
}
export function decodeJson(text: string, path: string): unknown {
  try {
    return JSON.parse(text);
  } catch (error: unknown) {
    const detail = error instanceof Error ? error.message : "invalid JSON";
    throw new ApiProtocolError(`API response ${path} is not valid JSON: ${detail}`);
  }
}
export function responseContentType(response: Response, path: string): void {
  const contentType = response.headers.get("content-type");
  if (contentType === null || !contentType.toLowerCase().includes("application/json"))
    throw new ApiProtocolError(`API response ${path} must use application/json`);
}
export async function boundedResponseJson(response: Response, path: string): Promise<unknown> {
  responseContentType(response, path);
  const text = await response.text();
  if (text.length === 0 || text.length > MAX_RESPONSE_CHARACTERS)
    throw new ApiProtocolError(`API response ${path} must contain bounded JSON`);
  return decodeJson(text, path);
}

function workspacePath(workspace: WorkspaceId): string {
  return `/api/workspaces/${encodedId(workspace)}`;
}
function workspaceRevision(response: Response, path: string): string {
  const revision = response.headers.get("etag");
  if (revision === null || !/^"[0-9]+"$/u.test(revision))
    throw new ApiProtocolError(`API response ${path} must include one strong numeric ETag`);
  return revision;
}
function courseAppearancePath(courseId: CourseId): string {
  return `/api/courses/${encodedId(courseId)}/appearance`;
}
function strongAppearanceRevision(value: string): string {
  if (!/^[1-9][0-9]*$/u.test(value) || BigInt(value) > 9_223_372_036_854_775_807n)
    throw new ApiProtocolError("Course appearance needs a canonical positive revision");
  return `"${value}"`;
}
function gradebookPath(
  courseId: CourseId,
  cursor: string | undefined,
  pageSize: number | undefined,
): string {
  if (pageSize !== undefined && (!Number.isSafeInteger(pageSize) || pageSize <= 0))
    throw new Error("gradebook pageSize must be a positive safe integer");
  const query = new URLSearchParams();
  if (cursor !== undefined) query.set("cursor", cursor);
  if (pageSize !== undefined) query.set("pageSize", String(pageSize));
  const suffix = query.size === 0 ? "" : `?${query.toString()}`;
  return `/api/courses/${encodedId(courseId)}/gradebook${suffix}`;
}

async function workspaceDraft(
  fetchImplementation: ApiFetch,
  basePath: string,
  workspace: WorkspaceId,
): Promise<WorkspaceDraftDetail> {
  const path = workspacePath(workspace);
  const response = await fetchImplementation(requestPath(basePath, path), {
    headers: { accept: "application/json" },
    credentials: "same-origin",
    cache: "no-store",
  });
  if (!response.ok) throw new ApiRequestError(response.status, path);
  const value = await boundedResponseJson(response, path);
  return {
    draft: decodeDraftQuestionDefinition(value, "response"),
    revision: workspaceRevision(response, path),
  };
}
async function courseAppearance(
  fetchImplementation: ApiFetch,
  basePath: string,
  courseId: CourseId,
): Promise<CourseAppearance> {
  const path = courseAppearancePath(courseId);
  const response = await fetchImplementation(requestPath(basePath, path), {
    headers: { accept: "application/json" },
    credentials: "same-origin",
    cache: "no-store",
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  const appearance = decodeCourseAppearance(await boundedResponseJson(response, path));
  if (response.headers.get("etag") !== strongAppearanceRevision(appearance.revision))
    throw new ApiProtocolError(`API response ${path} ETag does not match its appearance revision`);
  return appearance;
}
async function catalogProblemDetail(
  fetchImplementation: ApiFetch,
  basePath: string,
  questionId: QuestionId,
): Promise<CatalogProblemDetail> {
  const path = `/api/problems/by-id/${encodedId(questionId)}/detail`;
  const detail = await requestJson(fetchImplementation, basePath, path, decodeCatalogProblemDetail);
  if (detail.summary.questionId !== questionId)
    throw new ApiProtocolError(
      "Catalog detail identity does not match its requested immutable version",
    );
  return detail;
}
async function publicationDiff(
  fetchImplementation: ApiFetch,
  basePath: string,
  workspace: WorkspaceId,
): Promise<PublicationDiff> {
  const path = `${workspacePath(workspace)}/publication-diff`;
  const response = await fetchImplementation(requestPath(basePath, path), {
    headers: { accept: "application/json" },
    credentials: "same-origin",
    cache: "no-store",
  });
  if (!response.ok) throw new ApiRequestError(response.status, path);
  const diff = decodePublicationDiff(await boundedResponseJson(response, path), "response");
  if (workspaceRevision(response, path) !== diff.revision)
    throw new ApiProtocolError("Publication diff ETag does not match its draftRevision");
  return diff;
}
async function activeAttempt(client: ApiClient, runId: RunId): Promise<QuestionAttempt> {
  let cursor: string | undefined;
  const seen = new Set<string>();
  while (true) {
    const page = await client.listAttempts(runId, cursor);
    const active = page.items.find((attempt) => attempt.response === null);
    if (active !== undefined) return active;
    if (page.nextCursor === null)
      throw new ApiProtocolError(`Run ${runId} has no active question attempt`);
    if (seen.has(page.nextCursor))
      throw new ApiProtocolError(`Run ${runId} repeated an attempt cursor`);
    seen.add(page.nextCursor);
    cursor = page.nextCursor;
  }
}
function verifyRunEnrollment(run: AssignmentRun, enrollment: EnrollmentView): void {
  if (
    enrollment.enrollment.id !== run.enrollment ||
    enrollment.summary.enrollment !== enrollment.enrollment.id
  )
    throw new ApiProtocolError("Run screen enrollment records are inconsistent");
  if (
    run.tenant !== enrollment.enrollment.tenant ||
    enrollment.summary.tenant !== enrollment.enrollment.tenant
  )
    throw new ApiProtocolError("Run screen enrollment records cross tenant boundaries");
}
function verifyRunScreen(screen: RunScreenData): void {
  if (screen.run.id !== screen.attempt.run)
    throw new ApiProtocolError("Run screen attempt does not belong to the requested run");
  if (screen.assignment.courseId !== screen.course.summary.id)
    throw new ApiProtocolError("Run screen assignment does not belong to its course");
  if (
    screen.issuedQuestion.version !== screen.attempt.questionVersion ||
    screen.issuedQuestion.seed !== screen.attempt.seed
  )
    throw new ApiProtocolError("Run screen issued question does not match its attempt");
}

export function createResponseClient(
  fetchImplementation: ApiFetch,
  basePath: string,
  getClient: () => ApiClient,
): Pick<
  ApiClient,
  | "listWorkspaceDrafts"
  | "resolveNavigation"
  | "getWorkspaceDraft"
  | "getWorkspacePublicationDiff"
  | "listProblems"
  | "searchCatalog"
  | "resolveCatalogProblem"
  | "getCatalogProblemDetail"
  | "listTaxonomy"
  | "listCourses"
  | "getCourse"
  | "getCourseAppearance"
  | "listGradebook"
  | "listAssignments"
  | "getAssignment"
  | "getAssignmentEditor"
  | "getAssignmentSummary"
  | "getEnrollment"
  | "listRuns"
  | "getRun"
  | "getRunSummary"
  | "listAttempts"
  | "getAttempt"
  | "getIssuedQuestion"
  | "beginExternalToolLaunch"
  | "getSummary"
  | "getRunScreen"
  | "issueProtectedAssetDelivery"
  | "assetUrl"
> {
  return {
    resolveNavigation: (reference) =>
      requestJson(
        fetchImplementation,
        basePath,
        `/api/navigation/${encodedId(reference)}`,
        decodeNavigationResolution,
      ),
    listWorkspaceDrafts: (cursor) =>
      requestJson(
        fetchImplementation,
        basePath,
        cursorPath("/api/workspaces", cursor),
        decodeWorkspaceDraftPage,
      ),
    getWorkspaceDraft: (workspace) => workspaceDraft(fetchImplementation, basePath, workspace),
    getWorkspacePublicationDiff: (workspace) =>
      publicationDiff(fetchImplementation, basePath, workspace),
    listProblems: (cursor) =>
      requestJson(
        fetchImplementation,
        basePath,
        cursorPath("/api/problems", cursor),
        decodeCatalogPage,
      ),
    searchCatalog: (query: CatalogSearchQuery): Promise<CatalogSearchPage> =>
      requestJson(fetchImplementation, basePath, catalogSearchPath(query), decodeCatalogSearchPage),
    resolveCatalogProblem: (displayReference: string): Promise<CatalogProblemSummary> => {
      const path = catalogProblemReferencePath(displayReference);
      return requestJson(fetchImplementation, basePath, path, (value, decoderPath) =>
        decodeCatalogProblemSummary(value, decoderPath, true),
      );
    },
    getCatalogProblemDetail: (questionId) =>
      catalogProblemDetail(fetchImplementation, basePath, questionId),
    listTaxonomy: (cursor) =>
      requestJson(
        fetchImplementation,
        basePath,
        cursorPath("/api/taxonomy", cursor),
        decodeTaxonomyPage,
      ),
    listCourses: (cursor) =>
      requestJson(
        fetchImplementation,
        basePath,
        cursorPath("/api/courses", cursor),
        decodeCoursePage,
      ),
    getCourse: (courseId) =>
      requestJson(
        fetchImplementation,
        basePath,
        `/api/courses/${encodedId(courseId)}`,
        decodeCourseSummary,
      ),
    getCourseAppearance: (courseId) => courseAppearance(fetchImplementation, basePath, courseId),
    listGradebook: (courseId, cursor, pageSize): Promise<CursorPage<GradebookSummaryRow>> =>
      requestJson(
        fetchImplementation,
        basePath,
        gradebookPath(courseId, cursor, pageSize),
        decodeGradebookPage,
      ),
    listAssignments: (courseId, cursor) =>
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
        decodeAssignmentSummaryWithTiming,
      ),
    getAssignmentEditor: (assignmentId) =>
      requestAssignmentEditor(
        fetchImplementation,
        basePath,
        `/api/assignments/${encodedId(assignmentId)}`,
        { assignmentId },
      ),
    getAssignmentSummary: (assignmentId: AssignmentId) =>
      requestJson(
        fetchImplementation,
        basePath,
        `/api/assignments/${encodedId(assignmentId)}/summary`,
        decodeStudentAssignmentSummary,
      ),
    getEnrollment: (enrollmentId: EnrollmentId) =>
      requestJson(
        fetchImplementation,
        basePath,
        `/api/enrollments/${encodedId(enrollmentId)}`,
        decodeEnrollmentView,
      ),
    listRuns: (enrollmentId, cursor) =>
      requestJson(
        fetchImplementation,
        basePath,
        cursorPath(`/api/enrollments/${encodedId(enrollmentId)}/runs`, cursor),
        decodeRunPage,
      ),
    getRun: (runId) =>
      requestJson(
        fetchImplementation,
        basePath,
        `/api/runs/${encodedId(runId)}`,
        decodeAssignmentRun,
      ),
    getRunSummary: (runId, cursor, pageSize): Promise<RunSummaryResponse> => {
      if (cursor !== undefined && (cursor.length === 0 || cursor.length > 512))
        return Promise.reject(
          new ApiProtocolError("run summary cursor must be 1 through 512 characters"),
        );
      if (
        pageSize !== undefined &&
        (!Number.isSafeInteger(pageSize) || pageSize <= 0 || pageSize > 100)
      )
        return Promise.reject(new ApiProtocolError("run summary pageSize must be 1 through 100"));
      const query = new URLSearchParams();
      if (cursor !== undefined) query.set("cursor", cursor);
      if (pageSize !== undefined) query.set("pageSize", String(pageSize));
      return requestJson(
        fetchImplementation,
        basePath,
        `/api/runs/${encodedId(runId)}/summary${query.size === 0 ? "" : `?${query.toString()}`}`,
        decodeRunSummaryResponse,
      );
    },
    listAttempts: (runId, cursor) =>
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
    getIssuedQuestion: async (attemptId): Promise<QuestionEnvelope> => {
      const attempt = await requestJson(
        fetchImplementation,
        basePath,
        `/api/attempts/${encodedId(attemptId)}`,
        decodeQuestionAttempt,
      );
      return issuedQuestionForAttempt(fetchImplementation, basePath, attempt);
    },
    beginExternalToolLaunch: (attemptId) =>
      requestJson(
        fetchImplementation,
        basePath,
        `/api/attempts/${encodedId(attemptId)}/external-tool/launch`,
        decodeExternalToolLaunch,
        { method: "POST" },
      ),
    getSummary: (enrollmentId) =>
      requestJson(
        fetchImplementation,
        basePath,
        `/api/grading/summaries/${encodedId(enrollmentId)}`,
        decodeStudentAssignmentSummary,
      ),
    getRunScreen: async (runId): Promise<RunScreenData> => {
      const client = getClient();
      const run = await client.getRun(runId);
      const [enrollment, initial] = await Promise.all([
        client.getEnrollment(run.enrollment),
        activeAttempt(client, runId).then(
          (attempt) => ({ kind: "attempt" as const, attempt }),
          (error: unknown) => ({ kind: "error" as const, error }),
        ),
      ]);
      verifyRunEnrollment(run, enrollment);
      let attempt: QuestionAttempt;
      if (initial.kind === "attempt") attempt = initial.attempt;
      else {
        const noActive =
          initial.error instanceof ApiProtocolError &&
          initial.error.message === `Run ${runId} has no active question attempt`;
        if (!noActive || run.completedAt !== null) throw initial.error;
        const resumed = await client.startRun(enrollment.enrollment.assignment);
        if (
          resumed.id !== runId ||
          resumed.tenant !== run.tenant ||
          resumed.enrollment !== run.enrollment
        )
          throw new ApiProtocolError("Run screen recovery did not resume the requested run");
        attempt = await activeAttempt(client, runId);
      }
      const assignment = await client.getAssignment(enrollment.enrollment.assignment);
      if (assignment.id !== enrollment.enrollment.assignment)
        throw new ApiProtocolError("Run screen assignment does not match its enrollment");
      const [summary, appearance, issuedQuestion] = await Promise.all([
        client.getCourse(assignment.courseId),
        client.getCourseAppearance(assignment.courseId),
        issuedQuestionForAttempt(fetchImplementation, basePath, attempt),
      ]);
      const screen: RunScreenData = {
        course: { summary, appearance },
        assignment,
        run,
        attempt,
        issuedQuestion,
      };
      if (attempt.tenant !== run.tenant)
        throw new ApiProtocolError("Run screen attempt crosses a tenant boundary");
      verifyRunScreen(screen);
      return screen;
    },
    issueProtectedAssetDelivery: (assetId: AssetId) =>
      requestJson(
        fetchImplementation,
        basePath,
        `/api/assets/${encodedId(assetId)}/delivery`,
        decodeProtectedAssetDelivery,
        { method: "POST" },
      ),
    assetUrl: (assetId) => requestPath(basePath, `/api/assets/${encodedId(assetId)}`),
  };
}
