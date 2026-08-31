import type { AssignmentId } from "../../../generated/api/AssignmentId";
import type { AssignmentEntryId } from "../../../generated/api/AssignmentEntryId";
import type { AssignmentReference } from "../../../generated/api/AssignmentReference";
import type { AssignmentRevisionReference } from "../../../generated/api/AssignmentRevisionReference";
import type { AssignmentAttempt } from "../../../generated/api/AssignmentAttempt";
import type { CourseAppearance } from "../../../generated/api/CourseAppearance";
import type { CourseAppearanceUpdate } from "../../../generated/api/CourseAppearanceUpdate";
import type { CourseGradeSchemeUpdateView } from "../../../generated/api/CourseGradeSchemeUpdateView";
import type { CourseBannerCandidateReceipt } from "../../../generated/api/CourseBannerCandidateReceipt";
import type { CourseId } from "../../../generated/api/CourseId";
import type { QuestionId } from "../../../generated/api/QuestionId";
import type { CourseSummary } from "../../../generated/api/CourseSummary";
import type { QuestionAttemptId } from "../../../generated/api/QuestionAttemptId";
import type { StudentResponse } from "../../../generated/api/StudentResponse";
import type { WorkspaceId } from "../../../generated/api/WorkspaceId";
import type { ApiClient } from "../client";
import { isPublicByline } from "../public_byline";
import type {
  AssignmentEditorDetail,
  AssignmentContentInput,
  AssignmentDraftInput,
  AssignmentPoliciesInput,
  ReplaceAssignmentFixedItemInput,
  FeedbackReleaseResponse,
  InstructorStudentView,
  PublicationResult,
  PublicationRequest,
  PublicationValidationResponse,
  PrefetchedNextQuestion,
  QuestionSubmissionAcknowledgement,
  WorkspaceDraftDetail,
} from "../contracts";
import {
  decodeAssignmentContentInput,
  decodeInstructorStudentView,
  decodeAssignmentPoliciesValidationFailure,
  decodeAssignmentAttempt,
  decodeCapabilityViolations,
  decodeCourseAppearance,
  decodeCourseGradeSchemeView,
  decodeCourseGradeSchemeUpdateView,
  decodeCourseBannerCandidateReceipt,
  decodeCourseCreateInput,
  decodeCourseSummary,
  decodeCourseTermValidationFailure,
  decodeDraftQuestionDefinition,
  decodeFeedbackReleaseResponse,
  decodePrefetchedNextQuestion,
  decodePublicationReadinessFailure,
  decodePublicationResult,
  decodePublicationValidationFailure,
  decodePublicationValidationReport,
  decodeStudentResponseFormatCheck,
  decodeStudentResponse,
  decodeQuestionSubmissionAcknowledgement,
  decodeQuestionAttemptTimingDecision,
} from "../decoders";
import { decodeQuestionId } from "../decoders/shared";
import { decodeAssignmentReference } from "../decoders/question_library";
import {
  decodeAssignmentEditorDetail,
  decodeSuccessorAssignmentRevisionRequired,
} from "../decoders/assignment_workspace";
import {
  ApiProtocolError,
  ApiRequestError,
  AssignmentConflictError,
  AssignmentSuccessorRevisionRequiredError,
  AssignmentPoliciesValidationError,
  CourseAppearanceConflictError,
  CourseAppearanceFileError,
  CourseGradeSchemeConflictError,
  CourseTermValidationError,
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

/**
 * Canonical same-origin transport dispatch for every browser API operation.
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

async function requestCourseCreate(
  fetchImplementation: ApiFetch,
  basePath: string,
  input: import("../contracts").CourseCreateInput,
): Promise<CourseSummary> {
  const path = "/api/courses";
  const response = await fetchImplementation(requestPath(basePath, path), {
    method: "POST",
    headers: { accept: "application/json", "content-type": "application/json" },
    body: JSON.stringify(input),
    credentials: "same-origin",
    cache: "no-store",
  });
  if (response.status === 422) {
    const value = await boundedResponseJson(response, path);
    let failure: import("../../../generated/api/CourseTermValidationFailure").CourseTermValidationFailure;
    try {
      failure = decodeCourseTermValidationFailure(value, "response");
    } catch (_error: unknown) {
      throw new ApiRequestError(response.status, path);
    }
    throw new CourseTermValidationError(path, failure);
  }
  if (!response.ok) throw new ApiRequestError(response.status, path);
  return decodeCourseSummary(await boundedResponseJson(response, path), "response");
}

function workspacePath(workspace: WorkspaceId): string {
  return `/api/workspaces/${encodedId(workspace)}`;
}
function validRevision(value: string): boolean {
  return /^"[1-9][0-9]*"$/u.test(value) && BigInt(value.slice(1, -1)) <= 9_223_372_036_854_775_807n;
}

/** Converts the transport ETag into the exact immutable revision the Instructor reviewed. */
function assignmentRevisionPrecondition(
  assignment: AssignmentReference,
  assignmentRevisionEtag: string,
): AssignmentRevisionReference {
  if (!validRevision(assignmentRevisionEtag))
    throw new ApiProtocolError("assignment revision must be one positive strong numeric ETag");
  return {
    assignment: decodeAssignmentReference(assignment, "request.baseRevision.assignment"),
    revision_number: assignmentRevisionEtag.slice(1, -1),
  };
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

function assignmentDraftPath(courseId: CourseId): string {
  return `${assignmentPath(courseId)}/drafts`;
}

export function studentAttemptPath(
  courseId: CourseId,
  assignmentId: AssignmentId,
  attemptId: QuestionAttemptId,
): string {
  return `${assignmentPath(courseId, assignmentId)}/attempts/${encodedId(attemptId)}`;
}

function verifyQuestionSubmissionAcknowledgement(
  status: QuestionSubmissionAcknowledgement,
  attemptId: QuestionAttemptId,
): QuestionSubmissionAcknowledgement {
  const returnedAttemptId = status.receipt.attemptId;
  if (returnedAttemptId !== attemptId)
    throw new ApiProtocolError("Submission status attempt does not match its request");
  if (status.gradingState !== "graded") return status;
  if (
    status.receipt.nextIssued !== null &&
    status.receipt.nextIssued.id === status.receipt.attempt.id
  )
    throw new ApiProtocolError("Submission receipt next attempt is not bound to its response");
  if (status.receipt.nextPending && status.receipt.nextIssued !== null)
    throw new ApiProtocolError("Submission receipt cannot issue and defer the same successor");
  return status;
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
  conflict: "standard" | "contentSave" = "standard",
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
  if (response.status === 409 && conflict === "contentSave") {
    // Only the generated successor-revision body gets semantic recovery; other 409s stay generic.
    const value = await boundedResponseJson(response, path);
    try {
      const requirement = decodeSuccessorAssignmentRevisionRequired(value, "response");
      throw new AssignmentSuccessorRevisionRequiredError(path, requirement);
    } catch (error: unknown) {
      if (error instanceof AssignmentSuccessorRevisionRequiredError) throw error;
      throw new ApiRequestError(response.status, path);
    }
  }
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

/** Policies owns the aggregate 422 envelope; Questions retain their separate save contract. */
async function requestAssignmentPolicies(
  fetchImplementation: ApiFetch,
  basePath: string,
  courseId: CourseId,
  assignmentId: AssignmentId,
  assignmentReference: AssignmentReference,
  input: AssignmentPoliciesInput,
  assignmentRevisionEtag: string,
): Promise<AssignmentEditorDetail> {
  const path = `${assignmentPath(courseId, assignmentId)}/policies`;
  const baseRevision = assignmentRevisionPrecondition(assignmentReference, assignmentRevisionEtag);
  const response = await requestSameOrigin(fetchImplementation, basePath, path, {
    method: "PUT",
    body: { ...input, baseRevision },
    headers: { "if-match": assignmentRevisionEtag },
  });
  if (response.status === 409 || response.status === 412 || response.status === 428)
    throw new AssignmentConflictError(response.status, path);
  if (response.status === 422) {
    const value = await boundedResponseJson(response, path);
    try {
      const failure = decodeAssignmentPoliciesValidationFailure(value, "response");
      throw new AssignmentPoliciesValidationError(path, failure.issues);
    } catch (error: unknown) {
      if (error instanceof AssignmentPoliciesValidationError) throw error;
      throw new ApiRequestError(response.status, path);
    }
  }
  if (!response.ok) throw new ApiRequestError(response.status, path);
  const value = await boundedResponseJson(response, path);
  const detail = decodeAssignmentEditorDetail(value, "response");
  if (detail.id !== assignmentId) {
    throw new ApiProtocolError(
      "assignment policies response does not match the requested assignment",
    );
  }
  if (detail.courseId !== courseId) {
    throw new ApiProtocolError("assignment policies response does not match the requested course");
  }
  const revisionHeader = response.headers.get("etag");
  if (revisionHeader === null || !validRevision(revisionHeader)) {
    throw new ApiProtocolError(
      `API response ${path} must include one positive strong numeric ETag`,
    );
  }
  return { ...detail, revision: revisionHeader };
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
  | "saveCourseGradeScheme"
  | "createCourseGradeExport"
  | "createCourse"
  | "createAssignmentDraft"
  | "getAssignmentWorkspace"
  | "saveAssignmentContent"
  | "replaceAssignmentFixedItem"
  | "saveAssignmentPolicies"
  | "getInstructorStudentView"
  | "startAssignmentAttempt"
  | "prefetchNextQuestion"
  | "submitResponse"
  | "getSubmissionStatus"
  | "releaseAttemptFeedback"
  | "validateResponseFormatOnServer"
  | "questionAttemptTimingDecisionOnServer"
  | "validateAssignmentConfigOnServer"
> {
  return {
    saveCourseGradeScheme: async (
      courseId,
      update: CourseGradeSchemeUpdateView,
      revision: string,
    ): ReturnType<ApiClient["saveCourseGradeScheme"]> => {
      if (!validRevision(revision))
        throw new ApiProtocolError("course grade scheme needs one positive strong revision");
      const body = decodeCourseGradeSchemeUpdateView(update, "request");
      const path = `/api/courses/${encodedId(courseId)}/grade-scheme`;
      const response = await fetchImplementation(requestPath(basePath, path), {
        method: "PUT",
        headers: {
          accept: "application/json",
          "content-type": "application/json",
          "if-match": revision,
        },
        body: JSON.stringify(body),
        credentials: "same-origin",
        cache: "no-store",
      });
      requireNoStore(response, path);
      if (response.status === 412) throw new CourseGradeSchemeConflictError(path);
      if (!response.ok) throw new ApiRequestError(response.status, path);
      const scheme = decodeCourseGradeSchemeView(await boundedResponseJson(response, path));
      const nextRevision = response.headers.get("etag");
      if (nextRevision === null || !validRevision(nextRevision))
        throw new ApiProtocolError(
          `API response ${path} must include one positive strong numeric ETag`,
        );
      return { ...scheme, revision: nextRevision };
    },
    createCourseGradeExport: async (courseId): ReturnType<ApiClient["createCourseGradeExport"]> => {
      const path = `/api/courses/${encodedId(courseId)}/grade-export.csv`;
      const response = await fetchImplementation(requestPath(basePath, path), {
        method: "POST",
        headers: { accept: "text/csv" },
        credentials: "same-origin",
        cache: "no-store",
      });
      requireNoStore(response, path);
      if (!response.ok) throw new ApiRequestError(response.status, path);
      if (
        response.headers.get("content-type")?.split(";", 1)[0]?.trim().toLowerCase() !== "text/csv"
      )
        throw new ApiProtocolError(`API response ${path} must use text/csv`);
      const exportId = response.headers.get("x-ple-course-grade-export-id");
      const filename = response.headers
        .get("content-disposition")
        ?.match(/^attachment; filename=([A-Za-z0-9._-]+)$/u)?.[1];
      if (
        exportId === null ||
        !/^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/iu.test(exportId) ||
        filename === undefined
      )
        throw new ApiProtocolError(
          `API response ${path} must include a safe export identity and filename`,
        );
      const csv = await response.blob();
      if (csv.size > 4 * 1_024 * 1_024)
        throw new ApiProtocolError(`API response ${path} exceeds the course export limit`);
      return { exportId, filename, csv };
    },
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
      request: PublicationRequest,
      revision: string,
    ): Promise<PublicationResult> => {
      if (!isPublicByline(request.byline))
        throw new ApiProtocolError("publication requires one to sixteen reviewed author names");
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
        body: JSON.stringify(request),
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
    createCourse: (input): Promise<CourseSummary> =>
      requestCourseCreate(fetchImplementation, basePath, decodeCourseCreateInput(input, "request")),
    getAssignmentWorkspace: (courseId, assignmentId) =>
      requestAssignmentEditor(
        fetchImplementation,
        basePath,
        assignmentPath(courseId, assignmentId),
        { courseId, assignmentId },
      ),
    createAssignmentDraft: (
      courseId,
      input: AssignmentDraftInput,
    ): ReturnType<ApiClient["createAssignmentDraft"]> => {
      if (typeof input.title !== "string" || input.title.trim().length === 0)
        return Promise.reject(new ApiProtocolError("assignment draft needs a nonempty title"));
      return requestAssignmentEditor(
        fetchImplementation,
        basePath,
        assignmentDraftPath(courseId),
        { courseId },
        { method: "POST", body: { title: input.title } },
      );
    },
    saveAssignmentContent: (
      courseId,
      assignmentId,
      assignmentReference,
      input: AssignmentContentInput,
      assignmentRevisionEtag,
    ): ReturnType<ApiClient["saveAssignmentContent"]> => {
      const baseRevision = assignmentRevisionPrecondition(
        assignmentReference,
        assignmentRevisionEtag,
      );
      return requestAssignmentEditor(
        fetchImplementation,
        basePath,
        `${assignmentPath(courseId, assignmentId)}/content`,
        { courseId, assignmentId },
        {
          method: "PUT",
          body: { ...decodeAssignmentContentInput(input, "request"), baseRevision },
          headers: { "if-match": assignmentRevisionEtag },
        },
        "contentSave",
      );
    },
    replaceAssignmentFixedItem: (
      courseId,
      assignmentId,
      assignmentReference,
      itemId: AssignmentEntryId,
      questionId: QuestionId,
      assignmentRevisionEtag,
    ): ReturnType<ApiClient["replaceAssignmentFixedItem"]> => {
      const baseRevision = assignmentRevisionPrecondition(
        assignmentReference,
        assignmentRevisionEtag,
      );
      if (itemId.length === 0)
        return Promise.reject(
          new ApiProtocolError("assignment fixed-item identity must be present"),
        );
      const input: ReplaceAssignmentFixedItemInput = {
        questionId: decodeQuestionId(questionId, "request.questionId"),
      };
      return requestAssignmentEditor(
        fetchImplementation,
        basePath,
        `${assignmentPath(courseId, assignmentId)}/fixed-items/${encodedId(itemId)}`,
        { courseId, assignmentId },
        {
          method: "PUT",
          body: { ...input, baseRevision },
          headers: { "if-match": assignmentRevisionEtag },
        },
      );
    },
    saveAssignmentPolicies: (
      courseId,
      assignmentId,
      assignmentReference,
      input: AssignmentPoliciesInput,
      assignmentRevisionEtag,
    ): ReturnType<ApiClient["saveAssignmentPolicies"]> => {
      return requestAssignmentPolicies(
        fetchImplementation,
        basePath,
        courseId,
        assignmentId,
        assignmentReference,
        input,
        assignmentRevisionEtag,
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
    prefetchNextQuestion: async (
      courseId,
      assignmentId,
      attemptId,
      signal,
    ): Promise<PrefetchedNextQuestion | null> => {
      const path = `${studentAttemptPath(courseId, assignmentId, attemptId)}/prefetch-next`;
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
      courseId: CourseId,
      assignmentId: AssignmentId,
      attemptId: QuestionAttemptId,
      response: StudentResponse,
      idempotencyKey: string,
    ): ReturnType<ApiClient["submitResponse"]> => {
      const decoded = decodeStudentResponse(response, "request.response");
      const path =
        decoded.kind === "externalTool"
          ? `${studentAttemptPath(courseId, assignmentId, attemptId)}/external-tool/launch/submission`
          : `${studentAttemptPath(courseId, assignmentId, attemptId)}/submissions`;
      const status = await requestJson(
        fetchImplementation,
        basePath,
        path,
        decodeQuestionSubmissionAcknowledgement,
        {
          method: "POST",
          headers: { "idempotency-key": idempotencyKey },
          body: { response: decoded },
        },
      );
      return verifyQuestionSubmissionAcknowledgement(status, attemptId);
    },
    getSubmissionStatus: async (
      courseId,
      assignmentId,
      attemptId,
    ): ReturnType<ApiClient["getSubmissionStatus"]> => {
      const path = `${studentAttemptPath(courseId, assignmentId, attemptId)}/submission-status`;
      const status = await requestJson(
        fetchImplementation,
        basePath,
        path,
        decodeQuestionSubmissionAcknowledgement,
      );
      return verifyQuestionSubmissionAcknowledgement(status, attemptId);
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
        decodeStudentResponseFormatCheck,
        { method: "POST", body: { definition, response } },
      ),
    questionAttemptTimingDecisionOnServer: (evaluation) =>
      requestJson(fetchImplementation, basePath, "/api/validation/question-attempt-timing", decodeQuestionAttemptTimingDecision, {
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
