// application_api.tsx - one API client and one set of router-owned query identities.

import { query } from "@solidjs/router";
import { createContext, useContext, type JSX } from "solid-js";

import type { AssignmentId } from "../../generated/api/AssignmentId";
import type { CourseId } from "../../generated/api/CourseId";
import type { QuestionDetails } from "../../generated/api/QuestionDetails";
import type { QuestionSearchPage } from "../../generated/api/QuestionSearchPage";
import type { QuestionSearchRequest } from "../../generated/api/QuestionSearchRequest";
import type { QuestionId } from "../../generated/api/QuestionId";
import type { StudentAssignmentProgress } from "../../generated/api/StudentAssignmentProgress";
import type { ApiClient, OrdinaryBrowserApiClient } from "./client";
import type {
  StudentAssignmentLandingSummary,
  StudentAssignmentDetail,
  CourseRouteView,
  CourseSummary,
  CursorPage,
} from "./contracts";
import {
  resolveAssignmentAttemptIdentity,
  resolveCourseIdentity,
  type ResolvedAssignmentAttemptIdentity,
  type ResolvedCourseIdentity,
} from "../navigation/resolved_route";
import type {
  AssignmentAttemptRouteReference,
  CourseInstanceRouteReference,
} from "../navigation/public_route";
import type { StudentAssignmentAttemptContext } from "./assignment_attempt_navigation";
import type { StudentAssignmentAttemptHistory } from "./assignment_attempt_history";

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
    readonly assignments: QueryFunction<[CourseId], CursorPage<StudentAssignmentLandingSummary>>;
    readonly assignment: QueryFunction<[AssignmentId], StudentAssignmentDetail>;
    readonly assignmentSummary: QueryFunction<[AssignmentId], StudentAssignmentProgress>;
    readonly courseScope: QueryFunction<[CourseId], CourseRouteView>;
    readonly assignmentAttemptHistory: QueryFunction<
      [AssignmentAttemptRouteReference],
      StudentAssignmentAttemptHistory
    >;
    /** Live Student Attempt presentation scope keyed directly by R-n. */
    readonly assignmentAttemptScope: QueryFunction<
      [AssignmentAttemptRouteReference],
      StudentAssignmentAttemptContext
    >;
    /** Public-reference keyed scope identity; not an authorization result. */
    readonly resolveCourse: QueryFunction<[CourseInstanceRouteReference], ResolvedCourseIdentity>;
    /** Public-reference keyed attempt scope identity; not an authorization result. */
    readonly resolveAssignmentAttempt: QueryFunction<
      [AssignmentAttemptRouteReference],
      ResolvedAssignmentAttemptIdentity
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
          client.getCourseAppearanceView(courseId),
        ]);
        if (summary.id !== courseId) {
          throw new Error("Course scope response does not match the requested course");
        }
        return { summary, appearance };
      }, "course-scope"),
      assignmentAttemptHistory: query(
        (reference: AssignmentAttemptRouteReference) =>
          client.getStudentAssignmentAttemptHistory(reference),
        "assignment-attempt-history",
      ),
      assignmentAttemptScope: query(
        (reference: AssignmentAttemptRouteReference) =>
          client.getStudentAssignmentAttemptContext(reference),
        "assignment-attempt-scope",
      ),
      resolveCourse: query(
        (reference: CourseInstanceRouteReference) => resolveCourseIdentity(client, reference),
        "resolve-course",
      ),
      resolveAssignmentAttempt: query(
        (reference: AssignmentAttemptRouteReference) =>
          resolveAssignmentAttemptIdentity(client, reference),
        "resolve-assignment-attempt",
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
