// course_assignments_page.tsx - cursor-ready assignment list for one course.

import { A, createAsync, revalidate, useParams } from "@solidjs/router";
import { For, Show, Suspense, createSignal, type JSX } from "solid-js";

import type { CourseId } from "../../generated/api/CourseId";
import { useApiRuntime } from "../api/runtime";
import type { AssignmentSummary, CursorPage } from "../api/contracts";
import { CourseEntryIdentity } from "../features/course_appearance/course_entry_identity";
import {
  courseRouteData,
  useCourseThemeRouteData,
} from "../features/course_appearance/course_theme_context";
import { useSessionBootstrap } from "../auth/session_context";
import { CursorPageSession, type CursorPageSessionState } from "./cursor_page_session";

export interface AssignmentListProps {
  readonly courseId: CourseId;
  readonly initialPage: CursorPage<AssignmentSummary>;
  readonly reloadAssignments: () => Promise<void>;
}

function assignmentLinkId(assignmentId: string): string {
  const id = `assignment-review-${assignmentId}`;
  return id;
}

export function AssignmentList(props: AssignmentListProps): JSX.Element {
  const runtime = useApiRuntime();
  const [state, setState] = createSignal<CursorPageSessionState<AssignmentSummary>>({
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
    setAnnouncement(`Loaded ${count} more assignments. ${total} assignments shown.`);
  }

  function focusFirstAppended(appended: ReadonlyArray<AssignmentSummary>): void {
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
    if (current.nextCursor === null) return `All ${current.items.length} assignments are shown.`;
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
        fallback={<p class="empty-state">No assignments are available in this course.</p>}
      >
        <div class="card-grid" aria-busy={state().loading}>
          <For each={state().items}>
            {(assignment) => (
              <article class="course-card">
                <p class="card-kicker">Mastery practice</p>
                <h2>{assignment.title}</h2>
                <p>
                  {assignment.items.filter((item) => item.deliveryState === "active").length +
                    assignment.selectionGroups.reduce(
                      (count, group) => count + group.drawCount,
                      0,
                    )}{" "}
                  questions in each new run.
                </p>
                <A
                  class="primary-link"
                  href={`/courses/${assignment.courseId}/assignments/${assignment.id}`}
                  id={assignmentLinkId(assignment.id)}
                  ref={(element) => reviewLinks.set(assignment.id, element)}
                >
                  Review assignment
                </A>
              </article>
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
                Could not load more assignments. The {state().items.length} already shown are still
                available.
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
  const params = useParams();
  const courseScope = useCourseThemeRouteData();
  const assignments = createAsync(() => {
    const courseId = params["courseId"];
    if (courseId === undefined) {
      return Promise.reject(new Error("Course route is missing courseId"));
    }
    return runtime.queries.assignments(courseId);
  });
  const canManageCourse = (): boolean => {
    const current = session.state();
    const hasInstructorRole =
      current.kind === "authenticated" &&
      current.session.user.roles.some((role) => role === "instructor");
    if (!hasInstructorRole || courseScope?.kind !== "course") return false;
    const role = courseRouteData(courseScope).summary.role;
    return role === "instructor";
  };
  async function reloadAssignments(): Promise<void> {
    const courseId = params["courseId"];
    if (courseId === undefined) return;
    await revalidate(runtime.queries.assignments.keyFor(courseId));
  }

  return (
    <section class="page" data-route-surface="courseAssignments">
      <CourseEntryIdentity />
      <h2>Assignments</h2>
      <Show when={canManageCourse() && params["courseId"] !== undefined}>
        <nav class="course-management-nav" aria-label="Course management">
          <A href={`/instructor/courses/${params["courseId"] ?? ""}/assignments/new`}>
            New assignment
          </A>
          <A href={`/instructor/courses/${params["courseId"] ?? ""}/students`}>Students</A>
          <A href={`/instructor/courses/${params["courseId"] ?? ""}/gradebook`}>Gradebook</A>
          <A href={`/instructor/courses/${params["courseId"] ?? ""}/appearance`}>Appearance</A>
        </nav>
      </Show>
      <Suspense fallback={<p class="loading-state">Loading assignments...</p>}>
        <Show
          when={assignments()}
          keyed
          fallback={<p class="empty-state">No assignments are available in this course.</p>}
        >
          {(page) => {
            const courseId = params["courseId"];
            return courseId === undefined ? (
              <p class="empty-state">No assignments are available in this course.</p>
            ) : (
              <AssignmentList
                courseId={courseId}
                initialPage={page}
                reloadAssignments={reloadAssignments}
              />
            );
          }}
        </Show>
      </Suspense>
    </section>
  );
}
