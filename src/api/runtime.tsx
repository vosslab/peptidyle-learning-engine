// runtime.tsx - one API client and one set of router-owned query identities.

import { query } from "@solidjs/router";
import { createContext, useContext, type JSX } from "solid-js";

import type { AssignmentId } from "../../generated/api/AssignmentId";
import type { CourseId } from "../../generated/api/CourseId";
import type { CatalogProblemDetail } from "../../generated/api/CatalogProblemDetail";
import type { CatalogSearchPage } from "../../generated/api/CatalogSearchPage";
import type { CatalogSearchQuery } from "../../generated/api/CatalogSearchQuery";
import type { QuestionId } from "../../generated/api/QuestionId";
import type { GradebookSummaryRow } from "../../generated/api/GradebookSummaryRow";
import type { RunId } from "../../generated/api/RunId";
import type { StudentAssignmentSummary } from "../../generated/api/StudentAssignmentSummary";
import type { ApiClient } from "./client";
import type {
  AssignmentSummary,
  AssignmentSummaryWithTiming,
  CourseRouteData,
  CourseSummary,
  CursorPage,
  RunScreenData,
  RunSummaryResponse,
} from "./contracts";

interface QueryFunction<Arguments extends ReadonlyArray<unknown>, Result> {
  (...arguments_: Arguments): Promise<Result>;
  readonly key: string;
  readonly keyFor: (...arguments_: Arguments) => string;
}

export interface ApiRuntime {
  readonly client: ApiClient;
  readonly queries: {
    readonly courses: QueryFunction<[], CursorPage<CourseSummary>>;
    readonly catalogSearch: QueryFunction<[CatalogSearchQuery], CatalogSearchPage>;
    readonly catalogDetail: QueryFunction<[QuestionId], CatalogProblemDetail>;
    readonly gradebook: QueryFunction<[CourseId], CursorPage<GradebookSummaryRow>>;
    readonly assignments: QueryFunction<[CourseId], CursorPage<AssignmentSummary>>;
    readonly assignment: QueryFunction<[AssignmentId], AssignmentSummaryWithTiming>;
    readonly assignmentSummary: QueryFunction<[AssignmentId], StudentAssignmentSummary>;
    readonly courseScope: QueryFunction<[CourseId], CourseRouteData>;
    readonly runScreen: QueryFunction<[RunId], RunScreenData>;
    readonly runSummary: QueryFunction<[RunId], RunSummaryResponse>;
  };
}

/** Creates stable query identities around one injected transport. */
export function createApiRuntime(client: ApiClient): ApiRuntime {
  return {
    client,
    queries: {
      courses: query(() => client.listCourses(), "course-list"),
      catalogSearch: query(
        (search: CatalogSearchQuery) => client.searchCatalog(search),
        "catalog-search",
      ),
      catalogDetail: query(
        (questionId: QuestionId) => client.getCatalogProblemDetail(questionId),
        "catalog-detail",
      ),
      gradebook: query((courseId: CourseId) => client.listGradebook(courseId), "course-gradebook"),
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
      runScreen: query((runId: RunId) => client.getRunScreen(runId), "run-screen"),
      runSummary: query(
        (runId: RunId) => client.getRunSummary(runId, undefined, 30),
        "run-summary",
      ),
    },
  };
}

const ApiRuntimeContext = createContext<ApiRuntime>();

export interface ApiRuntimeProviderProps {
  readonly runtime: ApiRuntime;
  readonly children: JSX.Element;
}

/** Makes the injected client available without exposing its mock implementation. */
export function ApiRuntimeProvider(props: ApiRuntimeProviderProps): JSX.Element {
  return (
    <ApiRuntimeContext.Provider value={props.runtime}>{props.children}</ApiRuntimeContext.Provider>
  );
}

/** Reads the app-owned client/query runtime. */
export function useApiRuntime(): ApiRuntime {
  const runtime = useContext(ApiRuntimeContext);
  if (runtime === undefined) {
    throw new Error("ApiRuntimeProvider is missing from the application root");
  }
  return runtime;
}
