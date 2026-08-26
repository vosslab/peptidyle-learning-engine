// curriculum_adoption_live_page.tsx - course-route composition for B2 live curriculum adoption.

import { revalidate } from "@solidjs/router";
import { Show, createMemo, type JSX } from "solid-js";

import type { CourseReference } from "../../generated/api/CourseReference";

import { useApiRuntime } from "../api/runtime";
import { CourseManagementNav } from "../components/course_management_nav";
import { CurriculumAdoptionPage } from "../features/curriculum_adoption";
import { resolveCourseRoute } from "../navigation/resolved_route";
import {
  courseRouteData,
  useCourseThemeRouteData,
} from "../features/course_appearance/course_theme_context";

/** Uses the already-authorized course route projection; B2 adds no client-supplied authority. */
export function CurriculumAdoptionLivePage(): JSX.Element {
  const runtime = useApiRuntime();
  const scopedRoute = useCourseThemeRouteData();
  const course = scopedRoute?.kind === "course" ? courseRouteData(scopedRoute).summary : undefined;
  const instructorCourse = createMemo(() =>
    course !== undefined && course.role === "instructor" ? course : undefined,
  );

  async function refreshChangedCourse(changed: CourseReference): Promise<void> {
    await revalidate(runtime.queries.courses.key);
    const changedId = await resolveCourseRoute(runtime.client, changed);
    await Promise.all([
      revalidate(runtime.queries.courseScope.keyFor(changedId)),
      revalidate(runtime.queries.assignments.keyFor(changedId)),
    ]);
  }

  return (
    <Show
      when={instructorCourse()}
      fallback={
        <main class="page" data-route-surface="curriculumAdoption">
          <p role="alert">
            Curriculum adoption is available to an Instructor in the selected course.
          </p>
        </main>
      }
    >
      {(currentCourse) => (
        <>
          <CourseManagementNav
            courseReference={currentCourse().reference}
            active="curriculumAdoption"
          />
          <CurriculumAdoptionPage
            course={currentCourse()}
            client={runtime.client}
            reusableClient={runtime.client}
            onCourseChanged={refreshChangedCourse}
          />
        </>
      )}
    </Show>
  );
}
