// Student-owned answer-free course and available assessment landing.

import { A, useParams } from "@solidjs/router";
import { createMemo, createResource, Show, type JSX } from "solid-js";

import type {
  LiveStudentAssessmentLandingSummary,
  LiveStudentCourseLandingSummary,
  StudentCourseProgressAssessment,
} from "../api/live_student_course_landing";
import { assessmentTypePresentation } from "../assessment_type_presentation";
import { useApplicationApi } from "../api/application_api";
import { PageFrame } from "../components/page_frame/page_frame";
import { SearchBlock } from "../components/search_block/search_block";
import { SearchControls } from "../components/search_block/search_controls";
import { createLocalCompleteSearch } from "../components/search_block/local_complete_search_state";
import {
  byStudentTitleThenId,
  byStudentCrossCourseTitleThenId,
  studentAssessmentFields,
  studentCrossCourseAssessmentFields,
  studentCrossCourseAssessmentId,
  studentCrossCourseLabel,
  type StudentCrossCourseAssessment,
} from "./student_collection_fields";
import {
  loadStudentCrossCourseCollection,
  visibleStudentCoursework,
} from "./student_cross_course_collection";
import type { RecordContent, RecordFact } from "../components/data_sheet/record_content";
import { formatAssessmentDeliveryTime } from "../components/student_assessment_presentation";
import { CourseEntryBanner } from "../features/course_appearance/course_entry_banner";
import { createDisplayDateTimeFormatter } from "../format_datetime";
import { parseCourseInstanceId } from "../navigation/public_route";
import { buildRoutePath } from "../ribbon/ribbon_route_path";
import { studentCourseworkDisplay } from "./student_coursework_presentation";
import {
  hasSubmittedStudentAttempt,
  isInStudentDueSoonWindow,
} from "./student_coursework_presentation";

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

function assessmentContent(
  course: LiveStudentCourseLandingSummary,
  view: StudentCourseworkView,
  completedAssessmentIds: ReadonlySet<string>,
): (assessment: LiveStudentAssessmentLandingSummary) => RecordContent {
  return (assessment): RecordContent => {
    const display = assessmentDisplay(assessment);
    const dueAt = assessment.decision.dueAt;
    const formatDateTime = createDisplayDateTimeFormatter(assessment.decision.displayTimeZone);
    const details: Array<RecordFact> = [
      { kind: "text", label: "Status:", value: display.stateLabel },
      { kind: "assessmentType", value: assessment.assessmentType },
      { kind: "text", label: "Access:", value: assessmentAccessLabel(assessment) },
      dueAt === null
        ? { kind: "text", label: "Due:", value: "No due time" }
        : {
            kind: "time",
            label: "Due:",
            value: formatAssessmentDeliveryTime(dueAt, formatDateTime),
            dateTime: new Date(dueAt).toISOString(),
          },
      {
        kind: "text",
        value:
          view === "completed" && completedAssessmentIds.has(assessment.id)
            ? "Submitted Attempt on record"
            : display.completionLabel,
      },
    ];
    return {
      title: assessment.title,
      details,
      actions: [
        {
          id: "open-assessment",
          kind: "link",
          label: `${display.actionVerb} ${assessmentTypePresentation(assessment.assessmentType).label}`,
          href: assessmentOverviewPath(course.id, assessment.id),
          primary: true,
        },
      ],
    };
  };
}

export function AssessmentList(props: {
  readonly course: LiveStudentCourseLandingSummary;
  readonly assessments: ReadonlyArray<LiveStudentAssessmentLandingSummary>;
  readonly loading: boolean;
  readonly view: StudentCourseworkView;
  readonly completedAssessmentIds: ReadonlySet<string>;
  readonly onRefresh: () => unknown;
}): JSX.Element {
  const rows = createMemo(() =>
    props.loading ? undefined : byStudentTitleThenId(props.assessments),
  );
  const assessmentSearch = createLocalCompleteSearch(
    { rows: rows, loading: () => props.loading, refetchSource: () => props.onRefresh() },
    {
      fields: studentAssessmentFields,
      rowId: (assessment) => assessment.id,
      keyword: (assessment) =>
        `${assessment.title} ${assessmentTypePresentation(assessment.assessmentType).label}`,
      content: (assessment) =>
        assessmentContent(props.course, props.view, props.completedAssessmentIds)(assessment),
    },
  );
  return (
    <SearchBlock
      layout="above"
      controls={
        <SearchControls
          state={assessmentSearch.state}
          textLabel={`Search ${courseworkHeading(props.view)}`}
        />
      }
      state={assessmentSearch.state}
      collection={{
        ariaLabel: courseworkHeading(props.view),
        fields: studentAssessmentFields,
        query: assessmentSearch.query,
        get emptyState() {
          return {
            title:
              assessmentSearch.state.query().text || assessmentSearch.state.query().filters.size
                ? "No Coursework matches your search."
                : emptyCourseworkMessage(props.view),
          };
        },
      }}
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

function StudentCrossCourseworkCollection(props: {
  readonly courses: ReadonlyArray<LiveStudentCourseLandingSummary>;
  readonly view: StudentCourseworkView;
}): JSX.Element {
  const api = useApplicationApi();
  const [collection, { refetch }] = createResource(
    () => props.courses,
    (courses) =>
      loadStudentCrossCourseCollection<StudentCrossCourseAssessment>(courses, async (course) => {
        const [assessments, progress] = await Promise.all([
          api.client.listLiveStudentAssessments(course.id),
          props.view === "completed"
            ? api.client.getStudentCourseProgress(course.id)
            : Promise.resolve([]),
        ]);
        const visible = visibleStudentCoursework(assessments, props.view, progress);
        return visible.map((assessment) => ({ course, assessment }));
      }),
  );
  const search = createLocalCompleteSearch(
    {
      rows: () => {
        const source = collection();
        return source === undefined ? undefined : byStudentCrossCourseTitleThenId(source.rows);
      },
      loading: () => collection.loading,
      error: (): unknown => collection.error,
      refetchSource: refetch,
    },
    {
      fields: studentCrossCourseAssessmentFields,
      rowId: studentCrossCourseAssessmentId,
      keyword: (row) =>
        `${studentCrossCourseLabel(row.course)} ${row.assessment.title} ${assessmentTypePresentation(row.assessment.assessmentType).label}`,
      content: (row) => {
        const content = assessmentContent(
          row.course,
          props.view,
          props.view === "completed" ? new Set([row.assessment.id]) : new Set(),
        )(row.assessment);
        return {
          ...content,
          details: [
            { kind: "text", label: "Course:", value: studentCrossCourseLabel(row.course) },
            ...content.details,
          ],
        };
      },
    },
  );
  const partial = (): boolean => (collection()?.failedCourses.length ?? 0) > 0;

  return (
    <section class="student-coursework" aria-label={courseworkHeading(props.view)}>
      <Show when={partial()}>
        <p class="route-error" role="alert">
          Some Coursework could not be loaded. Results are partial. Try again for{" "}
          {collection()?.failedCourses.map(studentCrossCourseLabel).join(", ")}.
          <button type="button" onClick={() => void refetch()}>
            Retry Coursework
          </button>
        </p>
      </Show>
      <SearchBlock
        layout="above"
        controls={
          <SearchControls
            state={search.state}
            textLabel={`Search ${courseworkHeading(props.view)}`}
          />
        }
        state={search.state}
        collection={{
          ariaLabel: courseworkHeading(props.view),
          fields: studentCrossCourseAssessmentFields,
          query: search.query,
          get emptyState() {
            return {
              title: partial()
                ? "No matching Coursework in the Courses that loaded. Results are partial."
                : search.state.query().text || search.state.query().filters.size
                  ? "No Coursework matches your search."
                  : emptyCourseworkMessage(props.view),
            };
          },
        }}
      />
    </section>
  );
}

/** Shows the selected Coursework view across every currently enrolled Course. */
function StudentCourseworkAcrossCoursesPage(props: {
  readonly view: StudentCourseworkView;
}): JSX.Element {
  const api = useApplicationApi();
  const [courses, { refetch: refetchCourses }] = createResource(() =>
    api.client.listLiveStudentCourses(),
  );

  return (
    <PageFrame
      routeSurface={
        props.view === "all"
          ? "studentHome"
          : props.view === "dueSoon"
            ? "studentDueSoon"
            : "studentCompleted"
      }
    >
      <Show when={props.view === "dueSoon"}>
        <p>Next 7 days</p>
      </Show>
      <Show when={props.view === "completed"}>
        <p>Each listed Coursework item has at least one submitted Attempt.</p>
      </Show>
      <Show when={courses.loading}>
        <p class="loading-state">Loading Coursework...</p>
      </Show>
      <Show when={courses.error !== undefined}>
        <section class="route-error" role="alert">
          <h2>Coursework unavailable</h2>
          <p>Coursework could not be loaded right now.</p>
          <button type="button" onClick={() => void refetchCourses()}>
            Retry Coursework
          </button>
        </section>
      </Show>
      <Show when={!courses.loading && courses.error === undefined && courses()?.length === 0}>
        <p class="empty-state">No current Courses have Coursework available.</p>
      </Show>
      <Show
        keyed
        when={
          !courses.loading && courses.error === undefined && (courses()?.length ?? 0) > 0
            ? courses()
            : undefined
        }
      >
        {(current) => <StudentCrossCourseworkCollection courses={current} view={props.view} />}
      </Show>
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
  const [assessments, { refetch: refetchAssessments }] = createResource(course, loadAssessments);
  const [progress, { refetch: refetchProgress }] = createResource(
    () => (props.view === "completed" ? course() : undefined),
    (current) => applicationApi.client.getStudentCourseProgress(current.id),
  );
  async function refreshCoursework(): Promise<void> {
    await Promise.all([
      refetchAssessments(),
      props.view === "completed" ? refetchProgress() : Promise.resolve(),
    ]);
  }
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
      <Show keyed when={!unavailable() ? course() : undefined}>
        {(current) => (
          <>
            <CourseEntryBanner />
            <A class="quiet-link" href="/student/courses">
              Your Courses
            </A>
            <A class="quiet-link" href={`/student/courses/${current.id}/progress`}>
              Course Progress
            </A>
            <section class="student-coursework" aria-label={courseworkHeading(props.view)}>
              <Show when={props.view === "dueSoon"}>
                <p>Next 7 days</p>
              </Show>
              <Show when={props.view === "completed"}>
                <p>Each listed Coursework item has at least one submitted Attempt.</p>
              </Show>
              <AssessmentList
                assessments={visibleAssessments()}
                course={current}
                loading={assessments.loading || (props.view === "completed" && progress.loading)}
                view={props.view}
                completedAssessmentIds={completedAssessmentIds()}
                onRefresh={refreshCoursework}
              />
            </section>
          </>
        )}
      </Show>
    </PageFrame>
  );
}
