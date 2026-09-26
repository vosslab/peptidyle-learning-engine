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
import {
  RecordList,
  type RecordContent,
  type RecordFact,
  type RecordListState,
} from "../components/record_list/record_list";
import { CourseEntryBanner } from "../features/course_appearance/course_entry_banner";
import { parseCourseInstanceId } from "../navigation/public_route";
import { buildRoutePath } from "../ribbon/ribbon_contract";
import { createDisplayDateTimeFormatter } from "../format_datetime";
import "./student_course_progress_page.css";
import {
  completedStudentAssessmentCount,
  studentAssessmentActivityLabel,
  studentAssessmentScoreDescription,
  studentAssessmentScoreStateLabel,
} from "../student_course_progress_presentation";

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
    return formatDateTime()?.(timestamp);
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
                content={progressContent(current(), formatDateTime)}
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
