// course_assignments_page.tsx - cursor-ready assignment list for one course.

import { A, createAsync, revalidate } from "@solidjs/router";
import { For, Show, Suspense, createSignal, type JSX } from "solid-js";

import type { CourseId } from "../../generated/api/CourseId";
import { useApplicationApi } from "../api/application_api";
import type { CourseSummary, CursorPage, StudentAssignmentLandingSummary } from "../api/contracts";
import { CourseEntryIdentity } from "../features/course_appearance/course_entry_identity";
import { courseRouteView } from "../features/course_appearance/course_theme_context";
import { useSessionBootstrap } from "../auth/session_context";
import { studentProgressSummary } from "../student_progress";
import { CursorPageSession, type CursorPageSessionState } from "./cursor_page_session";
import {
  assignmentRouteReference,
  courseInstanceRouteReference,
  type CourseInstanceRouteReference,
} from "../navigation/public_route";
import { useRouteScopeData } from "../ribbon/route_scope_context";

const COURSE_ASSIGNMENTS_IDENTITY_STYLES = `
.course-assignments-identity {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  column-gap: var(--ple-space-4, 1rem);
  row-gap: var(--ple-space-1, 0.25rem);
  align-items: end;
  margin-bottom: var(--ple-space-4, 1rem);
}

.course-assignments-identity .eyebrow,
.course-assignments-identity h1 {
  grid-column: 1;
}

.course-assignments-identity .eyebrow {
  margin: 0;
}

.course-assignments-identity h1 {
  margin: 0;
}

.course-assignments-identity .primary-link {
  grid-column: 2;
  grid-row: 1 / span 2;
  margin: 0;
}

@media (max-width: 36rem) {
  .course-assignments-identity {
    grid-template-columns: 1fr;
  }

  .course-assignments-identity .primary-link {
    grid-column: 1;
    grid-row: auto;
    justify-self: start;
    margin-top: var(--ple-space-2, 0.5rem);
  }
}
`;

export interface AssignmentListProps {
  readonly courseId: CourseId;
  readonly courseReference: CourseInstanceRouteReference;
  readonly initialPage: CursorPage<StudentAssignmentLandingSummary>;
  readonly reloadAssignments: () => Promise<void>;
  readonly canCreateAssignment: boolean;
}

function assignmentLinkId(assignment: StudentAssignmentLandingSummary): string {
  const id = `assignment-review-${assignmentRouteReference(assignment.reference)}`;
  return id;
}

function pluralize(count: number, singular: string, plural: string): string {
  return count === 1 ? singular : plural;
}

interface AssignmentCardProps {
  readonly assignment: StudentAssignmentLandingSummary;
  readonly courseReference: CourseInstanceRouteReference;
  readonly canManageAssignment: boolean;
  readonly registerLink: (
    assignment: StudentAssignmentLandingSummary,
    element: HTMLAnchorElement,
  ) => void;
}

function AssignmentCard(props: AssignmentCardProps): JSX.Element {
  const applicationApi = useApplicationApi();
  const progress = createAsync(() =>
    props.canManageAssignment
      ? Promise.resolve(null)
      : applicationApi.queries.assignmentSummary(props.assignment.id).catch(() => null),
  );

  return (
    <article class="course-card">
      <p class="card-kicker">Mastery practice</p>
      <h2>
        <A
          class="assignment-title-link"
          href={
            props.canManageAssignment
              ? `/instructor/courses/${props.courseReference}/assignments/${assignmentRouteReference(props.assignment.reference)}`
              : `/courses/${props.courseReference}/assignments/${assignmentRouteReference(props.assignment.reference)}`
          }
          id={assignmentLinkId(props.assignment)}
          ref={(element) => props.registerLink(props.assignment, element)}
        >
          {props.assignment.title}
        </A>
      </h2>
      <p class="course-card-description">
        Open this assignment to review its instructions and delivery details.
      </p>
      <Show when={!props.canManageAssignment}>
        <Suspense fallback={<p class="course-card-progress">Progress loading...</p>}>
          <Show
            when={progress()}
            fallback={<p class="course-card-progress">Progress unavailable.</p>}
          >
            {(assignmentProgress) => (
              <p class="course-card-progress">{studentProgressSummary(assignmentProgress())}</p>
            )}
          </Show>
        </Suspense>
      </Show>
      <div class="course-card-actions">
        <A
          class="quiet-link"
          href={
            props.canManageAssignment
              ? `/instructor/courses/${props.courseReference}/assignments/${assignmentRouteReference(props.assignment.reference)}`
              : `/courses/${props.courseReference}/assignments/${assignmentRouteReference(props.assignment.reference)}`
          }
        >
          {props.canManageAssignment ? "Open assignment" : "Start assignment"}
        </A>
        <Show when={props.canManageAssignment}>
          <A
            class="quiet-link"
            href={`/instructor/courses/${props.courseReference}/assignments/${assignmentRouteReference(props.assignment.reference)}/delivery-check`}
          >
            Check assignment delivery
          </A>
        </Show>
      </div>
    </article>
  );
}

export function AssignmentList(props: AssignmentListProps): JSX.Element {
  const applicationApi = useApplicationApi();
  const [state, setState] = createSignal<CursorPageSessionState<StudentAssignmentLandingSummary>>({
    items: [],
    nextCursor: null,
    loading: false,
    error: null,
  });
  const [announcement, setAnnouncement] = createSignal("");
  const reviewLinks = new Map<string, HTMLAnchorElement>();
  let retryButton: HTMLButtonElement | undefined;
  let reloadButton: HTMLButtonElement | undefined;
  const session = new CursorPageSession(
    props.initialPage,
    (cursor) => applicationApi.client.listAssignments(props.courseId, cursor),
    (assignment) => assignment.id,
    setState,
  );
  setState(session.state);

  function announceLoaded(count: number, total: number): void {
    setAnnouncement(
      `Loaded ${count} more ${pluralize(count, "assignment", "assignments")}. ${total} ${pluralize(total, "assignment", "assignments")} visible.`,
    );
  }

  function focusFirstAppended(appended: ReadonlyArray<StudentAssignmentLandingSummary>): void {
    const first = appended[0];
    if (first === undefined) return;
    requestAnimationFrame(() => reviewLinks.get(first.id)?.focus());
  }

  function focusRecoveryButton(kind: "protocol" | "transport"): void {
    requestAnimationFrame(() => {
      if (kind === "transport") retryButton?.focus();
      else reloadButton?.focus();
    });
  }

  function statusMessage(): string {
    const current = state();
    if (current.loading) return "Loading more assignments...";
    if (current.error !== null || current.items.length === 0) return "";
    if (current.nextCursor === null) {
      const count = current.items.length;
      return `Loaded ${count} ${pluralize(count, "assignment", "assignments")}.`;
    }
    return announcement();
  }

  async function loadMore(): Promise<void> {
    const appended = await session.loadMore();
    const current = state();
    if (current.error !== null) {
      focusRecoveryButton(current.error.kind);
      return;
    }
    if (appended.length > 0) {
      announceLoaded(appended.length, current.items.length);
      focusFirstAppended(appended);
    }
  }

  async function retryLoadMore(): Promise<void> {
    const appended = await session.retry();
    const current = state();
    if (current.error !== null) {
      focusRecoveryButton(current.error.kind);
      return;
    }
    if (appended.length > 0) {
      announceLoaded(appended.length, current.items.length);
      focusFirstAppended(appended);
    }
  }

  return (
    <>
      <Show
        when={
          (state().error === null && state().nextCursor !== null) ||
          state().error?.kind === "transport" ||
          state().error?.kind === "protocol"
        }
      >
        <a class="skip-link" href="#assignment-pagination" target="_self">
          Skip to load more assignments
        </a>
      </Show>
      <Show
        when={state().items.length > 0 || state().nextCursor !== null}
        fallback={
          <section class="empty-state" aria-label="No assignments yet">
            <h2>No assignments yet</h2>
            <p>
              {props.canCreateAssignment
                ? "Build the first practice assignment from published questions."
                : "Your instructor has not released an Assignment for this course yet."}
            </p>
            <Show when={props.canCreateAssignment}>
              <A
                class="primary-link"
                href={`/instructor/courses/${props.courseReference}/assignments/new`}
              >
                Create the first assignment
              </A>
            </Show>
          </section>
        }
      >
        <div class="card-grid" aria-busy={state().loading}>
          <For each={state().items}>
            {(assignment) => (
              <AssignmentCard
                assignment={assignment}
                courseReference={props.courseReference}
                canManageAssignment={props.canCreateAssignment}
                registerLink={(currentAssignment, element) =>
                  reviewLinks.set(currentAssignment.id, element)
                }
              />
            )}
          </For>
        </div>
      </Show>
      <Show when={statusMessage().length > 0}>
        <p role="status" aria-live="polite" aria-atomic="true">
          {statusMessage()}
        </p>
      </Show>
      <Show
        when={
          state().error?.kind === "transport" ||
          state().error?.kind === "protocol" ||
          state().nextCursor !== null
        }
      >
        <section
          id="assignment-pagination"
          class="assignment-pagination"
          aria-label="Assignment pagination"
          tabindex="-1"
        >
          <Show when={state().error?.kind === "transport"}>
            <section class="route-error" role="alert">
              <p>
                Could not load more assignments. The {state().items.length}{" "}
                {pluralize(state().items.length, "assignment", "assignments")} already visible{" "}
                {state().items.length === 1 ? "is" : "are"} still available.
              </p>
              <button
                class="primary-action"
                type="button"
                ref={(element) => (retryButton = element)}
                onClick={() => void retryLoadMore()}
              >
                Try loading more assignments again
              </button>
            </section>
          </Show>
          <Show when={state().error?.kind === "protocol"}>
            <section class="route-error" role="alert">
              <p>{state().error?.message}</p>
              <button
                class="primary-action"
                type="button"
                ref={(element) => (reloadButton = element)}
                onClick={() => void props.reloadAssignments()}
              >
                Reload assignments
              </button>
            </section>
          </Show>
          <Show when={state().error === null && state().nextCursor !== null}>
            <button
              class="primary-action"
              type="button"
              disabled={state().loading}
              onClick={() => void loadMore()}
            >
              {state().loading ? "Loading more assignments..." : "Load more assignments"}
            </button>
          </Show>
        </section>
      </Show>
    </>
  );
}

function CourseAssignmentsContent(props: { readonly course: CourseSummary }): JSX.Element {
  const applicationApi = useApplicationApi();
  const session = useSessionBootstrap();
  const course = props.course;
  const courseId = course.id;
  const courseReference = courseInstanceRouteReference(course.reference);
  const assignmentCreateHref = (): string | undefined =>
    courseReference === undefined
      ? undefined
      : `/instructor/courses/${courseReference}/assignments/new`;
  const assignments = createAsync(() => {
    return applicationApi.queries.assignments(courseId);
  });
  const canManageCourse = (): boolean => {
    const current = session.state();
    const hasInstructorRole =
      current.kind === "authenticated" && current.session.account.productRole === "instructor";
    return hasInstructorRole && course.role === "instructor";
  };
  async function reloadAssignments(): Promise<void> {
    await revalidate(applicationApi.queries.assignments.keyFor(courseId));
  }

  return (
    <section class="page" data-route-surface="courseAssignments">
      <Show
        when={canManageCourse() && assignmentCreateHref() !== undefined}
        fallback={
          <>
            <CourseEntryIdentity />
            <h2>Assignments</h2>
          </>
        }
      >
        <header class="course-assignments-identity">
          <style>{COURSE_ASSIGNMENTS_IDENTITY_STYLES}</style>
          <p class="eyebrow">Instructor course</p>
          <h1>Assignments</h1>
          <A class="primary-link" href={assignmentCreateHref()!}>
            New assignment
          </A>
        </header>
      </Show>
      <Suspense fallback={<p class="loading-state">Loading assignments...</p>}>
        <Show
          when={assignments()}
          keyed
          fallback={<p class="empty-state">No assignments are available in this course.</p>}
        >
          {(page) => {
            return (
              <AssignmentList
                courseId={courseId}
                courseReference={courseReference}
                initialPage={page}
                reloadAssignments={reloadAssignments}
                canCreateAssignment={canManageCourse()}
              />
            );
          }}
        </Show>
      </Suspense>
    </section>
  );
}

/** Keeps the assignment resource below the content-level deferred course boundary. */
export function CourseAssignmentsPage(): JSX.Element {
  const routeData = useRouteScopeData();
  const course = (): CourseSummary | undefined => {
    const data = routeData();
    return data?.kind === "course" ? courseRouteView(data).summary : undefined;
  };
  return (
    <Show
      when={course()}
      keyed
      fallback={
        <section class="page" data-route-surface="courseAssignments">
          <p class="loading-state" role="status">
            Loading assignments...
          </p>
        </section>
      }
    >
      {(loadedCourse) => <CourseAssignmentsContent course={loadedCourse} />}
    </Show>
  );
}
