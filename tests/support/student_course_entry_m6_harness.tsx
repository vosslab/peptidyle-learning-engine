// Compiled browser harness for the Student current-Course entry decisions.

import { MemoryRouter, Route, createMemoryHistory, query, useLocation } from "@solidjs/router";
import type { JSX } from "solid-js";
import { render } from "solid-js/web";

import type { StudentAssessmentDecisionSummary } from "../../generated/api/StudentAssessmentDecisionSummary";
import { ApplicationApiProvider, type ApplicationApi } from "../../src/api/application_api";
import type { OrdinaryBrowserApiClient } from "../../src/api/client";
import type {
  LiveStudentAssessmentLandingSummary,
  LiveStudentCourseLandingSummary,
} from "../../src/api/live_student_course_landing";
import { AssessmentOverviewPage } from "../../src/pages/assessment_overview_page";
import { StudentCourseLandingPage } from "../../src/pages/student_course_landing_page";
import { StudentCoursesPage } from "../../src/pages/student_courses_page";
import { RouteScopeProvider } from "../../src/ribbon/route_scope_context";
import type { RouteScopeQueries } from "../../src/ribbon/route_scope_controller";
import type { CourseRouteView } from "../../src/api/contracts";
import type { CourseClassification } from "../../generated/api/CourseClassification";
import type { StudentAssessmentAttemptContext } from "../../src/api/assessment_attempt_navigation";
import type { StudentAssessmentAttemptHistory } from "../../src/api/assessment_attempt_history";
import type {
  CourseInstanceRouteReference,
  AssessmentAttemptRouteReference,
} from "../../src/navigation/public_route";

type StudentCourseEntryCase = "zero" | "one" | "choose" | "many" | "landing";

const FIXTURE_CLASSIFICATION = {
  disciplineUuid: "018f5e7d-01b6-7c14-8a0b-4bfef6390d6d",
  subjectUuid: null,
  topicUuid: null,
  subtopicUuid: null,
  tags: [],
} satisfies CourseClassification;

const COURSE_ONE: LiveStudentCourseLandingSummary = {
  reference: "CI7K3M2QAZ",
  shortName: "BCHM 301",
  longName: "Biochemistry 301: Proteins and Peptides",
};
const COURSE_TWO: LiveStudentCourseLandingSummary = {
  reference: "CI4W8QF9AD",
  shortName: "BIOL 302",
  longName: "Molecular Genetics: Gene Regulation",
};
const ASSESSMENT_DECISION = {
  availableAt: Date.parse("2026-09-14T14:00:00Z"),
  dueAt: Date.parse("2026-09-16T22:00:00Z"),
  closesAt: Date.parse("2026-09-17T22:00:00Z"),
  timeLimitSeconds: 3_600,
  attemptLimit: 2,
  lateWorkRule: "reject",
  displayTimeZone: "America/Chicago",
  evaluatedAt: Date.parse("2026-09-14T15:00:00Z"),
  startDecision: "may_start",
  publicReason: null,
} as const;
const ASSESSMENT: LiveStudentAssessmentLandingSummary = {
  reference: "A9D2RX5AF",
  title: "Protein structure practice",
  assessmentType: "regular_assignment",
  decision: ASSESSMENT_DECISION,
  assessmentAttemptNumber: 2,
  assessmentAttemptCompletion: "inProgress",
  canResumeAssessmentAttempt: true,
  gradedQuestionCount: 1,
  savedQuestionCount: 1,
  questionCount: 4,
  assessmentScore: { pointsEarned: 7, pointsPossible: 8 },
};
const BONUS_ASSESSMENT: LiveStudentAssessmentLandingSummary = {
  ...ASSESSMENT,
  reference: "A4N8BQ2A1",
  title: "Bonus protein challenge",
  assessmentType: "bonus_assignment",
  decision: { ...ASSESSMENT_DECISION, attemptLimit: null },
  assessmentAttemptNumber: 1,
  assessmentAttemptCompletion: "completed",
  canResumeAssessmentAttempt: false,
  gradedQuestionCount: 2,
  savedQuestionCount: 0,
  questionCount: 2,
  assessmentScore: { pointsEarned: 3, pointsPossible: 0 },
};
const WITHHELD_ASSESSMENT: LiveStudentAssessmentLandingSummary = {
  reference: "A7K2CW4A0",
  title: "Peptide quiz",
  assessmentType: "quiz",
  decision: ASSESSMENT_DECISION,
  assessmentAttemptNumber: 1,
  assessmentAttemptCompletion: "completed",
  canResumeAssessmentAttempt: false,
  gradedQuestionCount: 4,
  savedQuestionCount: 0,
  questionCount: 4,
};
const UNSTARTED_ASSESSMENT: LiveStudentAssessmentLandingSummary = {
  ...WITHHELD_ASSESSMENT,
  reference: "A3N7DX6AT",
  title: "Protein folding quiz",
  assessmentAttemptNumber: null,
  assessmentAttemptCompletion: null,
  gradedQuestionCount: 0,
  savedQuestionCount: 0,
};

function coursesFor(
  caseName: StudentCourseEntryCase,
): ReadonlyArray<LiveStudentCourseLandingSummary> {
  switch (caseName) {
    case "zero":
      return [];
    case "one":
    case "choose":
    case "landing":
      return [COURSE_ONE];
    case "many":
      return [COURSE_ONE, COURSE_TWO];
  }
}

function HarnessRoot(props: { readonly children?: JSX.Element }): JSX.Element {
  const location = useLocation();
  return (
    <RouteScopeProvider pathname={() => location.pathname}>
      <LocationProbe />
      {props.children}
    </RouteScopeProvider>
  );
}

function LocationProbe(): JSX.Element {
  const location = useLocation();
  return <output data-m6-location>{`${location.pathname}${location.search}`}</output>;
}

export interface StudentCourseEntryM6Harness {
  readonly dispose: () => void;
  readonly location: () => string;
}

/** Mounts the production Student pages with only their current-Course projection controlled. */
export function mountStudentCourseEntryM6Harness(
  target: HTMLElement,
  caseName: StudentCourseEntryCase,
): StudentCourseEntryM6Harness {
  const history = createMemoryHistory();
  history.set({
    value:
      caseName === "landing"
        ? "/student/courses/CI7K3M2QAZ"
        : caseName === "choose"
          ? "/student?choose=1"
          : "/",
  });
  const courses = coursesFor(caseName);
  const currentDecision = (): StudentAssessmentDecisionSummary => ({
    ...ASSESSMENT_DECISION,
  });
  const currentAssessment = (): LiveStudentAssessmentLandingSummary => ({
    ...ASSESSMENT,
    decision: currentDecision(),
  });
  const applicationApi = {
    client: {
      listLiveStudentCourses: () => Promise.resolve(courses),
      startLiveAssessment: () =>
        Promise.resolve({
          assessmentAttempt: "00000000-0000-0000-0000-000000000006",
          assessment: ASSESSMENT.reference,
          attemptNumber: ASSESSMENT.assessmentAttemptNumber,
          resumed: true,
          title: ASSESSMENT.title,
          instructions: "",
          questions: [],
        }),
      listLiveStudentAssessments: () =>
        Promise.resolve(
          caseName === "landing"
            ? [currentAssessment(), BONUS_ASSESSMENT, WITHHELD_ASSESSMENT, UNSTARTED_ASSESSMENT]
            : [],
        ),
      getLiveAssessmentAccess: (_course: string, assessment: string) => {
        const item = [
          currentAssessment(),
          BONUS_ASSESSMENT,
          WITHHELD_ASSESSMENT,
          UNSTARTED_ASSESSMENT,
        ].find((candidate) => candidate.reference === assessment);
        if (item === undefined) throw new Error("Unknown harness Coursework");
        return Promise.resolve({
          decision: item.decision,
          activeAssessmentAttempt: item.canResumeAssessmentAttempt ? "00000000-0000-0000-0000-000000000006" : null,
          title: item.title,
          assessmentType: item.assessmentType,
          questionCount: item.questionCount,
          pointsPossible: item.assessmentScore?.pointsPossible ?? 8,
          previousAttempts:
            item.assessmentAttemptCompletion === "completed"
              ? [
                  {
                    assessmentAttempt: "00000000-0000-0000-0000-000000000005",
                    attemptNumber: 1,
                    state: "submitted",
                    score: item.assessmentScore,
                  },
                ]
              : [],
        });
      },
    },
    queries: {
      courseScope: query(
        (_reference: CourseInstanceRouteReference): Promise<CourseRouteView> =>
          Promise.resolve({
            summary: {
              reference: COURSE_ONE.reference,
              shortName: COURSE_ONE.shortName,
              longName: COURSE_ONE.longName,
              classification: FIXTURE_CLASSIFICATION,
              term: { startDate: "2026-08-31", endDate: "2026-12-12" },
              role: "student",
            },
            appearance: { theme: "ocean", banner: null },
          }),
        "m6-course-scope",
      ),
      assessmentAttemptScope: query(
        (_reference: AssessmentAttemptRouteReference): Promise<StudentAssessmentAttemptContext> =>
          Promise.resolve({
            assessmentAttempt: "00000000-0000-0000-0000-000000000006",
            attemptNumber: ASSESSMENT.assessmentAttemptNumber ?? 1,
            displayTimeZone: ASSESSMENT_DECISION.displayTimeZone,
            expiresAt: null,
            timerRemainingMilliseconds: null,
            course: { ...COURSE_ONE, theme: "ocean" },
            assessment: { reference: ASSESSMENT.reference, title: ASSESSMENT.title },
          }),
        "m6-attempt-scope",
      ),
      assessmentAttemptHistory: query(
        (_reference: AssessmentAttemptRouteReference): Promise<StudentAssessmentAttemptHistory> =>
          Promise.resolve({
            assessmentAttempt: "00000000-0000-0000-0000-000000000005",
            attemptNumber: 1,
            course: { ...COURSE_ONE, theme: "ocean" },
            assessment: { reference: BONUS_ASSESSMENT.reference, title: BONUS_ASSESSMENT.title },
            state: "submitted",
            score: BONUS_ASSESSMENT.assessmentScore,
            questions: [],
          }),
        "m6-attempt-history",
      ),
    } satisfies RouteScopeQueries,
  } as unknown as ApplicationApi<OrdinaryBrowserApiClient>;
  const dispose = render(
    () => (
      <ApplicationApiProvider applicationApi={applicationApi}>
        <MemoryRouter history={history} root={HarnessRoot}>
          <Route path="/" component={StudentCoursesPage} />
          <Route path="/student" component={StudentCoursesPage} />
          <Route path="/student/courses/:courseRef" component={StudentCourseLandingPage} />
          <Route
            path="/courses/:courseRef/assessments/:assessmentRef"
            component={AssessmentOverviewPage}
          />
          <Route
            path="/assessment-attempts/:attemptRef"
            component={() => <p>Active Attempt destination</p>}
          />
          <Route
            path="/assessment-attempts/:attemptRef/summary"
            component={() => <p>Attempt summary destination</p>}
          />
        </MemoryRouter>
      </ApplicationApiProvider>
    ),
    target,
  );
  return { dispose, location: history.get };
}
