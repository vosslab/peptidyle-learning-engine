// runtime.tsx - one API client and one set of router-owned query identities.

import { query } from "@solidjs/router";
import { createContext, useContext, type JSX } from "solid-js";

import type { AssignmentId } from "../../generated/api/AssignmentId";
import type { CourseId } from "../../generated/api/CourseId";
import type { RunId } from "../../generated/api/RunId";
import type { ApiClient } from "./client";
import type { AssignmentSummary, CourseSummary, CursorPage, RunScreenData } from "./contracts";

interface QueryFunction<Arguments extends ReadonlyArray<unknown>, Result> {
  (...arguments_: Arguments): Promise<Result>;
  readonly key: string;
  readonly keyFor: (...arguments_: Arguments) => string;
}

export interface ApiRuntime {
  readonly client: ApiClient;
  readonly queries: {
    readonly courses: QueryFunction<[], CursorPage<CourseSummary>>;
    readonly assignments: QueryFunction<[CourseId], CursorPage<AssignmentSummary>>;
    readonly assignment: QueryFunction<[AssignmentId], AssignmentSummary>;
    readonly runScreen: QueryFunction<[RunId], RunScreenData>;
  };
}

/** Creates stable query identities around one injected transport. */
export function createApiRuntime(client: ApiClient): ApiRuntime {
  return {
    client,
    queries: {
      courses: query(() => client.listCourses(), "course-list"),
      assignments: query(
        (courseId: CourseId) => client.listAssignments(courseId),
        "course-assignments",
      ),
      assignment: query(
        (assignmentId: AssignmentId) => client.getAssignment(assignmentId),
        "assignment-overview",
      ),
      runScreen: query((runId: RunId) => client.getRunScreen(runId), "run-screen"),
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
