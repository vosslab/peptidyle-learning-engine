// Student-owned Course completion and released-score overview.

import { A, useParams } from "@solidjs/router";
import { createMemo, createResource, Show, type JSX } from "solid-js";

import type {
  LiveStudentCourseLandingSummary,
  StudentCourseProgressAssessment,
} from "../api/live_student_course_landing";
import { useApplicationApi } from "../api/application_api";
import { assessmentTypePresentation } from "../assessment_type_presentation";
import { PageFrame } from "../components/page_frame";
import { RecordList, type RecordListState } from "../components/record_list/record_list";
import type { RecordRegion } from "../components/record_list/region_spec";
import { CourseEntryBanner } from "../features/course_appearance/course_entry_banner";
import { parseCourseInstanceId } from "../navigation/public_route";
import { buildRoutePath } from "../ribbon/ribbon_contract";
import { createDisplayDateTimeFormatter } from "../format_datetime";
import {
  completedStudentAssessmentCount,
  studentAssessmentActivityLabel,
  studentAssessmentScoreDescription,
  studentAssessmentScoreStateLabel,
} from "../student_course_progress_presentation";
import "./student_course_progress_page.css";

function assessmentPath(courseInstanceId: string, assessmentId: string): string {
  const path = buildRoutePath("assessmentOverview", { courseInstanceId, assessmentId });
  if (path === undefined) {
    throw new Error("Student Progress requires canonical Course and Assessment IDs.");
  }
  return path;
}

function progressRegions(
  course: LiveStudentCourseLandingSummary,
  formatDateTime: () => ReturnType<typeof createDisplayDateTimeFormatter> | undefined,
): ReadonlyArray<RecordRegion<StudentCourseProgressAssessment>> {
  const latestActivityDateTime = (
    assessment: StudentCourseProgressAssessment,
  ): string | undefined => {
    const timestamp = assessment.latestActivityAt;
    if (timestamp === null) return undefined;
    return formatDateTime()?.(timestamp);
  };
  return [
    {
      id: "assessment",
      role: "identity",
      priority: "required",
      width: "minmax(12rem, 1fr)",
      align: "start",
      content: (assessment): JSX.Element => (
        <div class="student-course-progress__identity">
          <h3>{assessment.title}</h3>
          <p>
            {assessment.assessmentAttemptCount} Attempt
            {assessment.assessmentAttemptCount === 1 ? "" : "s"}
            {assessment.submittedAssessmentAttemptCount > 0 &&
              ` * ${assessment.submittedAssessmentAttemptCount} submitted`}
          </p>
          <Show when={latestActivityDateTime(assessment)}>
            {(dateTime) => <p>Latest activity: {dateTime()}</p>}
          </Show>
        </div>
      ),
    },
    {
      id: "status",
      role: "status",
      priority: "high",
      width: "minmax(16rem, 1fr)",
      align: "start",
      content: (assessment): JSX.Element => (
        <div class="student-course-progress__status">
          <strong>{studentAssessmentScoreStateLabel(assessment)}</strong>
          <p>{studentAssessmentActivityLabel(assessment)}</p>
          <p>{studentAssessmentScoreDescription(assessment)}</p>
        </div>
      ),
    },
    {
      id: "action",
      role: "actions",
      priority: "required",
      width: "auto",
      align: "end",
      content: (assessment): JSX.Element => (
        <A
          class="quiet-link student-course-progress__action"
          href={assessmentPath(course.id, assessment.id)}
        >
          Open {assessmentTypePresentation(assessment.assessmentType).label}
        </A>
      ),
    },
  ];
}

function progressListState(loading: boolean, unavailable: boolean): RecordListState {
  if (loading) return { kind: "loading", label: "Loading Coursework progress..." };
  if (unavailable) {
    return {
      kind: "error",
      title: "Coursework progress unavailable",
      message: "Coursework progress could not be loaded right now.",
    };
  }
  return { kind: "ready" };
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
    return timeZone === undefined ? undefined : createDisplayDateTimeFormatter(timeZone);
  });
  const course = createMemo(() => {
    const id = courseInstanceId();
    return id === null ? undefined : courses()?.find((candidate) => candidate.id === id);
  });
  const [assessments] = createResource(course, (current) =>
    applicationApi.client.getStudentCourseProgress(current.id),
  );
  const unavailable = (): boolean =>
    courseInstanceId() === null || courses.error !== undefined || assessments.error !== undefined;

  return (
    <PageFrame routeSurface="studentCourseProgress" title={course()?.longName ?? "Course Progress"}>
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
      <Show when={!unavailable() ? course() : undefined}>
        {(current) => (
          <>
            <CourseEntryBanner />
            <A class="quiet-link" href="/student/courses">
              Your courses
            </A>
            <section
              class="student-course-progress"
              aria-labelledby="student-course-progress-heading"
            >
              <h2 id="student-course-progress-heading">Course Progress</h2>
              <p class="student-course-progress__completion">
                {completedStudentAssessmentCount(assessments() ?? [])} of{" "}
                {assessments()?.length ?? 0} released Coursework items completed with at least one
                submitted Attempt.
              </p>
              <p class="student-course-progress__disclosure">
                Every released Coursework item remains listed. Attempts with no released score are
                marked "Score not released"; completion and a perfect score are separate.
              </p>
              <RecordList
                ariaLabel="Coursework progress"
                emptyState={{ title: "No Coursework has been released for this Course yet." }}
                recordId={(assessment) => assessment.id}
                regions={progressRegions(current(), formatDateTime)}
                rows={assessments() ?? []}
                state={progressListState(assessments.loading, assessments.error !== undefined)}
              />
            </section>
          </>
        )}
      </Show>
    </PageFrame>
  );
}
