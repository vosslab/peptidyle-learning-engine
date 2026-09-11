// Compiled browser harness for the Student current-Course entry decisions.

import { MemoryRouter, Route, createMemoryHistory, useLocation } from "@solidjs/router";
import type { JSX } from "solid-js";
import { render } from "solid-js/web";

import { ApplicationApiProvider, type ApplicationApi } from "../../src/api/application_api";
import type { OrdinaryBrowserApiClient } from "../../src/api/client";
import type { LiveStudentCourseLandingSummary } from "../../src/api/live_student_course_landing";
import { StudentCourseLandingPage } from "../../src/pages/student_course_landing_page";
import { StudentCoursesPage } from "../../src/pages/student_courses_page";

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
  return (
    <>
      <LocationProbe />
      {props.children}
    </>
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
  const applicationApi = {
    client: {
      listLiveStudentCourses: () => Promise.resolve(courses),
      listLiveStudentAssignments: () => Promise.resolve([]),
    },
  } as unknown as ApplicationApi<OrdinaryBrowserApiClient>;
  const dispose = render(
    () => (
      <ApplicationApiProvider applicationApi={applicationApi}>
        <MemoryRouter history={history} root={HarnessRoot}>
          <Route path="/" component={StudentCoursesPage} />
          <Route path="/student/courses/:courseRef" component={StudentCourseLandingPage} />
        </MemoryRouter>
      </ApplicationApiProvider>
    ),
    target,
  );
  return { dispose, location: history.get };
}
