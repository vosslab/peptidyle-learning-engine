// client.ts - typed, server-free API client backed by the WP-C7 handlers.

import { publishedProblemFixture } from "../../../generated/fixtures/published_problem";
import type { AssignmentId } from "../../../generated/api/AssignmentId";
import type { CourseId } from "../../../generated/api/CourseId";
import type { EnrollmentId } from "../../../generated/api/EnrollmentId";
import type { ProblemId } from "../../../generated/api/ProblemId";
import type { QuestionAttemptId } from "../../../generated/api/QuestionAttemptId";
import type { RunId } from "../../../generated/api/RunId";
import type { StudentResponse } from "../../../generated/api/StudentResponse";
import type { VersionId } from "../../../generated/api/VersionId";
import type { ApiClient } from "../client";
import type {
  AuthSession,
  CursorPage,
  EnrollmentView,
  RunScreenData,
  SubmissionReceipt,
} from "../contracts";
import { validateAssignmentConfigInMock } from "./capability_validation";
import { validateResponseFormatInMock } from "./format_validation";
import { createMockFetch } from "./handlers";
import { timerVerdictInMock } from "./timer";

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

/** Creates the API client used by UI work before the Rust routes exist. */
export function createMockApiClient(): ApiClient {
  const mockFetch = createMockFetch();

  const client: ApiClient = {
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
    listProblems: (cursor?: string) => {
      const expected = {
        items: [publishedProblemFixture.catalogProblem],
        nextCursor: null,
      } satisfies CursorPage<(typeof publishedProblemFixture)["catalogProblem"]>;
      const suffix = cursor === undefined ? "" : `?cursor=${encodeURIComponent(cursor)}`;
      return expectSerialized(mockFetch(`/api/problems${suffix}`), expected);
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
    getCourse: (courseId: CourseId) => {
      if (courseId !== publishedProblemFixture.course.id) {
        return Promise.reject(new Error(`Fixture has no course ${courseId}`));
      }
      return expectSerialized(
        mockFetch(`/api/courses/${courseId}`),
        publishedProblemFixture.course,
      );
    },
    listAssignments: (courseId: CourseId, cursor?: string) => {
      const expected = {
        items: [publishedProblemFixture.assignment],
        nextCursor: null,
      } satisfies CursorPage<(typeof publishedProblemFixture)["assignment"]>;
      const suffix = cursor === undefined ? "" : `?cursor=${encodeURIComponent(cursor)}`;
      return expectSerialized(mockFetch(`/api/courses/${courseId}/assignments${suffix}`), expected);
    },
    getAssignment: (assignmentId: AssignmentId) =>
      expectSerialized(
        mockFetch(`/api/assignments/${assignmentId}`),
        publishedProblemFixture.assignment,
      ),
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
    submitResponse: (
      attemptId: QuestionAttemptId,
      response: StudentResponse,
      idempotencyKey: string,
    ) => {
      const inProgress = publishedProblemFixture.attempts.find(
        (attempt) => attempt.result === null,
      );
      if (inProgress === undefined) {
        return Promise.reject(new Error("Fixture has no in-progress submission receipt"));
      }
      const expected: SubmissionReceipt = { accepted: true, attempt: inProgress };
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
    getRunScreen: async (runId: RunId): Promise<RunScreenData> => {
      const [run, attempts] = await Promise.all([client.getRun(runId), client.listAttempts(runId)]);
      const attempt = attempts.items[0];
      if (attempt === undefined) {
        throw new Error(`Run ${runId} has no question attempt`);
      }
      const question = await client.getProblemVersion(attempt.problem, attempt.questionVersion);
      return {
        course: publishedProblemFixture.course,
        assignment: publishedProblemFixture.assignment,
        run,
        attempt,
        question,
      };
    },
    assetUrl: (assetId) => `/api/assets/${assetId}`,
    validateResponseFormatOnServer: validateResponseFormatInMock,
    timerVerdictOnServer: timerVerdictInMock,
    validateAssignmentConfigOnServer: validateAssignmentConfigInMock,
  };

  return client;
}
