// gradebook_page.tsx - compact instructor projection with opt-in run history.

import { useParams } from "@solidjs/router";
import { For, Show, createSignal, onCleanup, onMount, type JSX } from "solid-js";

import type { AssignmentRun } from "../../generated/api/AssignmentRun";
import type { CourseId } from "../../generated/api/CourseId";
import type { EnrollmentId } from "../../generated/api/EnrollmentId";
import type { GradebookSummaryRow } from "../../generated/api/GradebookSummaryRow";
import { useApiRuntime } from "../api/runtime";
import { formatPercentScore } from "../score_format";
import { CursorPageSession, type CursorPageSessionState } from "./cursor_page_session";
import { loadGradebookPage, loadGradebookRunHistory } from "./gradebook_page_model";

type GradebookState =
  | { readonly kind: "loading" }
  | ({ readonly kind: "ready" } & CursorPageSessionState<GradebookSummaryRow>)
  | { readonly kind: "error" };

type RunHistoryState =
  | { readonly kind: "loading" }
  | {
      readonly kind: "ready";
      readonly runs: ReadonlyArray<AssignmentRun>;
      readonly nextCursor: string | null;
    }
  | { readonly kind: "error" };

function formatActivity(timestamp: number | null): string {
  if (timestamp === null) {
    return "No activity yet";
  }
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(timestamp));
}

function formatRunStatus(run: AssignmentRun): string {
  if (run.completedAt === null) {
    return "In progress";
  }
  return run.score === null ? "Completed" : `Completed * ${formatPercentScore(run.score)}`;
}

function gradebookRowKey(row: GradebookSummaryRow): string {
  return `${row.assignmentId}:${row.enrollmentId}`;
}

function gradebookHistoryControlId(row: GradebookSummaryRow): string {
  return `gradebook-history-control-${gradebookRowKey(row)}`;
}

interface GradebookCoursePageProps {
  readonly courseId: CourseId;
}

function GradebookCoursePage(props: GradebookCoursePageProps): JSX.Element {
  const runtime = useApiRuntime();
  const [gradebook, setGradebook] = createSignal<GradebookState>({ kind: "loading" });
  const [histories, setHistories] = createSignal<Readonly<Record<string, RunHistoryState>>>({});
  const [announcement, setAnnouncement] = createSignal("");
  let gradebookSession: CursorPageSession<GradebookSummaryRow> | undefined;
  let paginationRecoveryButton: HTMLButtonElement | undefined;
  let disposed = false;
  let loadGeneration = 0;

  const readyGradebook = (): Extract<GradebookState, { readonly kind: "ready" }> | undefined => {
    const state = gradebook();
    return state.kind === "ready" ? state : undefined;
  };

  async function loadGradebook(): Promise<void> {
    const generation = ++loadGeneration;
    const courseId = props.courseId;
    setGradebook({ kind: "loading" });
    try {
      const page = await loadGradebookPage(runtime.client, courseId);
      if (disposed || generation !== loadGeneration) return;
      let priorCount = page.items.length;
      gradebookSession = new CursorPageSession(
        page,
        (cursor) => loadGradebookPage(runtime.client, courseId, cursor),
        gradebookRowKey,
        (next) => {
          if (disposed || generation !== loadGeneration) return;
          const addedCount = next.items.length - priorCount;
          const shownCount = next.items.length;
          let message: string;
          if (next.loading) {
            message = "Loading more gradebook records...";
          } else if (next.error?.kind === "transport") {
            message = `Could not load more gradebook records. The ${shownCount} already shown are still available.`;
          } else if (next.error?.kind === "protocol") {
            message = `Gradebook pagination stopped because the next page was not distinct. The ${shownCount} already shown are still available.`;
          } else if (next.nextCursor === null) {
            message = `All ${shownCount} gradebook records are shown.`;
          } else {
            message = `Loaded ${addedCount} more gradebook records. ${shownCount} records shown.`;
          }
          priorCount = shownCount;
          setGradebook({ kind: "ready", ...next });
          setAnnouncement(next.error === null ? message : "");
          if (next.error !== null) {
            queueMicrotask(() => paginationRecoveryButton?.focus());
          }
        },
      );
      const initialState = gradebookSession.state;
      setGradebook({ kind: "ready", ...initialState });
      setAnnouncement(
        initialState.nextCursor === null
          ? `All ${initialState.items.length} gradebook records are shown.`
          : initialState.items.length === 1
            ? "Gradebook loaded with 1 assignment record."
            : `Gradebook loaded with ${initialState.items.length} assignment records.`,
      );
    } catch {
      if (disposed || generation !== loadGeneration) return;
      setGradebook({ kind: "error" });
      setAnnouncement("The gradebook could not load. You can try again.");
    }
  }

  async function loadMoreGradebook(): Promise<void> {
    const session = gradebookSession;
    const generation = loadGeneration;
    if (session === undefined) return;
    const appended = await session.loadMore();
    if (disposed || generation !== loadGeneration || session !== gradebookSession) return;
    const firstAppended = appended[0];
    if (firstAppended === undefined) return;
    queueMicrotask(() => {
      if (disposed || generation !== loadGeneration || session !== gradebookSession) return;
      document.getElementById(gradebookHistoryControlId(firstAppended))?.focus();
    });
  }

  async function retryLoadMoreGradebook(): Promise<void> {
    const session = gradebookSession;
    const generation = loadGeneration;
    if (session === undefined) return;
    const appended = await session.retry();
    if (disposed || generation !== loadGeneration || session !== gradebookSession) return;
    const firstAppended = appended[0];
    if (firstAppended === undefined) return;
    queueMicrotask(() => {
      if (disposed || generation !== loadGeneration || session !== gradebookSession) return;
      document.getElementById(gradebookHistoryControlId(firstAppended))?.focus();
    });
  }

  async function loadHistory(enrollmentId: EnrollmentId, cursor?: string): Promise<void> {
    const key = enrollmentId;
    const previous = histories()[key];
    setHistories((current) => ({ ...current, [key]: { kind: "loading" } }));
    try {
      const page = await loadGradebookRunHistory(runtime.client, enrollmentId, cursor);
      if (disposed) return;
      const previousRuns = cursor === undefined || previous?.kind !== "ready" ? [] : previous.runs;
      const runs = [...previousRuns, ...page.items];
      setHistories((current) => ({
        ...current,
        [key]: { kind: "ready", runs, nextCursor: page.nextCursor },
      }));
      setAnnouncement(
        page.items.length === 0
          ? "No additional runs were found."
          : `Run history updated with ${page.items.length} record${page.items.length === 1 ? "" : "s"}.`,
      );
    } catch {
      if (disposed) return;
      setHistories((current) => ({ ...current, [key]: { kind: "error" } }));
      setAnnouncement("Run history could not load. You can try again.");
    }
  }

  function openHistory(enrollmentId: EnrollmentId): void {
    const existing = histories()[enrollmentId];
    if (existing === undefined) {
      void loadHistory(enrollmentId);
    }
  }

  onMount(() => void loadGradebook());
  onCleanup(() => {
    disposed = true;
    loadGeneration += 1;
    gradebookSession = undefined;
    paginationRecoveryButton = undefined;
    setHistories({});
  });

  return (
    <section class="page gradebook-page" data-route-surface="gradebook">
      <p class="eyebrow">Course progress</p>
      <h1>Gradebook</h1>
      <p class="page-lede">
        A compact view of assignment progress. Open a learner's run history only when you need the
        detail.
      </p>
      <p class="gradebook-status" role="status" aria-live="polite" aria-atomic="true">
        {announcement()}
      </p>

      <Show when={gradebook().kind === "loading"}>
        <p class="loading-state" role="status">
          Loading gradebook...
        </p>
      </Show>
      <Show when={gradebook().kind === "error"}>
        <section class="route-error" role="alert">
          <p class="eyebrow">Gradebook unavailable</p>
          <h2>Progress is still safely recorded</h2>
          <p>Check your connection, then try loading this view again.</p>
          <button class="primary-action" type="button" onClick={() => void loadGradebook()}>
            Try again
          </button>
        </section>
      </Show>
      <Show when={readyGradebook()}>
        {(ready) => (
          <Show
            when={ready().items.length > 0 || ready().nextCursor !== null}
            fallback={
              <section class="gradebook-empty" aria-label="No gradebook records">
                <h2>No assignment progress yet</h2>
                <p>When learners begin an assignment, their progress will appear here.</p>
              </section>
            }
          >
            <Show when={ready().nextCursor !== null || ready().error !== null}>
              <a class="skip-link" href="#gradebook-pagination" target="_self">
                Skip to load more gradebook records
              </a>
            </Show>
            <div
              class="gradebook-table-wrap"
              role="region"
              aria-label="Gradebook records"
              aria-busy={ready().loading}
            >
              <table class="gradebook-table">
                <thead>
                  <tr>
                    <th scope="col">Assignment</th>
                    <th scope="col">Learner ID</th>
                    <th scope="col">Best</th>
                    <th scope="col">Latest</th>
                    <th scope="col">Completed</th>
                    <th scope="col">Last activity</th>
                    <th scope="col">
                      <span class="sr-only">Run history</span>
                    </th>
                  </tr>
                </thead>
                <tbody>
                  <For each={ready().items}>
                    {(row) => {
                      const history = (): RunHistoryState | undefined =>
                        histories()[row.enrollmentId];
                      const readyHistory = ():
                        Extract<RunHistoryState, { readonly kind: "ready" }> | undefined => {
                        const state = history();
                        return state?.kind === "ready" ? state : undefined;
                      };
                      return (
                        <>
                          <tr class="gradebook-row">
                            <th data-label="Assignment" scope="row">
                              {row.assignmentTitle}
                            </th>
                            <td data-label="Learner ID">
                              <code>{row.studentId}</code>
                            </td>
                            <td data-label="Best">{formatPercentScore(row.summary.bestScore)}</td>
                            <td data-label="Latest">
                              {formatPercentScore(row.summary.latestScore)}
                            </td>
                            <td data-label="Completed">{row.summary.completedRunCount}</td>
                            <td data-label="Last activity">
                              {formatActivity(row.summary.lastActivityAt)}
                            </td>
                            <td class="gradebook-history-control">
                              <button
                                id={gradebookHistoryControlId(row)}
                                class="quiet-action"
                                type="button"
                                aria-expanded={history() !== undefined}
                                aria-controls={`run-history-${row.enrollmentId}`}
                                onClick={() => openHistory(row.enrollmentId)}
                              >
                                View run history
                              </button>
                            </td>
                          </tr>
                          <Show when={history()}>
                            {(state) => (
                              <tr class="gradebook-history-row">
                                <td colSpan={7}>
                                  <section
                                    id={`run-history-${row.enrollmentId}`}
                                    aria-label={`Run history for learner ${row.studentId}`}
                                  >
                                    <Show when={state().kind === "loading"}>
                                      <p role="status">Loading run history...</p>
                                    </Show>
                                    <Show when={state().kind === "error"}>
                                      <div class="inline-error" role="alert">
                                        <p>Run history could not load.</p>
                                        <button
                                          class="quiet-action"
                                          type="button"
                                          onClick={() => void loadHistory(row.enrollmentId)}
                                        >
                                          Try history again
                                        </button>
                                      </div>
                                    </Show>
                                    <Show when={readyHistory()}>
                                      {(loaded) => (
                                        <>
                                          <Show
                                            when={loaded().runs.length > 0}
                                            fallback={<p>No runs have been started yet.</p>}
                                          >
                                            <ul class="run-history-list">
                                              <For each={loaded().runs}>
                                                {(run) => (
                                                  <li>
                                                    Run {run.runNumber}: {formatRunStatus(run)} *
                                                    started {formatActivity(run.startedAt)}
                                                  </li>
                                                )}
                                              </For>
                                            </ul>
                                          </Show>
                                          <Show when={loaded().nextCursor !== null}>
                                            <button
                                              class="quiet-action"
                                              type="button"
                                              onClick={() =>
                                                void loadHistory(
                                                  row.enrollmentId,
                                                  loaded().nextCursor ?? undefined,
                                                )
                                              }
                                            >
                                              Load older runs
                                            </button>
                                          </Show>
                                        </>
                                      )}
                                    </Show>
                                  </section>
                                </td>
                              </tr>
                            )}
                          </Show>
                        </>
                      );
                    }}
                  </For>
                </tbody>
              </table>
            </div>
            <section
              id="gradebook-pagination"
              class="gradebook-pagination"
              aria-label="Gradebook pagination"
              tabindex="-1"
            >
              <Show when={ready().error}>
                {(error) => (
                  <div class="inline-error" role="alert">
                    <p>
                      {error().kind === "transport"
                        ? `Could not load more gradebook records. The ${ready().items.length} already shown are still available.`
                        : `Gradebook pagination stopped because the next page was not distinct. The ${ready().items.length} already shown are still available.`}
                    </p>
                    <Show when={error().kind === "transport"}>
                      <button
                        class="quiet-action"
                        type="button"
                        ref={(element: HTMLButtonElement) => {
                          paginationRecoveryButton = element;
                        }}
                        onClick={() => void retryLoadMoreGradebook()}
                      >
                        Try loading more gradebook records again
                      </button>
                    </Show>
                    <Show when={error().kind === "protocol"}>
                      <button
                        class="quiet-action"
                        type="button"
                        ref={(element: HTMLButtonElement) => {
                          paginationRecoveryButton = element;
                        }}
                        onClick={() => void loadGradebook()}
                      >
                        Reload gradebook
                      </button>
                    </Show>
                  </div>
                )}
              </Show>
              <Show when={ready().nextCursor !== null && ready().error === null}>
                <button
                  class="quiet-action"
                  type="button"
                  disabled={ready().loading}
                  onClick={() => void loadMoreGradebook()}
                >
                  {ready().loading
                    ? "Loading more gradebook records..."
                    : "Load more gradebook records"}
                </button>
              </Show>
            </section>
          </Show>
        )}
      </Show>
    </section>
  );
}

/** Recreates course-owned pagination and history state whenever the route course changes. */
export function GradebookPage(): JSX.Element {
  const params = useParams();

  return (
    <Show
      when={params["courseId"]}
      keyed
      fallback={
        <section class="page gradebook-page" data-route-surface="gradebook">
          <section class="route-error" role="alert">
            <p class="eyebrow">Gradebook unavailable</p>
            <h1>Course route is missing</h1>
            <p>Return to your course list, then open the gradebook again.</p>
          </section>
        </section>
      }
    >
      {(courseId) => <GradebookCoursePage courseId={courseId} />}
    </Show>
  );
}
