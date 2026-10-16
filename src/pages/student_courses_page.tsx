// Student-owned current course index.

import { A } from "@solidjs/router";
import { createResource, type JSX } from "solid-js";

import type { LiveStudentCourseLandingSummary } from "../api/live_student_course_landing";
import { useApplicationApi } from "../api/application_api";
import { PageFrame } from "../components/page_frame";
import { SearchBlock } from "../features/search/search_block";
import { SearchControls } from "../features/search/search_controls";
import { createLocalCompleteSearch } from "../features/search/local_complete_search_state";
import { byStudentCourseNameThenId, studentCourseFields } from "./student_collection_fields";
import { type RecordContent } from "../components/record_list/record_list";

function courseContent(course: LiveStudentCourseLandingSummary): RecordContent {
  return {
    title: course.longName,
    details: [],
    actions: [
      {
        id: "open-course",
        kind: "link",
        label: "Open Course",
        href: `/student/courses/${course.id}`,
        primary: true,
      },
    ],
  };
}

/** Lists the signed-in Student's current Courses and invitations. */
export function StudentCoursesPage(): JSX.Element {
  const applicationApi = useApplicationApi();
  const [courses, { refetch }] = createResource(async () =>
    byStudentCourseNameThenId(await applicationApi.client.listLiveStudentCourses()),
  );
  const courseSearch = createLocalCompleteSearch(
    { resource: courses, refetchSource: refetch, rows: (value) => value },
    {
      fields: studentCourseFields,
      rowId: (course) => course.id,
      keyword: (course) => `${course.longName} ${course.shortName}`,
      content: courseContent,
    },
  );

  return (
    <PageFrame
      routeSurface="studentCourses"
      actions={
        <A class="quiet-link" href="/student/course-invitations">
          Course invitations
        </A>
      }
    >
      <SearchBlock
        layout="above"
        controls={<SearchControls state={courseSearch.state} textLabel="Search your courses" />}
        state={courseSearch.state}
        collection={{
          ariaLabel: "Your courses",
          fields: studentCourseFields,
          query: courseSearch.query,
          get emptyState() {
            return {
              title:
                courseSearch.state.query().text || courseSearch.state.query().filters.size
                  ? "No courses match your search."
                  : "You do not have any current courses.",
            };
          },
        }}
      />
    </PageFrame>
  );
}
