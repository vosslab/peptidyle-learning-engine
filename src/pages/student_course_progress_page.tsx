// Student-owned Course completion and released-score overview.

import { A, useParams } from "@solidjs/router";
import { createMemo, createResource, For, Show, type JSX } from "solid-js";

import type {
  LiveStudentCourseLandingSummary,
  StudentCourseProgressAssessment,
} from "../api/live_student_course_landing";
import { useApplicationApi } from "../api/application_api";
import { assessmentTypePresentation } from "../assessment_type_presentation";
import { PageFrame } from "../components/page_frame";
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

function ProgressRow(props: {
  readonly course: LiveStudentCourseLandingSummary;
  readonly assessment: StudentCourseProgressAssessment;
  readonly formatDateTime: () => ReturnType<typeof createDisplayDateTimeFormatter> | undefined;
}): JSX.Element {
  const latestActivityDateTime = (): string | undefined => {
    const timestamp = props.assessment.latestActivityAt;
    if (timestamp === null) return undefined;
    return props.formatDateTime()?.(timestamp);
  };

  return (
    <li class="student-course-progress__row">
      <div class="student-course-progress__identity">
        <h3>{props.assessment.title}</h3>
        <p>
          {props.assessment.assessmentAttemptCount} Attempt
          {props.assessment.assessmentAttemptCount === 1 ? "" : "s"}
          {props.assessment.submittedAssessmentAttemptCount > 0 &&
            ` * ${props.assessment.submittedAssessmentAttemptCount} submitted`}
        </p>
        <Show when={latestActivityDateTime()}>
          {(dateTime) => <p>Latest activity: {dateTime()}</p>}
        </Show>
      </div>
      <div class="student-course-progress__status">
        <strong>{studentAssessmentScoreStateLabel(props.assessment)}</strong>
        <p>{studentAssessmentActivityLabel(props.assessment)}</p>
        <p>{studentAssessmentScoreDescription(props.assessment)}</p>
      </div>
      <A
        class="quiet-link student-course-progress__action"
        href={assessmentPath(props.course.id, props.assessment.id)}
      >
        Open {assessmentTypePresentation(props.assessment.assessmentType).label}
      </A>
    </li>
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
              <Show when={assessments.loading}>
                <p class="loading-state">Loading Coursework progress...</p>
              </Show>
              <Show when={!assessments.loading && assessments.error === undefined}>
                <p class="student-course-progress__completion">
                  {completedStudentAssessmentCount(assessments() ?? [])} of{" "}
                  {assessments()?.length ?? 0} released Coursework items completed with at least one
                  submitted Attempt.
                </p>
                <p class="student-course-progress__disclosure">
                  Every released Coursework item remains listed. Attempts with no released score are
                  marked "Score not released"; completion and a perfect score are separate.
                </p>
                <Show when={assessments()?.length === 0}>
                  <p class="empty-state">No Coursework has been released for this Course yet.</p>
                </Show>
                <ul class="student-course-progress__list" aria-label="Coursework progress">
                  <For each={assessments()}>
                    {(assessment) => (
                      <ProgressRow
                        assessment={assessment}
                        course={current()}
                        formatDateTime={formatDateTime}
                      />
                    )}
                  </For>
                </ul>
              </Show>
              <Show when={assessments.error !== undefined}>
                <section class="route-error" role="alert">
                  <h3>Coursework progress unavailable</h3>
                  <p>Coursework progress could not be loaded right now.</p>
                </section>
              </Show>
            </section>
          </>
        )}
      </Show>
    </PageFrame>
  );
}
