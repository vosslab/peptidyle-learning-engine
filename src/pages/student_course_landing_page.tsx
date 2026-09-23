// Student-owned answer-free course and available assessment landing.

import { A, useParams } from "@solidjs/router";
import { createMemo, createResource, Show, type JSX } from "solid-js";

import type {
  LiveStudentAssessmentLandingSummary,
  LiveStudentCourseLandingSummary,
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
              <span>{display.completionLabel}</span>
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
}): JSX.Element {
  const formatDateTime = createMemo(() => {
    const timeZone = props.assessments[0]?.decision.displayTimeZone;
    return timeZone === undefined ? undefined : createDisplayDateTimeFormatter(timeZone);
  });
  const regions = createMemo(() => {
    const formatter = formatDateTime();
    return formatter === undefined ? [] : assessmentRegions(props.course, formatter);
  });
  function displayTimeZone(): string | undefined {
    return props.assessments[0]?.decision.displayTimeZone;
  }

  return (
    <>
      <Show when={displayTimeZone()}>{(timeZone) => <p>Times shown in {timeZone()}.</p>}</Show>
      <RecordList
        ariaLabel="Coursework"
        emptyState={{ title: "No Coursework is available right now." }}
        recordId={(assessment) => assessment.id}
        regions={regions()}
        rows={props.assessments}
        state={assessmentListState(props.loading)}
      />
    </>
  );
}

/** Student-owned entry point for answer-free current Course work. */
export function StudentCourseLandingPage(): JSX.Element {
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
  function unavailable(): boolean {
    return (
      courseInstanceId() === null || courses.error !== undefined || assessments.error !== undefined
    );
  }

  return (
    <PageFrame routeSurface="studentCourseLanding" title={course()?.longName ?? "Coursework"}>
      <Show when={courses.loading}>
        <p class="loading-state">Loading assigned work...</p>
      </Show>
      <Show when={unavailable()}>
        <section class="route-error" role="alert">
          <h2>Assigned work unavailable</h2>
          <p>This assigned work is not available.</p>
          <A class="primary-link" href="/">
            Return to courses
          </A>
        </section>
      </Show>
      <Show when={!courses.loading && !unavailable() && course() === undefined}>
        <section class="route-error" role="alert">
          <h2>Assigned work unavailable</h2>
          <p>This assigned work is not available.</p>
          <A class="primary-link" href="/">
            Return to courses
          </A>
        </section>
      </Show>
      <Show when={!unavailable() ? course() : undefined}>
        {(current) => (
          <>
            <CourseEntryBanner />
            <A class="quiet-link" href="/student?choose=1">
              Your courses
            </A>
            <section class="student-coursework" aria-labelledby="student-coursework-heading">
              <h2 id="student-coursework-heading">Available Coursework</h2>
              <AssessmentList
                assessments={assessments() ?? []}
                course={current()}
                loading={assessments.loading}
              />
            </section>
          </>
        )}
      </Show>
    </PageFrame>
  );
}
