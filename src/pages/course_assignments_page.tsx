// course_assignments_page.tsx - cursor-ready assignment list for one course.

import { A, createAsync, revalidate } from "@solidjs/router";
import { For, Show, Suspense, createSignal, type JSX } from "solid-js";

import type { CourseId } from "../../generated/api/CourseId";
import { useApiRuntime } from "../api/runtime";
import type { CursorPage, StudentAssignmentLandingSummary } from "../api/contracts";
import { CourseEntryIdentity } from "../features/course_appearance/course_entry_identity";
import {
  courseRouteData,
  useCourseThemeRouteData,
} from "../features/course_appearance/course_theme_context";
import { useSessionBootstrap } from "../auth/session_context";
import { studentProgressSummary } from "../student_progress";
import { CursorPageSession, type CursorPageSessionState } from "./cursor_page_session";
import {
  assignmentRouteReference,
  courseRouteReference,
  type CourseRouteReference,
} from "../navigation/public_route";

export interface AssignmentListProps {
  readonly courseId: CourseId;
  readonly courseReference: CourseRouteReference;
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
  readonly courseReference: CourseRouteReference;
  readonly canManageAssignment: boolean;
  readonly registerLink: (
    assignment: StudentAssignmentLandingSummary,
    element: HTMLAnchorElement,
  ) => void;
}

function AssignmentCard(props: AssignmentCardProps): JSX.Element {
  const runtime = useApiRuntime();
  const progress = createAsync(() =>
    props.canManageAssignment
      ? Promise.resolve(null)
      : runtime.queries.assignmentSummary(props.assignment.id).catch(() => null),
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
            href={`/instructor/courses/${props.courseReference}/assignments/${assignmentRouteReference(props.assignment.reference)}/access`}
          >
            Access and modifiers
          </A>
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
  const runtime = useApiRuntime();
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
    (cursor) => runtime.client.listAssignments(props.courseId, cursor),
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
                : "Your instructor has not published an assignment for this course yet."}
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

export function CourseAssignmentsPage(): JSX.Element {
  const runtime = useApiRuntime();
  const session = useSessionBootstrap();
  const courseScope = useCourseThemeRouteData();
  const course = courseScope?.kind === "course" ? courseRouteData(courseScope).summary : undefined;
  const courseId = course?.id;
  const courseReference = course === undefined ? undefined : courseRouteReference(course.reference);
  const assignments = createAsync(() => {
    if (courseId === undefined) {
      return Promise.reject(new Error("Course route is unavailable"));
    }
    return runtime.queries.assignments(courseId);
  });
  const canManageCourse = (): boolean => {
    const current = session.state();
    const hasInstructorRole =
      current.kind === "authenticated" && current.session.account.role === "instructor";
    if (!hasInstructorRole || courseScope?.kind !== "course") return false;
    const role = courseRouteData(courseScope).summary.role;
    return role === "instructor";
  };
  async function reloadAssignments(): Promise<void> {
    if (courseId === undefined) return;
    await revalidate(runtime.queries.assignments.keyFor(courseId));
  }

  return (
    <section class="page" data-route-surface="courseAssignments">
      <Show when={!canManageCourse()}>
        <CourseEntryIdentity />
      </Show>
      <h2>Assignments</h2>
      <Suspense fallback={<p class="loading-state">Loading assignments...</p>}>
        <Show
          when={assignments()}
          keyed
          fallback={<p class="empty-state">No assignments are available in this course.</p>}
        >
          {(page) => {
            return courseId === undefined || courseReference === undefined ? (
              <p class="empty-state">No assignments are available in this course.</p>
            ) : (
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
