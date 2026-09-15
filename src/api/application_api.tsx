// application_api.tsx - one API client and one set of router-owned query identities.

import { query } from "@solidjs/router";
import { createContext, useContext, type JSX } from "solid-js";

import type { AssessmentId } from "../../generated/api/AssessmentId";
import type { CourseId } from "../../generated/api/CourseId";
import type { QuestionDetails } from "../../generated/api/QuestionDetails";
import type { QuestionSearchPage } from "../../generated/api/QuestionSearchPage";
import type { QuestionSearchRequest } from "../../generated/api/QuestionSearchRequest";
import type { QuestionId } from "../../generated/api/QuestionId";
import type { StudentAssessmentProgress } from "../../generated/api/StudentAssessmentProgress";
import type { ApiClient, OrdinaryBrowserApiClient } from "./client";
import type {
  StudentAssessmentLandingSummary,
  StudentAssessmentDetail,
  CourseRouteView,
  CourseSummary,
  CursorPage,
} from "./contracts";
import {
  resolveAssessmentAttemptIdentity,
  resolveCourseIdentity,
  type ResolvedAssessmentAttemptIdentity,
  type ResolvedCourseIdentity,
} from "../navigation/resolved_route";
import type {
  AssessmentAttemptRouteReference,
  CourseInstanceRouteReference,
} from "../navigation/public_route";
import type { StudentAssessmentAttemptContext } from "./assessment_attempt_navigation";
import type { StudentAssessmentAttemptHistory } from "./assessment_attempt_history";

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
    readonly assessments: QueryFunction<[CourseId], CursorPage<StudentAssessmentLandingSummary>>;
    readonly assessment: QueryFunction<[AssessmentId], StudentAssessmentDetail>;
    readonly assessmentSummary: QueryFunction<[AssessmentId], StudentAssessmentProgress>;
    readonly courseScope: QueryFunction<[CourseId], CourseRouteView>;
    readonly assessmentAttemptHistory: QueryFunction<
      [AssessmentAttemptRouteReference],
      StudentAssessmentAttemptHistory
    >;
    /** Live Student Attempt presentation scope keyed directly by R-n. */
    readonly assessmentAttemptScope: QueryFunction<
      [AssessmentAttemptRouteReference],
      StudentAssessmentAttemptContext
    >;
    /** Public-reference keyed scope identity; not an authorization result. */
    readonly resolveCourse: QueryFunction<[CourseInstanceRouteReference], ResolvedCourseIdentity>;
    /** Public-reference keyed attempt scope identity; not an authorization result. */
    readonly resolveAssessmentAttempt: QueryFunction<
      [AssessmentAttemptRouteReference],
      ResolvedAssessmentAttemptIdentity
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
      assessments: query(
        (courseId: CourseId) => client.listAssessments(courseId),
        "course-assessments",
      ),
      assessment: query(
        (assessmentId: AssessmentId) => client.getAssessment(assessmentId),
        "assessment-overview",
      ),
      assessmentSummary: query(
        (assessmentId: AssessmentId) => client.getAssessmentSummary(assessmentId),
        "assessment-summary",
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
      assessmentAttemptHistory: query(
        (reference: AssessmentAttemptRouteReference) =>
          client.getStudentAssessmentAttemptHistory(reference),
        "assessment-attempt-history",
      ),
      assessmentAttemptScope: query(
        (reference: AssessmentAttemptRouteReference) =>
          client.getStudentAssessmentAttemptContext(reference),
        "assessment-attempt-scope",
      ),
      resolveCourse: query(
        (reference: CourseInstanceRouteReference) => resolveCourseIdentity(client, reference),
        "resolve-course",
      ),
      resolveAssessmentAttempt: query(
        (reference: AssessmentAttemptRouteReference) =>
          resolveAssessmentAttemptIdentity(client, reference),
        "resolve-assessment-attempt",
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
