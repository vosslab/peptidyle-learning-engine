// runtime.tsx - one API client and one set of router-owned query identities.

import { query } from "@solidjs/router";
import { createContext, useContext, type JSX } from "solid-js";

import type { AssignmentId } from "../../generated/api/AssignmentId";
import type { CourseId } from "../../generated/api/CourseId";
import type { CatalogQuestionDetail } from "../../generated/api/CatalogQuestionDetail";
import type { CatalogSearchPage } from "../../generated/api/CatalogSearchPage";
import type { CatalogSearchQuery } from "../../generated/api/CatalogSearchQuery";
import type { QuestionId } from "../../generated/api/QuestionId";
import type { AssignmentAttemptId } from "../../generated/api/AssignmentAttemptId";
import type { AssignmentProgress } from "../../generated/api/AssignmentProgress";
import type { ApiClient, OrdinaryBrowserApiClient } from "./client";
import type { CalculatedGradebookResult } from "./decoders/calculated_gradebook";
import type {
  StudentAssignmentLandingSummary,
  StudentAssignmentDetail,
  CourseRouteData,
  CourseSummary,
  CursorPage,
  AssignmentAttemptScreenData,
  AssignmentAttemptSummaryResponse,
} from "./contracts";

interface QueryFunction<Arguments extends ReadonlyArray<unknown>, Result> {
  (...arguments_: Arguments): Promise<Result>;
  readonly key: string;
  readonly keyFor: (...arguments_: Arguments) => string;
}

export interface ApiRuntime<Client extends ApiClient = ApiClient> {
  readonly client: Client;
  readonly queries: {
    readonly courses: QueryFunction<[], CursorPage<CourseSummary>>;
    readonly catalogSearch: QueryFunction<[CatalogSearchQuery], CatalogSearchPage>;
    readonly catalogDetail: QueryFunction<[QuestionId], CatalogQuestionDetail>;
    readonly gradebook: QueryFunction<[CourseId], CalculatedGradebookResult>;
    readonly assignments: QueryFunction<[CourseId], CursorPage<StudentAssignmentLandingSummary>>;
    readonly assignment: QueryFunction<[AssignmentId], StudentAssignmentDetail>;
    readonly assignmentSummary: QueryFunction<[AssignmentId], AssignmentProgress>;
    readonly courseScope: QueryFunction<[CourseId], CourseRouteData>;
    readonly assignmentAttemptScreen: QueryFunction<
      [AssignmentAttemptId],
      AssignmentAttemptScreenData
    >;
    readonly assignmentAttemptSummary: QueryFunction<
      [AssignmentAttemptId],
      AssignmentAttemptSummaryResponse
    >;
  };
}

/** Creates stable query identities around one injected transport. */
export function createApiRuntime<Client extends ApiClient>(client: Client): ApiRuntime<Client> {
  return {
    client,
    queries: {
      courses: query(() => client.listCourses(), "course-list"),
      catalogSearch: query(
        (search: CatalogSearchQuery) => client.searchCatalog(search),
        "catalog-search",
      ),
      catalogDetail: query(
        (questionId: QuestionId) => client.getCatalogQuestionDetail(questionId),
        "catalog-detail",
      ),
      gradebook: query(
        (courseId: CourseId) => client.getCalculatedGradebook(courseId),
        "course-gradebook",
      ),
      assignments: query(
        (courseId: CourseId) => client.listAssignments(courseId),
        "course-assignments",
      ),
      assignment: query(
        (assignmentId: AssignmentId) => client.getAssignment(assignmentId),
        "assignment-overview",
      ),
      assignmentSummary: query(
        (assignmentId: AssignmentId) => client.getAssignmentSummary(assignmentId),
        "assignment-summary",
      ),
      courseScope: query(async (courseId: CourseId) => {
        const [summary, appearance] = await Promise.all([
          client.getCourse(courseId),
          client.getCourseAppearance(courseId),
        ]);
        if (summary.id !== courseId) {
          throw new Error("Course scope response does not match the requested course");
        }
        return { summary, appearance };
      }, "course-scope"),
      assignmentAttemptScreen: query(
        (assignmentAttemptId: AssignmentAttemptId) =>
          client.getAssignmentAttemptScreen(assignmentAttemptId),
        "assignment-attempt-screen",
      ),
      assignmentAttemptSummary: query(
        (assignmentAttemptId: AssignmentAttemptId) =>
          client.getAssignmentAttemptSummary(assignmentAttemptId, undefined, 30),
        "assignment-attempt-summary",
      ),
    },
  };
}

const ApiRuntimeContext = createContext<ApiRuntime<OrdinaryBrowserApiClient>>();

export interface ApiRuntimeProviderProps {
  readonly runtime: ApiRuntime<OrdinaryBrowserApiClient>;
  readonly children: JSX.Element;
}

/** Makes the injected API client and query identities available to routed pages. */
export function ApiRuntimeProvider(props: ApiRuntimeProviderProps): JSX.Element {
  return (
    <ApiRuntimeContext.Provider value={props.runtime}>{props.children}</ApiRuntimeContext.Provider>
  );
}

/** Reads the app-owned client/query runtime. */
export function useApiRuntime(): ApiRuntime<OrdinaryBrowserApiClient> {
  const runtime = useContext(ApiRuntimeContext);
  if (runtime === undefined) {
    throw new Error("ApiRuntimeProvider is missing from the application root");
  }
  return runtime;
}
