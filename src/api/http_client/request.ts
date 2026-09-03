import type { AssignmentId } from "../../../generated/api/AssignmentId";
import type { AssignmentReference } from "../../../generated/api/AssignmentReference";
import type { AssignmentAttempt } from "../../../generated/api/AssignmentAttempt";
import type { CourseGradeSchemeUpdateView } from "../../../generated/api/CourseGradeSchemeUpdateView";
import type { CourseId } from "../../../generated/api/CourseId";
import type { CourseSummary } from "../../../generated/api/CourseSummary";
import type { QuestionAttemptId } from "../../../generated/api/QuestionAttemptId";
import type { StudentResponse } from "../../../generated/api/StudentResponse";
import type { ApiClient } from "../client";
import type {
  AssignmentEditorDetail,
  AssignmentContentInput,
  AssignmentCreateInput,
  AssignmentPoliciesInput,
  StudentFeedbackReleaseResponse,
  InstructorStudentView,
  PrefetchedNextQuestion,
  QuestionSubmissionAcknowledgement,
} from "../contracts";
import {
  decodeAssignmentContentInput,
  decodeInstructorStudentView,
  decodeAssignmentPoliciesValidationFailure,
  decodeAssignmentAttempt,
  decodeCapabilityViolations,
  decodeCourseGradeSchemeView,
  decodeCourseGradeSchemeUpdateView,
  decodeCourseCreateInput,
  decodeCourseSummary,
  decodeCourseTermValidationFailure,
  decodeStudentFeedbackReleaseResponse,
  decodePrefetchedNextQuestion,
  decodeStudentResponseFormatCheck,
  decodeStudentResponse,
  decodeQuestionSubmissionAcknowledgement,
  decodeQuestionAttemptTimingDecision,
} from "../decoders";
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
  CourseGradeSchemeConflictError,
  CourseTermValidationError,
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

/** Policies owns the aggregate 422 validation response; Questions retain their separate save contract. */
async function requestAssignmentPolicies(
  fetchImplementation: ApiFetch,
  basePath: string,
  courseId: CourseId,
  assignmentId: AssignmentId,
  _assignmentReference: AssignmentReference,
  input: AssignmentPoliciesInput,
  assignmentRevisionEtag: string,
): Promise<AssignmentEditorDetail> {
  const path = `${assignmentPath(courseId, assignmentId)}/policies`;
  const baseEditNumber = assignmentEditPrecondition(assignmentRevisionEtag);
  const response = await requestSameOrigin(fetchImplementation, basePath, path, {
    method: "PUT",
    body: { ...input, baseEditNumber },
    headers: { "if-match": assignmentRevisionEtag },
  });
  if (response.status === 409 || response.status === 412 || response.status === 428)
    throw new AssignmentConflictError(response.status, path);
  if (response.status === 422) {
    const value = await boundedResponseJson(response, path);
    try {
      const validationFailure = decodeAssignmentPoliciesValidationFailure(value, "response");
      throw new AssignmentPoliciesValidationError(path, validationFailure.issues);
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
  | "saveCourseGradeScheme"
  | "createCourseGradeExport"
  | "createCourse"
  | "createAssignment"
  | "getAssignmentWorkspace"
  | "saveAssignmentContent"
  | "saveAssignmentPolicies"
  | "getInstructorStudentView"
  | "startAssignmentAttempt"
  | "prefetchNextQuestion"
  | "submitResponse"
  | "getSubmissionStatus"
  | "releaseStudentFeedback"
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
    createCourse: (input): Promise<CourseSummary> =>
      requestCourseCreate(fetchImplementation, basePath, decodeCourseCreateInput(input, "request")),
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
      assignmentRevisionEtag,
    ): ReturnType<ApiClient["saveAssignmentContent"]> => {
      const baseEditNumber = assignmentEditPrecondition(assignmentRevisionEtag);
      return requestAssignmentEditor(
        fetchImplementation,
        basePath,
        `${assignmentPath(courseId, assignmentId)}/content`,
        { courseId, assignmentId },
        {
          method: "PUT",
          body: { ...decodeAssignmentContentInput(input, "request"), baseEditNumber },
          headers: { "if-match": assignmentRevisionEtag },
        },
        "contentSave",
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
        decoded.kind === "imathasQuestionBackend"
          ? `${studentAttemptPath(courseId, assignmentId, attemptId)}/imathas-question-backend/launch/submission`
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
