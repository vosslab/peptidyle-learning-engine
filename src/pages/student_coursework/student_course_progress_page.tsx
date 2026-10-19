// Student-owned Course completion and released-score overview.

import { A, useParams } from "@solidjs/router";
import { createMemo, createResource, Show, type Accessor, type JSX } from "solid-js";

import type {
  LiveStudentCourseLandingSummary,
  StudentCourseProgressAssessment,
} from "../../api/live_student_course_landing";
import { useApplicationApi } from "../../api/application_api";
import { assessmentTypePresentation } from "../../assessment_type_presentation";
import { PageFrame } from "../../components/page_frame/page_frame";
import { SearchBlock } from "../../components/search_block/search_block";
import { SearchControls } from "../../components/search_block/search_controls";
import { createLocalCompleteSearch } from "../../components/search_block/local_complete_search_state";
import { byStudentTitleThenId, studentProgressFields } from "./student_collection_fields";
import type { RecordContent, RecordFact } from "../../components/data_sheet/record_content";
import { CourseEntryBanner } from "../../features/course_appearance/course_entry_banner";
import { parseCourseInstanceId } from "../../navigation/public_route";
import { buildRoutePath } from "../../ribbon/ribbon_route_path";
import { createDisplayDateTimeFormatter } from "../../format_datetime";
import "./student_course_progress_page.css";
import {
  completedStudentAssessmentCount,
  studentAssessmentActivityLabel,
  studentAssessmentScoreDescription,
  studentAssessmentScoreStateLabel,
} from "../../student_course_progress_presentation";

function assessmentPath(courseInstanceId: string, assessmentId: string): string {
  const path = buildRoutePath("assessmentOverview", { courseInstanceId, assessmentId });
  if (path === undefined) {
    throw new Error("Student Progress requires canonical Course and Assessment IDs.");
  }
  return path;
}

function progressContent(
  course: LiveStudentCourseLandingSummary,
  formatDateTime: () => ReturnType<typeof createDisplayDateTimeFormatter> | undefined,
): (assessment: StudentCourseProgressAssessment) => RecordContent {
  const latestActivityDateTime = (
    assessment: StudentCourseProgressAssessment,
  ): string | undefined => {
    const timestamp = assessment.latestActivityAt;
    if (timestamp === null) return undefined;
    const format = formatDateTime();
    return format?.(timestamp);
  };
  return (assessment): RecordContent => {
    const details: Array<RecordFact> = [
      { kind: "assessmentType", value: assessment.assessmentType },
      {
        kind: "text",
        label: "Attempts:",
        value: `${assessment.assessmentAttemptCount} Attempt${assessment.assessmentAttemptCount === 1 ? "" : "s"}${
          assessment.submittedAssessmentAttemptCount > 0
            ? ` * ${assessment.submittedAssessmentAttemptCount} submitted`
            : ""
        }`,
      },
    ];
    const latestActivity = latestActivityDateTime(assessment);
    if (latestActivity !== undefined) {
      details.push({ kind: "text", label: "Latest activity:", value: latestActivity });
    }
    details.push(
      { kind: "text", label: "Score:", value: studentAssessmentScoreStateLabel(assessment) },
      { kind: "text", label: "Activity:", value: studentAssessmentActivityLabel(assessment) },
      { kind: "text", value: studentAssessmentScoreDescription(assessment) },
    );
    return {
      title: assessment.title,
      details,
      actions: [
        {
          kind: "link",
          id: "open-assessment",
          label: `Open ${assessmentTypePresentation(assessment.assessmentType).label}`,
          href: assessmentPath(course.id, assessment.id),
          primary: true,
        },
      ],
    };
  };
}

function StudentCourseProgressCollection(props: {
  readonly course: LiveStudentCourseLandingSummary;
  readonly formatDateTime: Accessor<ReturnType<typeof createDisplayDateTimeFormatter> | undefined>;
}): JSX.Element {
  const applicationApi = useApplicationApi();
  const [assessments, { refetch }] = createResource(
    () => props.course.id,
    async (courseId) =>
      byStudentTitleThenId(await applicationApi.client.getStudentCourseProgress(courseId)),
  );
  const progressSearch = createLocalCompleteSearch(
    {
      rows: assessments,
      loading: () => assessments.loading,
      error: (): unknown => assessments.error,
      refetchSource: refetch,
    },
    {
      fields: studentProgressFields,
      rowId: (assessment) => assessment.id,
      keyword: (assessment) => `${assessment.title} ${assessment.assessmentType}`,
      content: (assessment) => {
        return progressContent(props.course, props.formatDateTime)(assessment);
      },
    },
  );

  return (
    <section
      class="student-course-progress"
      aria-label={`${props.course.shortName} Course Progress`}
    >
      <p class="student-course-progress__completion">
        {completedStudentAssessmentCount(assessments() ?? [])} of {assessments()?.length ?? 0}{" "}
        released Coursework items completed with at least one submitted Attempt.
      </p>
      <p class="student-course-progress__disclosure">
        "Score pending" means automatic grading is still running. Completion and a perfect score are
        separate.
      </p>
      <SearchBlock
        layout="above"
        controls={
          <SearchControls state={progressSearch.state} textLabel="Search Coursework progress" />
        }
        state={progressSearch.state}
        collection={{
          ariaLabel: "Coursework progress",
          ...progressSearch.collection,
          get emptyState() {
            return {
              title:
                progressSearch.state.query().text || progressSearch.state.query().filters.size
                  ? "No Coursework progress matches your search."
                  : "No Coursework has been released for this Course yet.",
            };
          },
        }}
      />
    </section>
  );
}

/** Shows every released Assessment, including those with Attempts but no released score. */
export function StudentCourseProgressPage(): JSX.Element {
  const applicationApi = useApplicationApi();
  const params = useParams();
  function courseInstanceId(): ReturnType<typeof parseCourseInstanceId> {
    return parseCourseInstanceId(params["courseInstanceId"] ?? "");
  }
  const [courses] = createResource(() => applicationApi.client.listLiveStudentCourses());
  const [accountSettings] = createResource(() => applicationApi.client.getAccountSettings());
  const formatDateTime = createMemo(() => {
    const timeZone = accountSettings()?.timeZone;
    if (timeZone !== undefined) return createDisplayDateTimeFormatter(timeZone);
    return accountSettings.error !== undefined ? (): string => "Time unavailable" : undefined;
  });
  const course = createMemo(() => {
    const id = courseInstanceId();
    return id === null ? undefined : courses()?.find((candidate) => candidate.id === id);
  });
  const unavailable = (): boolean => courseInstanceId() === null || courses.error !== undefined;

  return (
    <PageFrame routeSurface="studentCourseProgress">
      <Show when={courses.loading}>
        <p class="loading-state">Loading Course Progress...</p>
      </Show>
      <Show when={unavailable() || (!courses.loading && course() === undefined)}>
        <section class="route-error" role="alert">
          <h2>Course Progress unavailable</h2>
          <p>This Course Progress is not available.</p>
          <A class="primary-link" href="/student/courses">
            Return to Courses
          </A>
        </section>
      </Show>
      <Show keyed when={!unavailable() ? course() : undefined}>
        {(current) => (
          <>
            <CourseEntryBanner />
            <A class="quiet-link" href="/student/courses">
              Your courses
            </A>
            <StudentCourseProgressCollection course={current} formatDateTime={formatDateTime} />
          </>
        )}
      </Show>
    </PageFrame>
  );
}
