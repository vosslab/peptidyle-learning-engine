// Sysadmin find and inspect for installation Courses. Management stays out of this page.

import { A, useParams } from "@solidjs/router";
import { Show, createResource, createSignal, type JSX } from "solid-js";

import { useApplicationApi } from "../api/application_api";
import type { CourseInstanceId } from "../../generated/api/CourseInstanceId";
import type {
  CourseInstanceLifecycleState,
  CourseRetentionLifecycleState,
  InstallationCourseInspection,
} from "../api/course_instance";
import type { SysadminStudentDataAccess } from "../api/sysadmin_student_access";
import { PageFrame } from "../components/page_frame";
import { RecordList, type RecordContent } from "../components/record_list/record_list";
import type {
  RecordField,
  RecordListQuery,
  RecordSort,
} from "../components/record_list/record_fields";
import { SearchBlock } from "../features/search/search_block";
import { createSearchState } from "../features/search/search_state";
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

function rosterStateLabel(state: SysadminStudentDataAccess["state"]): string {
  const labels: Record<SysadminStudentDataAccess["state"], string> = {
    invitationPending: "Invitation pending",
    activeStudent: "Active Student",
    removed: "Course access removed",
  };
  return labels[state];
}

function accessErrorMessage(error: unknown): string {
  if (typeof error === "object" && error !== null && "status" in error && error.status === 404) {
    return "That Course roster record is unavailable.";
  }
  if (typeof error === "object" && error !== null && "status" in error && error.status === 422) {
    return "The administrative-work confirmation was not accepted. Review it and try again.";
  }
  return "Student data could not be loaded. Check the roster ID and try again.";
}

function SysadminStudentDataAccessForm(props: {
  readonly courseInstanceId: CourseInstanceId;
}): JSX.Element {
  const runtime = useApplicationApi();
  const [rosterId, setRosterId] = createSignal("");
  const [confirmed, setConfirmed] = createSignal(false);
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [access, setAccess] = createSignal<SysadminStudentDataAccess | null>(null);

  const clearResult = (): void => {
    setAccess(null);
    setError(null);
  };
  const cancel = (): void => {
    setRosterId("");
    setConfirmed(false);
    clearResult();
  };

  return (
    <section class="auth-panel" aria-labelledby="sysadmin-student-data-heading">
      <h2 id="sysadmin-student-data-heading">Access one Student roster record</h2>
      <p>
        Use this lookup only to support Course administration or resolve a Student issue. Access is
        recorded for audit.
      </p>
      <form
        onSubmit={(event) => {
          event.preventDefault();
          if (!confirmed() || busy()) return;
          const requestedRosterId = rosterId().trim();
          if (!/^[A-Za-z0-9._-]{1,64}$/u.test(requestedRosterId)) {
            setError(
              "Enter a roster ID using 1 to 64 letters, numbers, periods, underscores, or hyphens.",
            );
            return;
          }
          setBusy(true);
          clearResult();
          void runtime.client
            .accessSysadminStudentData(props.courseInstanceId, requestedRosterId)
            .then(setAccess)
            .catch((cause: unknown) => setError(accessErrorMessage(cause)))
            .finally(() => setBusy(false));
        }}
      >
        <label for="sysadmin-student-roster-id">
          Student roster ID
          <input
            id="sysadmin-student-roster-id"
            name="studentRosterId"
            type="text"
            autocomplete="off"
            autocapitalize="none"
            spellcheck={false}
            maxlength={64}
            value={rosterId()}
            onInput={(event) => {
              setRosterId(event.currentTarget.value);
              setConfirmed(false);
              clearResult();
            }}
          />
        </label>
        <label>
          <input
            name="administrativeAccessConfirmed"
            type="checkbox"
            checked={confirmed()}
            onChange={(event) => {
              setConfirmed(event.currentTarget.checked);
              clearResult();
            }}
          />
          I confirm this access is needed for administrative work.
        </label>
        <div class="action-row">
          <button class="primary-action" type="submit" disabled={busy() || !confirmed()}>
            {busy() ? "Accessing Student record..." : "Access Student record"}
          </button>
          <button class="quiet-action" type="button" disabled={busy()} onClick={cancel}>
            Cancel
          </button>
        </div>
      </form>
      <Show when={busy()}>
        <p role="status">Accessing the Student roster record and recording the audit event...</p>
      </Show>
      <Show when={error()}>{(message) => <p role="alert">{message()}</p>}</Show>
      <Show when={access()}>
        {(record) => (
          <section aria-labelledby="sysadmin-student-data-result-heading" role="status">
            <h3 id="sysadmin-student-data-result-heading">Student roster record accessed</h3>
            <dl>
              <dt>Roster ID</dt>
              <dd>{record().rosterId}</dd>
              <dt>Course access</dt>
              <dd>{rosterStateLabel(record().state)}</dd>
              <dt>Audit event</dt>
              <dd>{record().audit.eventId}</dd>
              <dt>Recorded at</dt>
              <dd>{new Date(record().audit.occurredAt).toISOString()}</dd>
            </dl>
          </section>
        )}
      </Show>
    </section>
  );
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
  type Query = {
    query: string;
    sort: "nameAscending" | "nameDescending";
    activity: ReadonlyArray<CourseInstanceLifecycleState>;
    retention: ReadonlyArray<CourseRetentionLifecycleState>;
  };
  type SortKey = "name";
  type FilterKey = "activity" | "retention";
  const [activityChoices, setActivityChoices] = createSignal<
    ReadonlyArray<{ key: CourseInstanceLifecycleState; count: number }>
  >([]);
  const [retentionChoices, setRetentionChoices] = createSignal<
    ReadonlyArray<{ key: CourseRetentionLifecycleState; count: number }>
  >([]);
  const fields: ReadonlyArray<RecordField<InstallationCourseInspection, SortKey, FilterKey>> = [
    {
      id: "name",
      label: "Course",
      role: "identity",
      sortKey: "name",
      value: (course) => course.longName,
      defaultColumn: { priority: 0, width: "18rem" },
    },
    {
      id: "course_id",
      label: "Course ID",
      value: (course) => course.id,
      defaultColumn: { priority: 1, width: "12rem" },
    },
    {
      id: "short_name",
      label: "Short name",
      value: (course) => course.shortName,
      defaultColumn: { priority: 2, width: "12rem" },
    },
    {
      id: "activity",
      label: "Activity",
      filterKey: "activity",
      value: (course) => course.lifecycleState,
      defaultColumn: { priority: 3, width: "8rem" },
    },
    {
      id: "retention",
      label: "Retention",
      filterKey: "retention",
      value: (course) => course.retentionLifecycleState,
      defaultColumn: { priority: 4, width: "8rem" },
    },
    {
      id: "instructor",
      label: "Instructor",
      value: (course) => course.instructorDisplayNames,
      defaultColumn: { priority: 5, width: "12rem" },
    },
    {
      id: "term",
      label: "Term",
      value: (course) => `${course.term.startDate} through ${course.term.endDate}`,
      defaultColumn: { priority: 6, width: "14rem" },
    },
  ];
  const searchState = createSearchState<Query, InstallationCourseInspection, undefined>(
    {
      initialQuery: { query: "", sort: "nameAscending", activity: [], retention: [] },
      maximumPageSize: 100,
      cleanup: (query) => ({ ...query, query: query.query.trim() }),
      isEligible: () => true,
      getText: (query) => query.query,
      setText: (query, text) => ({ ...query, query: text }),
      rowId: (course) => course.id,
      content: courseContent,
      fetchPage: async (query, cursor, pageSize) => {
        const page = await runtime.client.listInstallationCourses({
          ...query,
          cursor,
          pageSize: pageSize === 250 ? 100 : pageSize,
        });
        setActivityChoices(page.activityChoices);
        setRetentionChoices(page.retentionChoices);
        return { items: page.courses, nextCursor: page.nextCursor, filterCounts: undefined };
      },
    },
    { runOnMount: true },
  );
  const recordQuery: RecordListQuery<SortKey, FilterKey> = {
    kind: "paged",
    sort: {
      get value(): RecordSort<SortKey> {
        return {
          key: "name",
          direction: searchState.query().sort === "nameAscending" ? "ascending" : "descending",
        };
      },
      onChange: (sort) =>
        void searchState.apply((current) => ({
          ...current,
          sort: sort.direction === "ascending" ? "nameAscending" : "nameDescending",
        })),
    },
    filters: {
      get values() {
        return new Map<FilterKey, ReadonlyArray<string>>([
          ["activity", searchState.query().activity],
          ["retention", searchState.query().retention],
        ]);
      },
      onChange: (field, values) =>
        void searchState.apply((current) =>
          field === "activity"
            ? { ...current, activity: values as CourseInstanceLifecycleState[] }
            : { ...current, retention: values as CourseRetentionLifecycleState[] },
        ),
      onClear: () =>
        void searchState.apply((current) => ({ ...current, activity: [], retention: [] })),
      choices: (field, search, _cursor, signal) => {
        const source = field === "activity" ? activityChoices() : retentionChoices();
        return Promise.resolve({
          choices: signal.aborted
            ? []
            : source
                .filter((choice) => choice.key.includes(search.toLowerCase()))
                .map((choice) => ({
                  key: choice.key,
                  label:
                    field === "activity"
                      ? activityLabel(choice.key as CourseInstanceLifecycleState)
                      : retentionLabel(choice.key as CourseRetentionLifecycleState),
                  count: choice.count,
                })),
          selected: [],
          nextCursor: null,
        });
      },
    },
  };

  return (
    <PageFrame routeSurface="sysadminCourseInspection">
      <SearchBlock
        layout="above"
        state={searchState}
        controls={
          <form
            class="auth-panel"
            novalidate
            onSubmit={(event) => {
              event.preventDefault();
              void searchState.submit();
            }}
          >
            <label for="installation-course-find">
              Course name, Course ID, or Instructor
              <input
                id="installation-course-find"
                name="installationCourseFind"
                type="search"
                value={searchState.typedText()}
                onInput={(event) => searchState.setTypedText(event.currentTarget.value)}
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
        }
        collection={{
          ariaLabel: "Installation Courses",
          fields,
          query: recordQuery,
          emptyState: { title: "No Courses match the current search or filters." },
        }}
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
    <PageFrame routeSurface="sysadminCourseInspectionDetail">
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
          <>
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
            <SysadminStudentDataAccessForm courseInstanceId={inspection().id} />
          </>
        )}
      </Show>
    </PageFrame>
  );
}
