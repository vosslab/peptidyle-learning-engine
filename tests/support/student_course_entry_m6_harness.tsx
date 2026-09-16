// Compiled browser harness for the Student current-Course entry decisions.

import { MemoryRouter, Route, createMemoryHistory, useLocation } from "@solidjs/router";
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

type StudentCourseEntryCase = "zero" | "one" | "choose" | "many" | "landing";

const COURSE_ONE: LiveStudentCourseLandingSummary = {
  reference: "CI7K3M2Q",
  shortName: "BCHM 301",
  longName: "Biochemistry 301: Proteins and Peptides",
};
const COURSE_TWO: LiveStudentCourseLandingSummary = {
  reference: "CI4W8QF9",
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
  reference: "A9D2RX5",
  title: "Protein structure practice",
  assessmentType: "regular_assignment",
  decision: ASSESSMENT_DECISION,
  assessmentAttemptNumber: 2,
  assessmentAttemptCompletion: "inProgress",
  canResumeAssessmentAttempt: true,
  gradedQuestionCount: 1,
  questionCount: 4,
  assessmentScore: { pointsEarned: 7, pointsPossible: 8 },
};
const BONUS_ASSESSMENT: LiveStudentAssessmentLandingSummary = {
  ...ASSESSMENT,
  reference: "A4N8BQ2",
  title: "Bonus protein challenge",
  assessmentType: "bonus_assignment",
  assessmentAttemptNumber: 1,
  assessmentAttemptCompletion: "completed",
  canResumeAssessmentAttempt: false,
  gradedQuestionCount: 2,
  questionCount: 2,
  assessmentScore: { pointsEarned: 3, pointsPossible: 0 },
};
const WITHHELD_ASSESSMENT: LiveStudentAssessmentLandingSummary = {
  reference: "A7K2CW4",
  title: "Peptide quiz",
  assessmentType: "quiz",
  decision: ASSESSMENT_DECISION,
  assessmentAttemptNumber: 1,
  assessmentAttemptCompletion: "completed",
  canResumeAssessmentAttempt: false,
  gradedQuestionCount: 4,
  questionCount: 4,
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
        ? "/student/courses/CI7K3M2Q"
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
      listLiveStudentAssessments: () =>
        Promise.resolve(
          caseName === "landing"
            ? [currentAssessment(), BONUS_ASSESSMENT, WITHHELD_ASSESSMENT]
            : [],
        ),
      getLiveAssessmentAccess: () =>
        Promise.resolve({
          decision: currentDecision(),
          activeAssessmentAttempt: null,
          title: ASSESSMENT.title,
          assessmentType: ASSESSMENT.assessmentType,
          questionCount: ASSESSMENT.questionCount,
          pointsPossible: 8,
          previousAttempts: [],
        }),
    },
    queries: {
      courseScope: () =>
        Promise.resolve({
          summary: {
            reference: COURSE_ONE.reference,
            shortName: COURSE_ONE.shortName,
            longName: COURSE_ONE.longName,
            term: { startDate: "2026-08-31", endDate: "2026-12-12" },
            role: "student",
          },
          appearance: { theme: "ocean", banner: null },
        }),
    },
  } as unknown as ApplicationApi<OrdinaryBrowserApiClient>;
  const dispose = render(
    () => (
      <ApplicationApiProvider applicationApi={applicationApi}>
        <MemoryRouter history={history} root={HarnessRoot}>
          <Route path="/" component={StudentCoursesPage} />
          <Route path="/student/courses/:courseRef" component={StudentCourseLandingPage} />
          <Route
            path="/courses/:courseRef/assessments/:assessmentRef"
            component={AssessmentOverviewPage}
          />
        </MemoryRouter>
      </ApplicationApiProvider>
    ),
    target,
  );
  return { dispose, location: history.get };
}
