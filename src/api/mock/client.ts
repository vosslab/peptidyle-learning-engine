// client.ts - typed, server-free API client backed by the WP-C7 handlers.

import { publishedProblemFixture } from "../../../generated/fixtures/published_problem";
import type { AssignmentId } from "../../../generated/api/AssignmentId";
import type { CourseId } from "../../../generated/api/CourseId";
import type { CourseSummary } from "../../../generated/api/CourseSummary";
import type { CourseAppearance } from "../../../generated/api/CourseAppearance";
import type { CourseAppearanceUpdate } from "../../../generated/api/CourseAppearanceUpdate";
import type { CourseBannerCandidateReceipt } from "../../../generated/api/CourseBannerCandidateReceipt";
import type { CatalogProblemDetail } from "../../../generated/api/CatalogProblemDetail";
import type { CatalogSearchPage } from "../../../generated/api/CatalogSearchPage";
import type { GradebookSummaryRow } from "../../../generated/api/GradebookSummaryRow";
import type { EnrollmentId } from "../../../generated/api/EnrollmentId";
import type { ProblemId } from "../../../generated/api/ProblemId";
import type { QuestionAttemptId } from "../../../generated/api/QuestionAttemptId";
import type { QuestionEnvelope } from "../../../generated/api/QuestionEnvelope";
import type { RunId } from "../../../generated/api/RunId";
import type { StudentResponse } from "../../../generated/api/StudentResponse";
import type { VersionId } from "../../../generated/api/VersionId";
import type { DraftQuestionDefinition } from "../../../generated/api/DraftQuestionDefinition";
import type { WorkspaceId } from "../../../generated/api/WorkspaceId";
import type { PublicationScope } from "../../../generated/api/PublicationScope";
import type { ApiClient } from "../client";
import {
  ApiProtocolError,
  ApiRequestError,
  AssignmentConflictError,
  AssignmentValidationError,
  CourseAppearanceConflictError,
  CourseAppearanceFileError,
  WorkspaceConflictError,
} from "../http_client";
import { catalogProblemReferencePath, catalogSearchPath } from "../catalog_query";
import {
  decodeCatalogProblemDetail,
  decodeCatalogProblemSummary,
  decodeCatalogSearchPage,
  decodeCourseAppearance,
  decodeCourseBannerCandidateReceipt,
  decodeCourseCreateInput,
  decodeAssignmentCapabilityViolations,
  decodeAssignmentEditorDetail,
  decodeAssignmentEditorInput,
  decodeAssignmentSummary,
  decodeFeedbackReleaseResponse,
  decodeIssuedPresentationEnvelope,
  decodePrefetchedNextQuestion,
  decodeQuestionEnvelope,
  decodeRunSummaryResponse,
  decodeSubmissionReceipt,
} from "../decoders";
import type {
  AssignmentEditorDetail,
  AssignmentEditorInput,
  AuthSession,
  CourseCreateInput,
  CursorPage,
  EnrollmentView,
  ExternalToolLaunch,
  FeedbackReleaseResponse,
  RunScreenData,
  RunSummaryResponse,
  SubmissionReceipt,
  WorkspaceDraftDetail,
  WorkspaceDraftPage,
  PublicationDiff,
  PublicationResult,
} from "../contracts";
import { validateAssignmentConfigInMock } from "./capability_validation";
import { validateResponseFormatInMock } from "./format_validation";
import {
  createMockFetch,
  externalToolFixtureAttempt,
  mockCourseAppearance,
  secondaryMockCourse,
  mockAttemptById,
  mockExternalToolSubmissionReceipt,
  mockFeedbackForAttempt,
  prefetchFixtureAttempt,
  type MockFetch,
} from "./handlers";
import { timerVerdictInMock } from "./timer";
import { createMockEnrollmentClient } from "./enrollment";

async function expectSerialized<T>(responsePromise: Promise<Response>, expected: T): Promise<T> {
  const response = await responsePromise;
  const body = await response.text();
  if (!response.ok) {
    throw new Error(`Mock API request failed with status ${response.status}`);
  }
  const expectedBody = JSON.stringify(expected);
  if (body !== expectedBody) {
    throw new Error("Mock API response drifted from its typed fixture contract");
  }
  return expected;
}

async function decodeMockCatalogResponse<T>(
  responsePromise: Promise<Response>,
  path: string,
  decoder: (value: unknown, decoderPath?: string) => T,
): Promise<T> {
  const response = await responsePromise;
  if (!response.ok) {
    throw new ApiRequestError(response.status, path);
  }
  const body = await response.text();
  let value: unknown;
  try {
    value = JSON.parse(body);
  } catch (error: unknown) {
    const detail = error instanceof Error ? error.message : "invalid JSON";
    const protocolError = Object.assign(
      new Error(`Mock API response ${path} is not valid JSON: ${detail}`),
      { cause: error },
    );
    throw protocolError;
  }
  return decoder(value, "response");
}

async function decodeMockPrefetch(
  responsePromise: Promise<Response>,
  attemptId: QuestionAttemptId,
): Promise<import("../contracts").PrefetchedNextQuestion | null> {
  const response = await responsePromise;
  if (response.status === 204) return null;
  if (!response.ok)
    throw new ApiRequestError(response.status, `/api/attempts/${attemptId}/prefetch-next`);
  const decoded = decodePrefetchedNextQuestion(JSON.parse(await response.text()), "response");
  if (decoded.predecessor !== attemptId) {
    throw new ApiProtocolError("Mock prefetch response does not match its requested predecessor");
  }
  return decoded;
}

async function decodeMockSubmission(
  responsePromise: Promise<Response>,
  attemptId: QuestionAttemptId,
): Promise<SubmissionReceipt> {
  const response = await responsePromise;
  if (!response.ok) throw new ApiRequestError(response.status, `/api/submissions/${attemptId}`);
  const decoded = decodeSubmissionReceipt(JSON.parse(await response.text()), "response");
  if (decoded.attempt.id !== attemptId) {
    throw new ApiProtocolError("Mock submission receipt does not match its requested attempt");
  }
  if (
    decoded.nextIssued !== null &&
    (decoded.nextIssued.run !== decoded.attempt.run || decoded.nextIssued.id === decoded.attempt.id)
  ) {
    throw new ApiProtocolError("Mock submission receipt has an invalid next-attempt binding");
  }
  return decoded;
}

export interface MockApiClientConfig {
  readonly fetch?: MockFetch;
  /** Workspace authoring is denied unless a focused instructor fixture explicitly enables it. */
  readonly workspaceAuthoring?: boolean;
  /** Assignment mutation is denied unless a focused instructor fixture explicitly enables it. */
  readonly assignmentAuthoring?: boolean;
  /** Course appearance mutation is denied unless an instructor fixture explicitly enables it. */
  readonly courseAppearanceAuthoring?: boolean;
}

function requireMockNoStore(response: Response, path: string): void {
  const directives =
    response.headers
      .get("cache-control")
      ?.split(",")
      .map((directive) => directive.trim().toLowerCase()) ?? [];
  if (!directives.includes("no-store")) {
    throw new ApiProtocolError(`Mock API response ${path} must be no-store`);
  }
}

async function requestMockCourseAppearance(
  responsePromise: Promise<Response>,
  path: string,
): Promise<CourseAppearance> {
  const response = await responsePromise;
  requireMockNoStore(response, path);
  if (response.status === 412) throw new CourseAppearanceConflictError(path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  const appearance = decodeCourseAppearance(JSON.parse(await response.text()), "response");
  if (response.headers.get("etag") !== `"${appearance.revision}"`) {
    throw new ApiProtocolError(`Mock API response ${path} ETag does not match its revision`);
  }
  return appearance;
}

async function requestMockBannerCandidate(
  responsePromise: Promise<Response>,
  path: string,
): Promise<CourseBannerCandidateReceipt> {
  const response = await responsePromise;
  requireMockNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  if (response.status !== 201) {
    throw new ApiProtocolError(`Mock API response ${path} must use status 201`);
  }
  return decodeCourseBannerCandidateReceipt(JSON.parse(await response.text()), "response");
}

function validMockAssignmentRevision(revision: string): boolean {
  return (
    /^"[1-9][0-9]*"$/u.test(revision) && BigInt(revision.slice(1, -1)) <= 9_223_372_036_854_775_807n
  );
}

function mockAssignmentRevision(response: Response, path: string): string {
  const revision = response.headers.get("etag");
  if (revision === null || !validMockAssignmentRevision(revision)) {
    throw new ApiProtocolError(
      `Mock API response ${path} must include one positive strong numeric ETag`,
    );
  }
  return revision;
}

async function requestMockAssignment(
  responsePromise: Promise<Response>,
  path: string,
  expected: { readonly assignmentId?: AssignmentId; readonly courseId?: CourseId },
): Promise<AssignmentEditorDetail> {
  const response = await responsePromise;
  const text = await response.text();
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch (error: unknown) {
    const detail = error instanceof Error ? error.message : "invalid JSON";
    throw new ApiProtocolError(`Mock API response ${path} is not valid JSON: ${detail}`);
  }
  if (response.status === 409 || response.status === 428) {
    throw new AssignmentConflictError(response.status, path);
  }
  if (
    response.status === 422 &&
    typeof value === "object" &&
    value !== null &&
    !Array.isArray(value) &&
    "error" in value &&
    Reflect.get(value, "error") === "assignment configuration is not supported"
  ) {
    throw new AssignmentValidationError(path, decodeAssignmentCapabilityViolations(value));
  }
  if (!response.ok) throw new ApiRequestError(response.status, path);
  const detail = decodeAssignmentEditorDetail(value);
  if (expected.assignmentId !== undefined && detail.id !== expected.assignmentId) {
    throw new ApiProtocolError("Mock assignment response does not match the requested assignment");
  }
  if (expected.courseId !== undefined && detail.courseId !== expected.courseId) {
    throw new ApiProtocolError("Mock assignment response does not match the requested course");
  }
  return { ...detail, revision: mockAssignmentRevision(response, path) };
}

/** Creates the API client used by UI work before the Rust routes exist. */
export function createMockApiClient(config: MockApiClientConfig = {}): ApiClient {
  const mockFetch = config.fetch ?? createMockFetch();
  let workspaceDraft: DraftQuestionDefinition | undefined = publishedProblemFixture.draft;
  let workspaceRevision = 1;

  function workspaceAuthoringError(): Error | undefined {
    return config.workspaceAuthoring === true
      ? undefined
      : new Error("Mock workspace authoring is not authorized");
  }

  function assignmentAuthoringError(): Error | undefined {
    return config.assignmentAuthoring === true
      ? undefined
      : new Error("Mock assignment authoring is not authorized");
  }

  function courseAppearanceAuthoringError(): Error | undefined {
    return config.courseAppearanceAuthoring === true
      ? undefined
      : new Error("Mock course appearance authoring is not authorized");
  }

  function workspaceDetail(): WorkspaceDraftDetail {
    if (workspaceDraft === undefined) throw new Error("Mock workspace draft is unavailable");
    return { draft: workspaceDraft, revision: `"${workspaceRevision}"` };
  }

  function publicationProjection(draft: DraftQuestionDefinition): PublicationDiff["current"] {
    const response = draft.response;
    const optionCount =
      response.kind === "multipleChoice"
        ? response.choices.length
        : response.kind === "multiBlank"
          ? response.blanks.length
          : response.kind === "matching"
            ? response.prompts.length
            : response.kind === "ordering"
              ? response.items.length
              : response.kind === "hotspot"
                ? response.regions.length
                : null;
    return {
      sourceBackend: draft.source.backend,
      title: draft.metadata.title,
      prompt: { blocks: draft.prompt.map((block) => block.kind) },
      response: { kind: response.kind, optionCount },
      attemptPolicy: draft.attemptPolicy,
      timingPolicy: draft.timingPolicy,
      randomization: { kind: draft.randomization.kind },
      metadata: {
        tags: draft.metadata.tags,
        taxonomy: draft.metadata.taxonomy,
        license: draft.metadata.license,
        language: draft.metadata.language,
      },
    };
  }

  const client: ApiClient = {
    ...createMockEnrollmentClient(),
    resolveNavigation: (reference) => {
      if (reference === `C-${publishedProblemFixture.course.publicId}`) {
        return Promise.resolve({ kind: "course", courseId: publishedProblemFixture.course.id });
      }
      if (reference === `C-${secondaryMockCourse.publicId}`) {
        return Promise.resolve({ kind: "course", courseId: secondaryMockCourse.id });
      }
      if (reference === `A-${publishedProblemFixture.assignment.publicId}`) {
        return Promise.resolve({
          kind: "assignment",
          courseId: publishedProblemFixture.assignment.courseId,
          assignmentId: publishedProblemFixture.assignment.id,
        });
      }
      const run = publishedProblemFixture.runs.find(
        (candidate) => reference === `R-${candidate.publicId}`,
      );
      if (run !== undefined) return Promise.resolve({ kind: "run", runId: run.id });
      if (reference === "W-1" && workspaceDraft !== undefined) {
        return Promise.resolve({ kind: "workspace", workspaceId: workspaceDraft.workspace });
      }
      return Promise.reject(new Error(`Mock navigation target ${reference} is not found`));
    },
    getSession: () => {
      const expected: AuthSession = {
        authenticated: true,
        tenant: publishedProblemFixture.enrollment.tenant,
        user: {
          id: publishedProblemFixture.enrollment.user,
          displayName: "Fixture Student",
          roles: ["student"],
        },
      };
      return expectSerialized(mockFetch("/api/auth/session"), expected);
    },
    logout: async () => {
      await expectSerialized(mockFetch("/api/auth/logout", { method: "POST" }), {
        authenticated: false,
      });
    },
    listWorkspaceDrafts: (): Promise<WorkspaceDraftPage> => {
      const authorizationError = workspaceAuthoringError();
      if (authorizationError !== undefined) return Promise.reject(authorizationError);
      const draft = workspaceDraft;
      return Promise.resolve({
        items:
          draft === undefined
            ? []
            : [
                {
                  workspace: draft.workspace,
                  publicId: 1,
                  title: draft.metadata.title,
                  sourceBackend: draft.source.backend,
                },
              ],
        nextCursor: null,
      });
    },
    getWorkspaceDraft: (workspace: WorkspaceId): Promise<WorkspaceDraftDetail> => {
      const authorizationError = workspaceAuthoringError();
      if (authorizationError !== undefined) return Promise.reject(authorizationError);
      const detail = workspaceDetail();
      if (detail.draft.workspace !== workspace) {
        return Promise.reject(new Error("Mock workspace is not found"));
      }
      return Promise.resolve(detail);
    },
    saveWorkspaceDraft: (
      workspace: WorkspaceId,
      draft: DraftQuestionDefinition,
      revision?: string,
    ): Promise<WorkspaceDraftDetail> => {
      const authorizationError = workspaceAuthoringError();
      if (authorizationError !== undefined) return Promise.reject(authorizationError);
      if (draft.workspace !== workspace) {
        return Promise.reject(new Error("Mock workspace path/body mismatch"));
      }
      if (workspaceDraft !== undefined && revision !== `"${workspaceRevision}"`) {
        return Promise.reject(new WorkspaceConflictError(409, `/api/workspaces/${workspace}`));
      }
      if (workspaceDraft === undefined && revision !== undefined) {
        return Promise.reject(new Error("Mock workspace create must not include If-Match"));
      }
      workspaceDraft = draft;
      workspaceRevision += 1;
      return Promise.resolve(workspaceDetail());
    },
    deleteWorkspaceDraft: (workspace: WorkspaceId, revision: string): Promise<void> => {
      const authorizationError = workspaceAuthoringError();
      if (authorizationError !== undefined) return Promise.reject(authorizationError);
      const detail = workspaceDetail();
      if (detail.draft.workspace !== workspace) {
        return Promise.reject(new Error("Mock workspace is not found"));
      }
      if (revision !== `"${workspaceRevision}"`) {
        return Promise.reject(new WorkspaceConflictError(409, `/api/workspaces/${workspace}`));
      }
      workspaceDraft = undefined;
      return Promise.resolve();
    },
    validateWorkspacePublication: (workspace: WorkspaceId) => {
      const authorizationError = workspaceAuthoringError();
      if (authorizationError !== undefined) return Promise.reject(authorizationError);
      const detail = workspaceDetail();
      if (detail.draft.workspace !== workspace)
        return Promise.reject(new Error("Mock workspace is not found"));
      return Promise.resolve({
        kind: "capabilityReport",
        revision: `"${workspaceRevision}"`,
        violations: [],
      });
    },
    getWorkspacePublicationDiff: (workspace: WorkspaceId): Promise<PublicationDiff> => {
      const authorizationError = workspaceAuthoringError();
      if (authorizationError !== undefined) return Promise.reject(authorizationError);
      const detail = workspaceDetail();
      if (detail.draft.workspace !== workspace)
        return Promise.reject(new Error("Mock workspace is not found"));
      return Promise.resolve({
        draftRevision: workspaceRevision,
        revision: `"${workspaceRevision}"`,
        baseline: "firstPublication",
        prior: null,
        previous: null,
        current: publicationProjection(detail.draft),
        changed: [],
      });
    },
    publishWorkspace: (
      workspace: WorkspaceId,
      scope: PublicationScope,
      revision: string,
    ): Promise<PublicationResult> => {
      const authorizationError = workspaceAuthoringError();
      if (authorizationError !== undefined) return Promise.reject(authorizationError);
      if (scope !== "institution" && scope !== "public") {
        return Promise.reject(new Error("Mock publication scope is invalid"));
      }
      const detail = workspaceDetail();
      if (detail.draft.workspace !== workspace)
        return Promise.reject(new Error("Mock workspace is not found"));
      if (revision !== `"${workspaceRevision}"`) {
        return Promise.reject(
          new WorkspaceConflictError(409, `/api/problems/${workspace}/publish`),
        );
      }
      return Promise.resolve({
        reference: {
          problem: publishedProblemFixture.publishedProblem.problem,
          version: publishedProblemFixture.publishedProblem.version,
        },
      });
    },
    listProblems: (cursor?: string) => {
      const expected = {
        items: [publishedProblemFixture.catalogProblem],
        nextCursor: null,
      } satisfies CursorPage<(typeof publishedProblemFixture)["catalogProblem"]>;
      const suffix = cursor === undefined ? "" : `?cursor=${encodeURIComponent(cursor)}`;
      return expectSerialized(mockFetch(`/api/problems${suffix}`), expected);
    },
    searchCatalog: (query): Promise<CatalogSearchPage> => {
      const path = catalogSearchPath(query);
      return decodeMockCatalogResponse(mockFetch(path), path, decodeCatalogSearchPage);
    },
    resolveCatalogProblem: (displayReference) => {
      const path = catalogProblemReferencePath(displayReference);
      return decodeMockCatalogResponse(mockFetch(path), path, (value, decoderPath) =>
        decodeCatalogProblemSummary(value, decoderPath, true),
      );
    },
    getCatalogProblemDetail: (
      problemId: ProblemId,
      versionId: VersionId,
    ): Promise<CatalogProblemDetail> => {
      if (
        problemId !== publishedProblemFixture.publishedProblem.problem ||
        versionId !== publishedProblemFixture.publishedProblem.version
      ) {
        return Promise.reject(
          new Error("Mock catalog detail does not recognize this immutable version"),
        );
      }
      const path = `/api/problems/${problemId}/versions/${versionId}/detail`;
      return decodeMockCatalogResponse(mockFetch(path), path, decodeCatalogProblemDetail);
    },
    getProblemVersion: (problemId: ProblemId, versionId: VersionId) =>
      expectSerialized(
        mockFetch(`/api/problems/${problemId}/versions/${versionId}`),
        publishedProblemFixture.publishedProblem,
      ),
    listTaxonomy: (cursor?: string) => {
      const expected = {
        items: publishedProblemFixture.publishedProblem.metadata.taxonomy,
        nextCursor: null,
      } satisfies CursorPage<
        (typeof publishedProblemFixture)["publishedProblem"]["metadata"]["taxonomy"][number]
      >;
      const suffix = cursor === undefined ? "" : `?cursor=${encodeURIComponent(cursor)}`;
      return expectSerialized(mockFetch(`/api/taxonomy${suffix}`), expected);
    },
    listCourses: (cursor?: string) => {
      const expected = {
        items: [publishedProblemFixture.course],
        nextCursor: null,
      } satisfies CursorPage<(typeof publishedProblemFixture)["course"]>;
      const suffix = cursor === undefined ? "" : `?cursor=${encodeURIComponent(cursor)}`;
      return expectSerialized(mockFetch(`/api/courses${suffix}`), expected);
    },
    createCourse: (input: CourseCreateInput) => {
      const course = {
        ...publishedProblemFixture.course,
        title: decodeCourseCreateInput(input, "request").title,
        role: "instructor",
      } satisfies CourseSummary;
      return Promise.resolve(course);
    },
    getCourse: (courseId: CourseId) => {
      const expected =
        courseId === publishedProblemFixture.course.id
          ? publishedProblemFixture.course
          : courseId === secondaryMockCourse.id
            ? secondaryMockCourse
            : undefined;
      if (expected === undefined)
        return Promise.reject(new Error(`Fixture has no course ${courseId}`));
      return expectSerialized(mockFetch(`/api/courses/${courseId}`), expected);
    },
    getCourseAppearance: (courseId: CourseId) =>
      requestMockCourseAppearance(
        mockFetch(`/api/courses/${courseId}/appearance`),
        `/api/courses/${courseId}/appearance`,
      ),
    uploadCourseBannerCandidate: (courseId: CourseId, image: Blob) => {
      const authorizationError = courseAppearanceAuthoringError();
      if (authorizationError !== undefined) return Promise.reject(authorizationError);
      if (image.size <= 0) return Promise.reject(new CourseAppearanceFileError("image is empty"));
      if (image.size > 2 * 1_024 * 1_024) {
        return Promise.reject(new CourseAppearanceFileError("image exceeds 2 MiB"));
      }
      const path = `/api/courses/${courseId}/appearance/banner-candidates`;
      return requestMockBannerCandidate(
        mockFetch(path, {
          method: "POST",
          headers: { accept: "application/json", "content-type": image.type },
          body: image,
        }),
        path,
      );
    },
    saveCourseAppearance: (
      courseId: CourseId,
      update: CourseAppearanceUpdate,
      revision: string,
    ) => {
      const authorizationError = courseAppearanceAuthoringError();
      if (authorizationError !== undefined) return Promise.reject(authorizationError);
      const path = `/api/courses/${courseId}/appearance`;
      return requestMockCourseAppearance(
        mockFetch(path, {
          method: "PUT",
          headers: {
            accept: "application/json",
            "content-type": "application/json",
            "if-match": `"${revision}"`,
          },
          body: JSON.stringify(update),
        }),
        path,
      );
    },
    listGradebook: (courseId: CourseId, cursor?: string, pageSize?: number) => {
      if (courseId !== publishedProblemFixture.course.id) {
        return Promise.reject(new Error(`Fixture has no course ${courseId}`));
      }
      if (pageSize !== undefined && (!Number.isSafeInteger(pageSize) || pageSize <= 0)) {
        return Promise.reject(new Error("gradebook pageSize must be a positive safe integer"));
      }
      const query = new URLSearchParams();
      if (cursor !== undefined) {
        query.set("cursor", cursor);
      }
      if (pageSize !== undefined) {
        query.set("pageSize", String(pageSize));
      }
      const suffix = query.size === 0 ? "" : `?${query.toString()}`;
      const expected = {
        items: publishedProblemFixture.gradebook,
        nextCursor: null,
      } satisfies CursorPage<GradebookSummaryRow>;
      return expectSerialized(mockFetch(`/api/courses/${courseId}/gradebook${suffix}`), expected);
    },
    listAssignments: (courseId: CourseId, cursor?: string) => {
      const expected = {
        items: courseId === secondaryMockCourse.id ? [] : [publishedProblemFixture.assignment],
        nextCursor: null,
      } satisfies CursorPage<(typeof publishedProblemFixture)["assignment"]>;
      const suffix = cursor === undefined ? "" : `?cursor=${encodeURIComponent(cursor)}`;
      return expectSerialized(mockFetch(`/api/courses/${courseId}/assignments${suffix}`), expected);
    },
    getAssignment: (assignmentId: AssignmentId) => {
      const path = `/api/assignments/${assignmentId}`;
      return decodeMockCatalogResponse(mockFetch(path), path, decodeAssignmentSummary);
    },
    getAssignmentEditor: (assignmentId: AssignmentId) =>
      requestMockAssignment(
        mockFetch(`/api/assignments/${assignmentId}`),
        `/api/assignments/${assignmentId}`,
        { assignmentId },
      ),
    createAssignment: (courseId: CourseId, input: AssignmentEditorInput) => {
      const authorizationError = assignmentAuthoringError();
      if (authorizationError !== undefined) return Promise.reject(authorizationError);
      const body = decodeAssignmentEditorInput(input, "request");
      const path = `/api/courses/${courseId}/assignments`;
      return requestMockAssignment(
        mockFetch(path, {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify(body),
        }),
        path,
        { courseId },
      );
    },
    saveAssignment: (
      courseId: CourseId,
      assignmentId: AssignmentId,
      input: AssignmentEditorInput,
      revision: string,
    ) => {
      const authorizationError = assignmentAuthoringError();
      if (authorizationError !== undefined) return Promise.reject(authorizationError);
      if (!validMockAssignmentRevision(revision)) {
        return Promise.reject(
          new ApiProtocolError("assignment revision must be one positive strong numeric ETag"),
        );
      }
      const body = decodeAssignmentEditorInput(input, "request");
      const path = `/api/courses/${courseId}/assignments/${assignmentId}`;
      return requestMockAssignment(
        mockFetch(path, {
          method: "PUT",
          headers: { "content-type": "application/json", "if-match": revision },
          body: JSON.stringify(body),
        }),
        path,
        { courseId, assignmentId },
      );
    },
    getEnrollment: (enrollmentId: EnrollmentId) => {
      const expected: EnrollmentView = {
        enrollment: publishedProblemFixture.enrollment,
        summary: publishedProblemFixture.summary,
      };
      return expectSerialized(mockFetch(`/api/enrollments/${enrollmentId}`), expected);
    },
    listRuns: (enrollmentId: EnrollmentId, cursor?: string) => {
      const expected = {
        items: publishedProblemFixture.runs.filter((run) => run.enrollment === enrollmentId),
        nextCursor: null,
      } satisfies CursorPage<(typeof publishedProblemFixture)["runs"][number]>;
      const suffix = cursor === undefined ? "" : `?cursor=${encodeURIComponent(cursor)}`;
      return expectSerialized(
        mockFetch(`/api/enrollments/${enrollmentId}/runs${suffix}`),
        expected,
      );
    },
    startRun: (assignmentId: AssignmentId) => {
      if (assignmentId !== publishedProblemFixture.assignment.id) {
        return Promise.reject(new Error(`Fixture has no assignment ${assignmentId}`));
      }
      const run = publishedProblemFixture.runs.find((candidate) => candidate.completedAt === null);
      if (run === undefined) {
        return Promise.reject(new Error("Fixture has no active run"));
      }
      return expectSerialized(
        mockFetch("/api/runs", {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({ assignmentId }),
        }),
        run,
      );
    },
    getRun: (runId: RunId) => {
      const run = publishedProblemFixture.runs.find((candidate) => candidate.id === runId);
      if (run === undefined) {
        return Promise.reject(new Error(`Fixture has no run ${runId}`));
      }
      return expectSerialized(mockFetch(`/api/runs/${runId}`), run);
    },
    getRunSummary: (
      runId: RunId,
      cursor?: string,
      pageSize?: number,
    ): Promise<RunSummaryResponse> => {
      if (cursor !== undefined && (cursor.length === 0 || cursor.length > 512)) {
        return Promise.reject(new Error("run summary cursor must be 1 through 512 characters"));
      }
      if (
        pageSize !== undefined &&
        (!Number.isSafeInteger(pageSize) || pageSize < 1 || pageSize > 100)
      ) {
        return Promise.reject(new Error("run summary pageSize must be 1 through 100"));
      }
      const query = new URLSearchParams();
      if (cursor !== undefined) query.set("cursor", cursor);
      if (pageSize !== undefined) query.set("pageSize", String(pageSize));
      const suffix = query.size === 0 ? "" : `?${query.toString()}`;
      return decodeMockCatalogResponse(
        mockFetch(`/api/runs/${runId}/summary${suffix}`),
        "run summary",
        (value, path) => decodeRunSummaryResponse(value, path),
      );
    },
    listAttempts: (runId: RunId, cursor?: string) => {
      const expected = {
        items: publishedProblemFixture.attempts.filter((attempt) => attempt.run === runId),
        nextCursor: null,
      } satisfies CursorPage<(typeof publishedProblemFixture)["attempts"][number]>;
      const suffix = cursor === undefined ? "" : `?cursor=${encodeURIComponent(cursor)}`;
      return expectSerialized(mockFetch(`/api/runs/${runId}/attempts${suffix}`), expected);
    },
    getAttempt: (attemptId: QuestionAttemptId) => {
      const attempt = publishedProblemFixture.attempts.find(
        (candidate) => candidate.id === attemptId,
      );
      if (attempt === undefined) {
        return Promise.reject(new Error(`Fixture has no attempt ${attemptId}`));
      }
      return expectSerialized(mockFetch(`/api/attempts/${attemptId}`), attempt);
    },
    getIssuedQuestion: (attemptId: QuestionAttemptId): Promise<QuestionEnvelope> => {
      const attempt = mockAttemptById(attemptId);
      if (attempt === undefined) {
        return Promise.reject(new Error(`Fixture has no attempt ${attemptId}`));
      }
      return decodeMockCatalogResponse(
        mockFetch(`/api/attempts/${attemptId}/question`),
        "issued question",
        attempt.issuedCapability === "notApplicable"
          ? decodeQuestionEnvelope
          : decodeIssuedPresentationEnvelope,
      );
    },
    prefetchNextQuestion: (attemptId) =>
      decodeMockPrefetch(
        mockFetch(`/api/attempts/${attemptId}/prefetch-next`, { method: "POST" }),
        attemptId,
      ),
    beginExternalToolLaunch: (attemptId: QuestionAttemptId): Promise<ExternalToolLaunch> => {
      const attempt = mockAttemptById(attemptId);
      if (attempt === undefined) {
        return Promise.reject(new Error(`Fixture has no attempt ${attemptId}`));
      }
      return expectSerialized(
        mockFetch(`/api/attempts/${attemptId}/external-tool/launch`, { method: "POST" }),
        {
          launchUrl: `/api/attempts/${attempt.id}/external-tool/launch`,
        },
      );
    },
    submitResponse: (
      attemptId: QuestionAttemptId,
      response: StudentResponse,
      idempotencyKey: string,
    ) => {
      if (attemptId === prefetchFixtureAttempt.id) {
        return decodeMockSubmission(
          mockFetch(`/api/submissions/${attemptId}`, {
            method: "POST",
            headers: { "content-type": "application/json", "idempotency-key": idempotencyKey },
            body: JSON.stringify({ response }),
          }),
          attemptId,
        );
      }
      if (response.kind === "externalTool") {
        if (attemptId !== externalToolFixtureAttempt.id) {
          return Promise.reject(new Error(`Fixture has no external-tool attempt for ${attemptId}`));
        }
        const expected = mockExternalToolSubmissionReceipt();
        return expectSerialized(
          mockFetch(`/api/attempts/${attemptId}/external-tool/launch/submission`, {
            method: "POST",
            headers: {
              "content-type": "application/json",
              "idempotency-key": idempotencyKey,
            },
            body: JSON.stringify({ response }),
          }),
          expected,
        );
      }
      const attempt = mockAttemptById(attemptId);
      if (attempt === undefined) {
        return Promise.reject(new Error(`Fixture has no submission receipt for ${attemptId}`));
      }
      const expected: SubmissionReceipt = {
        accepted: true,
        attempt,
        feedback: mockFeedbackForAttempt(attempt),
        nextIssued: null,
        nextPending: false,
      };
      return expectSerialized(
        mockFetch(`/api/submissions/${attemptId}`, {
          method: "POST",
          headers: {
            "content-type": "application/json",
            "idempotency-key": idempotencyKey,
          },
          body: JSON.stringify({ response }),
        }),
        expected,
      );
    },
    getSummary: (enrollmentId: EnrollmentId) =>
      expectSerialized(
        mockFetch(`/api/grading/summaries/${enrollmentId}`),
        publishedProblemFixture.summary,
      ),
    releaseAttemptFeedback: (attemptId: QuestionAttemptId): Promise<FeedbackReleaseResponse> =>
      decodeMockCatalogResponse(
        mockFetch(`/api/attempts/${attemptId}/feedback-release`, { method: "POST" }),
        "feedback release",
        decodeFeedbackReleaseResponse,
      ),
    getRunScreen: async (runId: RunId): Promise<RunScreenData> => {
      const [run, attempts] = await Promise.all([client.getRun(runId), client.listAttempts(runId)]);
      const attempt = attempts.items[0];
      if (attempt === undefined) {
        throw new Error(`Run ${runId} has no question attempt`);
      }
      const issuedQuestion = await client.getIssuedQuestion(attempt.id);
      return {
        course: {
          summary: publishedProblemFixture.course,
          appearance: mockCourseAppearance,
        },
        assignment: publishedProblemFixture.assignment,
        run,
        attempt,
        issuedQuestion,
      };
    },
    issueProtectedAssetDelivery: (assetId) => Promise.resolve(`/api/assets/${assetId}`),
    assetUrl: (assetId) => `/api/assets/${assetId}`,
    validateResponseFormatOnServer: validateResponseFormatInMock,
    timerVerdictOnServer: timerVerdictInMock,
    validateAssignmentConfigOnServer: validateAssignmentConfigInMock,
  };

  return client;
}
