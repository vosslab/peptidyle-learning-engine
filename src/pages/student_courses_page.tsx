// Student-owned current course index.

import { A } from "@solidjs/router";
import { createResource, type JSX } from "solid-js";

import type { LiveStudentCourseLandingSummary } from "../api/live_student_course_landing";
import { useApplicationApi } from "../api/application_api";
import { PageFrame } from "../components/page_frame";
import { RecordList, type RecordListState } from "../components/record_list/record_list";
import type { RecordRegion } from "../components/record_list/region_spec";

function courseRegions(): ReadonlyArray<RecordRegion<LiveStudentCourseLandingSummary>> {
  return [
    {
      id: "course",
      role: "identity",
      priority: "required",
      width: "minmax(0, 1fr)",
      align: "start",
      content: (course): JSX.Element => <h2>{course.longName}</h2>,
    },
    {
      id: "action",
      role: "actions",
      priority: "required",
      width: "auto",
      align: "end",
      content: (course): JSX.Element => (
        <A class="primary-link" href={`/student/courses/${course.id}`}>
          Open Course
        </A>
      ),
    },
  ];
}

function courseListState(loading: boolean, unavailable: boolean): RecordListState {
  if (loading) return { kind: "loading", label: "Loading your courses..." };
  if (unavailable) {
    return {
      kind: "error",
      title: "Your courses are unavailable",
      message: "Your courses are not available right now.",
    };
  }
  return { kind: "ready" };
}

/** Lists the signed-in Student's current Courses and invitations. */
export function StudentCoursesPage(): JSX.Element {
  const applicationApi = useApplicationApi();
  const [courses] = createResource(() => applicationApi.client.listLiveStudentCourses());

  return (
    <PageFrame
      routeSurface="studentCourses"
      eyebrow="Your learning"
      title="Your courses"
      lede="Open one of your Courses. Coursework and Grades include all of your current Courses."
    >
      <A class="quiet-link" href="/student/course-invitations">
        Course invitations
      </A>
      <RecordList
        ariaLabel="Your courses"
        emptyState={{ title: "You do not have any current courses." }}
        recordId={(course) => course.id}
        regions={courseRegions()}
        rows={courses() ?? []}
        state={courseListState(courses.loading, courses.error !== undefined)}
      />
    </PageFrame>
  );
}
