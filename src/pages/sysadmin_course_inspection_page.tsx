// Sysadmin find and inspect for installation Courses. Management stays out of this page.

import { A, useParams } from "@solidjs/router";
import { Show, createResource, createSignal, type JSX } from "solid-js";

import { useApplicationApi } from "../api/application_api";
import type {
  CourseInstanceLifecycleState,
  CourseRetentionLifecycleState,
  InstallationCourseInspection,
} from "../api/course_instance";
import { PageFrame } from "../components/page_frame";
import {
  RecordList,
  type RecordContent,
  type RecordListState,
} from "../components/record_list/record_list";
import {
  RecordPageControls,
  type RecordPageSize,
} from "../components/record_list/record_page_controls";
import { parseCourseInstanceId } from "../navigation/public_route";

function activityLabel(state: CourseInstanceLifecycleState): string {
  return state === "active" ? "Active" : "Inactive";
}

function retentionLabel(state: CourseRetentionLifecycleState): string {
  switch (state) {
    case "active":
      return "Active";
    case "archived":
      return "Archived";
    case "deleted":
      return "Deleted";
  }
}

function instructorLabel(names: ReadonlyArray<string>): string {
  return names.length === 0 ? "No active Instructor" : names.join(", ");
}

function courseContent(course: InstallationCourseInspection): RecordContent {
  return {
    title: course.longName,
    description: course.shortName,
    details: [
      { kind: "text", label: "Activity", value: activityLabel(course.lifecycleState) },
      { kind: "text", label: "Retention", value: retentionLabel(course.retentionLifecycleState) },
      { kind: "text", label: "Instructor", value: instructorLabel(course.instructorDisplayNames) },
      {
        kind: "text",
        label: "Term",
        value: `${course.term.startDate} through ${course.term.endDate}`,
      },
      { kind: "text", label: "Course ID", value: course.id },
    ],
    actions: [
      {
        id: "inspect",
        kind: "link",
        label: "Inspect",
        href: `/sysadmin/courses/${encodeURIComponent(course.id)}`,
        primary: true,
      },
    ],
  };
}

/** Finds installation Courses and opens one inspection row. */
export function SysadminCourseInspectionListPage(): JSX.Element {
  const runtime = useApplicationApi();
  const [draft, setDraft] = createSignal("");
  const [request, setRequest] = createSignal<{
    query: string;
    cursor: string | null;
    pageSize: RecordPageSize;
    previousCursors: ReadonlyArray<string | null>;
  }>({
    query: "",
    cursor: null,
    pageSize: 50,
    previousCursors: [],
  });
  const [courses] = createResource(request, (value) =>
    runtime.client.listInstallationCourses(value.query, value.cursor, value.pageSize),
  );
  const continuation = (): string | null => {
    const cursor = courses()?.nextCursor;
    return !courses.loading && courses.error === undefined && typeof cursor === "string"
      ? cursor
      : null;
  };
  const listState = (): RecordListState => {
    if (courses.loading) return { kind: "loading", label: "Loading Courses..." };
    if (courses.error !== undefined) {
      return {
        kind: "error",
        title: "Courses unavailable",
        message: "Courses could not be loaded.",
      };
    }
    return { kind: "ready" };
  };

  return (
    <PageFrame
      routeSurface="sysadminCourseInspection"
      headingId="sysadmin-course-inspection-heading"
      eyebrow="System administration"
      title="Courses"
      lede="Find and inspect Courses across the installation."
    >
      <form
        class="auth-panel"
        novalidate
        onSubmit={(event) => {
          event.preventDefault();
          setRequest({ query: draft(), cursor: null, pageSize: 50, previousCursors: [] });
        }}
      >
        <h2>Find a Course</h2>
        <label for="installation-course-find">
          Course name, Course ID, or Instructor
          <input
            id="installation-course-find"
            name="installationCourseFind"
            type="search"
            value={draft()}
            onInput={(event) => setDraft(event.currentTarget.value)}
            autocomplete="off"
            autocapitalize="none"
            spellcheck={false}
            maxlength={200}
          />
        </label>
        <button class="primary-action" type="submit">
          Find Courses
        </button>
      </form>
      <RecordList
        ariaLabel="Installation Courses"
        emptyState={{
          title:
            request().query.trim().length > 0
              ? "No Course matches that search."
              : "No Courses are available.",
        }}
        recordId={(course) => course.id}
        content={courseContent}
        rows={courses()?.courses ?? []}
        state={listState()}
      />
      <RecordPageControls
        ariaLabel="Installation Course pages"
        hasPrevious={request().previousCursors.length > 0}
        hasNext={continuation() !== null}
        loading={courses.loading}
        disabled={courses.error !== undefined}
        onPrevious={() => {
          if (courses.loading || request().previousCursors.length === 0) return;
          const previousCursors = request().previousCursors.slice(0, -1);
          setRequest({
            ...request(),
            cursor: request().previousCursors[request().previousCursors.length - 1] ?? null,
            previousCursors,
          });
        }}
        onNext={() => {
          const cursor = continuation();
          if (cursor === null) return;
          setRequest({
            ...request(),
            previousCursors: [...request().previousCursors, request().cursor],
            cursor,
          });
        }}
        pageSize={request().pageSize}
        onPageSizeChange={(pageSize) =>
          setRequest({ ...request(), pageSize, cursor: null, previousCursors: [] })
        }
      />
    </PageFrame>
  );
}

/** Shows the Instructor and Course status for one installation Course. */
export function SysadminCourseInspectionPage(): JSX.Element {
  const runtime = useApplicationApi();
  const params = useParams();
  const courseId = (): ReturnType<typeof parseCourseInstanceId> =>
    parseCourseInstanceId(params.courseInstanceId ?? "");
  const [course] = createResource(courseId, (id) => runtime.client.loadInstallationCourse(id));
  const loaded = (): InstallationCourseInspection | undefined =>
    course.error === undefined ? course() : undefined;

  return (
    <PageFrame
      routeSurface="sysadminCourseInspectionDetail"
      headingId="sysadmin-course-detail-heading"
      eyebrow="System administration"
      title={loaded()?.longName ?? "Course"}
      lede="Inspect the Instructor and Course status. This page does not change the Course."
    >
      <p>
        <A href="/sysadmin/courses">All Courses</A>
      </p>
      <Show when={course.loading}>
        <p role="status">Loading Course...</p>
      </Show>
      <Show when={courseId() === null || course.error !== undefined}>
        <p role="alert">This Course could not be loaded.</p>
      </Show>
      <Show when={loaded()}>
        {(inspection) => (
          <RecordList
            ariaLabel="Course inspection"
            emptyState={{ title: "This Course could not be loaded." }}
            recordId={(item) => item.id}
            content={(item) => ({
              title: item.longName,
              description: item.shortName,
              details: courseContent(item).details,
              actions: [],
            })}
            rows={[inspection()]}
            state={{ kind: "ready" }}
          />
        )}
      </Show>
    </PageFrame>
  );
}
