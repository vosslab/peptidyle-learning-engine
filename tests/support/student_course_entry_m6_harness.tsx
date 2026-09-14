// Compiled browser harness for the Student current-Course entry decisions.

import { MemoryRouter, Route, createMemoryHistory, useLocation } from "@solidjs/router";
import type { JSX } from "solid-js";
import { render } from "solid-js/web";

import type { StudentAssignmentDecisionSummary } from "../../generated/api/StudentAssignmentDecisionSummary";
import { ApplicationApiProvider, type ApplicationApi } from "../../src/api/application_api";
import type { OrdinaryBrowserApiClient } from "../../src/api/client";
import type {
  LiveStudentAssignmentLandingSummary,
  LiveStudentCourseLandingSummary,
} from "../../src/api/live_student_course_landing";
import { AssignmentOverviewPage } from "../../src/pages/assignment_overview_page";
import { StudentCourseLandingPage } from "../../src/pages/student_course_landing_page";
import { StudentCoursesPage } from "../../src/pages/student_courses_page";
import { RouteScopeProvider } from "../../src/ribbon/route_scope_context";

type StudentCourseEntryCase = "zero" | "one" | "choose" | "many" | "landing";

const COURSE_ONE: LiveStudentCourseLandingSummary = {
  reference: "C-1",
  shortName: "BCHM 301",
  longName: "Biochemistry 301: Proteins and Peptides",
};
const COURSE_TWO: LiveStudentCourseLandingSummary = {
  reference: "C-2",
  shortName: "BIOL 302",
  longName: "Molecular Genetics: Gene Regulation",
};
const ASSIGNMENT_DECISION = {
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
const ASSIGNMENT: LiveStudentAssignmentLandingSummary = {
  reference: "A-1",
  title: "Protein structure practice",
  decision: ASSIGNMENT_DECISION,
  assignmentAttemptNumber: null,
  assignmentAttemptCompletion: null,
  gradedQuestionCount: 0,
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
      caseName === "landing" ? "/student/courses/C-1" : caseName === "choose" ? "/?choose=1" : "/",
  });
  const courses = coursesFor(caseName);
  let studentTimeZone: string = ASSIGNMENT_DECISION.displayTimeZone;
  const currentDecision = (): StudentAssignmentDecisionSummary => ({
    ...ASSIGNMENT_DECISION,
    displayTimeZone: studentTimeZone,
  });
  const currentAssignment = (): LiveStudentAssignmentLandingSummary => ({
    ...ASSIGNMENT,
    decision: currentDecision(),
  });
  const applicationApi = {
    client: {
      listLiveStudentCourses: () => Promise.resolve(courses),
      listLiveStudentAssignments: () =>
        Promise.resolve(caseName === "landing" ? [currentAssignment()] : []),
      getStudentTimeZoneProfile: () => Promise.resolve({ timeZone: studentTimeZone }),
      updateStudentTimeZone: (input: { readonly timeZone: string }) => {
        studentTimeZone = input.timeZone;
        return Promise.resolve(input);
      },
      getLiveAssignmentAccess: () =>
        Promise.resolve({
          decision: currentDecision(),
          activeAssignmentAttempt: null,
          title: ASSIGNMENT.title,
          questionCount: ASSIGNMENT.questionCount,
          pointsPossible: 8,
          previousAttempts: [],
        }),
    },
    queries: {
      resolveCourse: () => Promise.resolve({ courseId: "00000000-0000-0000-0000-000000000001" }),
      courseScope: () =>
        Promise.resolve({
          summary: {
            id: "00000000-0000-0000-0000-000000000001",
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
            path="/courses/:courseRef/assignments/:assignmentRef"
            component={AssignmentOverviewPage}
          />
        </MemoryRouter>
      </ApplicationApiProvider>
    ),
    target,
  );
  return { dispose, location: history.get };
}
