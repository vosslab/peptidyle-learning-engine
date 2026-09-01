// application_api.tsx - one API client and one set of router-owned query identities.

import { query } from "@solidjs/router";
import { createContext, useContext, type JSX } from "solid-js";

import type { AssignmentId } from "../../generated/api/AssignmentId";
import type { CourseId } from "../../generated/api/CourseId";
import type { QuestionDetails } from "../../generated/api/QuestionDetails";
import type { QuestionSearchPage } from "../../generated/api/QuestionSearchPage";
import type { QuestionSearchRequest } from "../../generated/api/QuestionSearchRequest";
import type { QuestionId } from "../../generated/api/QuestionId";
import type { AssignmentAttemptId } from "../../generated/api/AssignmentAttemptId";
import type { AssignmentProgress } from "../../generated/api/AssignmentProgress";
import type { ApiClient, OrdinaryBrowserApiClient } from "./client";
import type { CalculatedGradebookResult } from "./decoders/calculated_gradebook";
import type {
  StudentAssignmentLandingSummary,
  StudentAssignmentDetail,
  CourseRouteView,
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

export interface ApplicationApi<Client extends ApiClient = ApiClient> {
  readonly client: Client;
  readonly queries: {
    readonly courses: QueryFunction<[], CursorPage<CourseSummary>>;
    readonly questionSearch: QueryFunction<[QuestionSearchRequest], QuestionSearchPage>;
    readonly questionDetails: QueryFunction<[QuestionId], QuestionDetails>;
    readonly gradebook: QueryFunction<[CourseId], CalculatedGradebookResult>;
    readonly assignments: QueryFunction<[CourseId], CursorPage<StudentAssignmentLandingSummary>>;
    readonly assignment: QueryFunction<[AssignmentId], StudentAssignmentDetail>;
    readonly assignmentSummary: QueryFunction<[AssignmentId], AssignmentProgress>;
    readonly courseScope: QueryFunction<[CourseId], CourseRouteView>;
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
export function createApplicationApi<Client extends ApiClient>(
  client: Client,
): ApplicationApi<Client> {
  return {
    client,
    queries: {
      courses: query(() => client.listCourses(), "course-list"),
      questionSearch: query(
        (search: QuestionSearchRequest) => client.searchQuestionLibrary(search),
        "question-search",
      ),
      questionDetails: query(
        (questionId: QuestionId) => client.getQuestionDetails(questionId),
        "question-details",
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

const ApplicationApiContext = createContext<ApplicationApi<OrdinaryBrowserApiClient>>();

export interface ApplicationApiProviderProps {
  readonly applicationApi: ApplicationApi<OrdinaryBrowserApiClient>;
  readonly children: JSX.Element;
}

/** Makes the injected API client and query identities available to routed pages. */
export function ApplicationApiProvider(props: ApplicationApiProviderProps): JSX.Element {
  return (
    <ApplicationApiContext.Provider value={props.applicationApi}>
      {props.children}
    </ApplicationApiContext.Provider>
  );
}

/** Reads the app-owned client and query definitions. */
export function useApplicationApi(): ApplicationApi<OrdinaryBrowserApiClient> {
  const applicationApi = useContext(ApplicationApiContext);
  if (applicationApi === undefined) {
    throw new Error("ApplicationApiProvider is missing from the application root");
  }
  return applicationApi;
}
