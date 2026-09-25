// Student-owned answer-free course and available assessment landing.

import { A, useParams } from "@solidjs/router";
import { createMemo, createResource, For, Show, type JSX } from "solid-js";

import type {
  LiveStudentAssessmentLandingSummary,
  LiveStudentCourseLandingSummary,
  StudentCourseProgressAssessment,
} from "../api/live_student_course_landing";
import { assessmentTypePresentation } from "../assessment_type_presentation";
import { useApplicationApi } from "../api/application_api";
import { PageFrame } from "../components/page_frame";
import { RecordList, type RecordListState } from "../components/record_list/record_list";
import type { RecordRegion } from "../components/record_list/region_spec";
import { formatAssessmentDeliveryTime } from "../components/student_assessment_presentation";
import { CourseEntryBanner } from "../features/course_appearance/course_entry_banner";
import { createDisplayDateTimeFormatter } from "../format_datetime";
import { parseCourseInstanceId } from "../navigation/public_route";
import { buildRoutePath } from "../ribbon/ribbon_contract";
import { RibbonIcon } from "../ribbon/ribbon_icon";
import { studentCourseworkDisplay } from "./student_coursework_presentation";
import {
  hasSubmittedStudentAttempt,
  isInStudentDueSoonWindow,
} from "./student_coursework_presentation";
import "./student_course_landing_page.css";

function assessmentDisplay(
  assessment: LiveStudentAssessmentLandingSummary,
): ReturnType<typeof studentCourseworkDisplay> {
  const display = studentCourseworkDisplay(
    assessment.decision.startDecision,
    assessment.assessmentAttemptCompletion,
    assessment.canResumeAssessmentAttempt,
  );
  return display;
}

function assessmentOverviewPath(courseInstanceId: string, assessmentId: string): string {
  const path = buildRoutePath("assessmentOverview", { courseInstanceId, assessmentId });
  if (path === undefined) {
    throw new Error(
      "Student Assessment overview route requires canonical Course and Assessment IDs.",
    );
  }
  return path;
}

function assessmentAccessLabel(assessment: LiveStudentAssessmentLandingSummary): string {
  const access = assessment.decision.startDecision === "may_start" ? "Can start" : "Cannot start";
  const reason = assessment.decision.publicReason;
  if (reason === null) return access;
  return `${access}: ${reason}`;
}

function assessmentRegions(
  course: LiveStudentCourseLandingSummary,
  formatDateTime: ReturnType<typeof createDisplayDateTimeFormatter>,
  view: StudentCourseworkView,
  completedAssessmentIds: ReadonlySet<string>,
): ReadonlyArray<RecordRegion<LiveStudentAssessmentLandingSummary>> {
  return [
    {
      id: "assessment",
      role: "identity",
      priority: "required",
      width: "minmax(0, 1.3fr)",
      align: "stretch",
      content: (assessment): JSX.Element => {
        const type = assessmentTypePresentation(assessment.assessmentType);
        const display = assessmentDisplay(assessment);
        const due = formatAssessmentDeliveryTime(
          assessment.decision.dueAt,
          formatDateTime,
          "No due time",
        );
        return (
          <>
            <h3 class="student-coursework__title">{assessment.title}</h3>
            <p
              class="student-coursework__type"
              style={`--student-coursework-type-color: var(${type.colorToken})`}
            >
              <RibbonIcon glyph={type.icon} />
              <span>{type.label}</span>
            </p>
            <p class="student-coursework__facts">
              <span>{assessmentAccessLabel(assessment)}</span>
              <span>
                Due <span data-assessment-decision-due>{due}</span>
              </span>
              <span>
                {view === "completed" && completedAssessmentIds.has(assessment.id)
                  ? "Submitted Attempt on record"
                  : display.completionLabel}
              </span>
            </p>
          </>
        );
      },
    },
    {
      id: "action",
      role: "actions",
      priority: "required",
      width: "auto",
      align: "end",
      content: (assessment): JSX.Element => {
        const display = assessmentDisplay(assessment);
        const type = assessmentTypePresentation(assessment.assessmentType);
        return (
          <A
            class="primary-link student-coursework__action"
            href={assessmentOverviewPath(course.id, assessment.id)}
            aria-label={`${display.actionVerb} ${type.label}`}
          >
            {display.actionVerb}
          </A>
        );
      },
    },
  ];
}

function assessmentListState(loading: boolean): RecordListState {
  if (loading) return { kind: "loading", label: "Loading Coursework..." };
  return { kind: "ready" };
}

function AssessmentList(props: {
  readonly course: LiveStudentCourseLandingSummary;
  readonly assessments: ReadonlyArray<LiveStudentAssessmentLandingSummary>;
  readonly loading: boolean;
  readonly view: StudentCourseworkView;
  readonly completedAssessmentIds: ReadonlySet<string>;
}): JSX.Element {
  const formatDateTime = createMemo(() => {
    const timeZone = props.assessments[0]?.decision.displayTimeZone;
    return timeZone === undefined ? undefined : createDisplayDateTimeFormatter(timeZone);
  });
  const regions = createMemo(() => {
    const formatter = formatDateTime();
    return formatter === undefined
      ? []
      : assessmentRegions(props.course, formatter, props.view, props.completedAssessmentIds);
  });

  return (
    <RecordList
      ariaLabel={courseworkHeading(props.view)}
      emptyState={{ title: emptyCourseworkMessage(props.view) }}
      recordId={(assessment) => assessment.id}
      regions={regions()}
      rows={props.assessments}
      state={assessmentListState(props.loading)}
    />
  );
}

export type StudentCourseworkView = "all" | "dueSoon" | "completed";

function courseworkHeading(view: StudentCourseworkView): string {
  switch (view) {
    case "all":
      return "Available Coursework";
    case "dueSoon":
      return "Due Soon";
    case "completed":
      return "Completed Coursework";
  }
}

function emptyCourseworkMessage(view: StudentCourseworkView): string {
  switch (view) {
    case "all":
      return "No Coursework is available right now.";
    case "dueSoon":
      return "No Coursework is due in the next 7 days.";
    case "completed":
      return "No Coursework has a submitted Attempt yet.";
  }
}

/** Student-owned entry point for answer-free current Course work. */
export function StudentCourseLandingPage(): JSX.Element {
  return <StudentCourseworkPage view="all" />;
}

export function StudentDueSoonPage(): JSX.Element {
  return <StudentCourseworkAcrossCoursesPage view="dueSoon" />;
}

export function StudentCompletedPage(): JSX.Element {
  return <StudentCourseworkAcrossCoursesPage view="completed" />;
}

export function StudentAllCourseworkPage(): JSX.Element {
  return <StudentCourseworkAcrossCoursesPage view="all" />;
}

function StudentCourseworkCourseSection(props: {
  readonly course: LiveStudentCourseLandingSummary;
  readonly view: StudentCourseworkView;
}): JSX.Element {
  const api = useApplicationApi();
  const [assessments] = createResource(
    () => props.course,
    (course) => api.client.listLiveStudentAssessments(course.id),
  );
  const [progress] = createResource(
    () => (props.view === "completed" ? props.course : undefined),
    (course) => api.client.getStudentCourseProgress(course.id),
  );
  const completedAssessmentIds = createMemo<ReadonlySet<string>>(() => {
    const submitted = (progress() ?? [])
      .filter((assessment) =>
        hasSubmittedStudentAttempt(assessment.submittedAssessmentAttemptCount),
      )
      .map((assessment) => assessment.id);
    return new Set(submitted);
  });
  const visibleAssessments = createMemo(() => {
    const rows = assessments() ?? [];
    if (props.view === "dueSoon") {
      return rows.filter((assessment) =>
        isInStudentDueSoonWindow(assessment.decision.dueAt, assessment.decision.evaluatedAt),
      );
    }
    if (props.view === "completed") {
      return rows.filter((assessment) => completedAssessmentIds().has(assessment.id));
    }
    return rows;
  });
  const loading = (): boolean =>
    assessments.loading || (props.view === "completed" && progress.loading);
  const unavailable = (): boolean =>
    assessments.error !== undefined || (props.view === "completed" && progress.error !== undefined);

  return (
    <section class="student-coursework" aria-label={`${props.course.shortName} Coursework`}>
      <h2>
        {props.course.shortName}: {props.course.longName}
      </h2>
      <Show when={unavailable()}>
        <p class="route-error" role="alert">
          Coursework could not be loaded for this Course.
        </p>
      </Show>
      <Show when={!unavailable()}>
        <AssessmentList
          assessments={visibleAssessments()}
          course={props.course}
          loading={loading()}
          view={props.view}
          completedAssessmentIds={completedAssessmentIds()}
        />
      </Show>
    </section>
  );
}

/** Shows the selected Coursework view across every currently enrolled Course. */
function StudentCourseworkAcrossCoursesPage(props: {
  readonly view: StudentCourseworkView;
}): JSX.Element {
  const api = useApplicationApi();
  const [courses] = createResource(() => api.client.listLiveStudentCourses());
  const title = props.view === "all" ? "All Coursework" : courseworkHeading(props.view);

  return (
    <PageFrame
      routeSurface={
        props.view === "all"
          ? "studentHome"
          : props.view === "dueSoon"
            ? "studentDueSoon"
            : "studentCompleted"
      }
      title={title}
    >
      <p>
        {props.view === "dueSoon"
          ? "Coursework with due dates in the next 7 days."
          : props.view === "completed"
            ? "Each listed Coursework item has at least one submitted Attempt. A newer Attempt may still be in progress."
            : "Coursework from all of your current Courses."}
      </p>
      <Show when={courses.loading}>
        <p class="loading-state">Loading Coursework...</p>
      </Show>
      <Show when={courses.error !== undefined}>
        <section class="route-error" role="alert">
          <h2>Coursework unavailable</h2>
          <p>Coursework could not be loaded right now.</p>
        </section>
      </Show>
      <Show when={!courses.loading && courses.error === undefined && courses()?.length === 0}>
        <p class="empty-state">No current Courses have Coursework available.</p>
      </Show>
      <For each={courses()}>
        {(course) => <StudentCourseworkCourseSection course={course} view={props.view} />}
      </For>
    </PageFrame>
  );
}

function StudentCourseworkPage(props: { readonly view: StudentCourseworkView }): JSX.Element {
  const applicationApi = useApplicationApi();
  const params = useParams();
  function courseInstanceId(): ReturnType<typeof parseCourseInstanceId> {
    return parseCourseInstanceId(params["courseInstanceId"] ?? "");
  }
  async function loadCourses(): Promise<ReadonlyArray<LiveStudentCourseLandingSummary>> {
    return applicationApi.client.listLiveStudentCourses();
  }
  const [courses] = createResource(loadCourses);
  const course = createMemo(() => {
    const id = courseInstanceId();
    if (id === null) return undefined;
    return courses()?.find((candidate) => candidate.id === id);
  });
  // ASVS V2.2.2/2.3.1: the server projects Student membership; this view makes no access decision.
  async function loadAssessments(
    current: LiveStudentCourseLandingSummary,
  ): Promise<ReadonlyArray<LiveStudentAssessmentLandingSummary>> {
    return applicationApi.client.listLiveStudentAssessments(current.id);
  }
  const [assessments] = createResource(course, loadAssessments);
  const [progress] = createResource(
    () => (props.view === "completed" ? course() : undefined),
    (current) => applicationApi.client.getStudentCourseProgress(current.id),
  );
  const completedAssessmentIds = createMemo<ReadonlySet<string>>(() => {
    const submitted = (progress() ?? [])
      .filter((assessment: StudentCourseProgressAssessment) =>
        hasSubmittedStudentAttempt(assessment.submittedAssessmentAttemptCount),
      )
      .map((assessment: StudentCourseProgressAssessment) => assessment.id);
    return new Set(submitted);
  });
  const visibleAssessments = createMemo(() => {
    const rows = assessments() ?? [];
    if (props.view === "dueSoon") {
      return rows.filter((assessment) =>
        isInStudentDueSoonWindow(assessment.decision.dueAt, assessment.decision.evaluatedAt),
      );
    }
    if (props.view === "completed") {
      return rows.filter((assessment) => completedAssessmentIds().has(assessment.id));
    }
    return rows;
  });
  function unavailable(): boolean {
    return (
      courseInstanceId() === null ||
      courses.error !== undefined ||
      assessments.error !== undefined ||
      (props.view === "completed" && progress.error !== undefined)
    );
  }

  return (
    <PageFrame
      routeSurface={
        props.view === "all"
          ? "studentCourseLanding"
          : props.view === "dueSoon"
            ? "studentDueSoon"
            : "studentCompleted"
      }
      title={course()?.longName ?? "Coursework"}
    >
      <Show when={courses.loading}>
        <p class="loading-state">Loading assigned work...</p>
      </Show>
      <Show when={unavailable()}>
        <section class="route-error" role="alert">
          <h2>Assigned work unavailable</h2>
          <p>This assigned work is not available.</p>
          <A class="primary-link" href="/student">
            Return to Coursework
          </A>
        </section>
      </Show>
      <Show when={!courses.loading && !unavailable() && course() === undefined}>
        <section class="route-error" role="alert">
          <h2>Assigned work unavailable</h2>
          <p>This assigned work is not available.</p>
          <A class="primary-link" href="/student">
            Return to Coursework
          </A>
        </section>
      </Show>
      <Show when={!unavailable() ? course() : undefined}>
        {(current) => (
          <>
            <CourseEntryBanner />
            <A class="quiet-link" href="/student/courses">
              Your Courses
            </A>
            <A class="quiet-link" href={`/student/courses/${current().id}/progress`}>
              Course Progress
            </A>
            <section class="student-coursework" aria-labelledby="student-coursework-heading">
              <h2 id="student-coursework-heading">{courseworkHeading(props.view)}</h2>
              <Show when={props.view === "dueSoon"}>
                <p>Coursework with due dates in the next 7 days.</p>
              </Show>
              <Show when={props.view === "completed"}>
                <p>
                  Each listed Coursework item has at least one submitted Attempt. A newer Attempt
                  may still be in progress.
                </p>
              </Show>
              <AssessmentList
                assessments={visibleAssessments()}
                course={current()}
                loading={assessments.loading || (props.view === "completed" && progress.loading)}
                view={props.view}
                completedAssessmentIds={completedAssessmentIds()}
              />
            </section>
          </>
        )}
      </Show>
    </PageFrame>
  );
}
