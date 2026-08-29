// gradebook_page.tsx - roster-first, server-calculated Instructor Gradebook.

import { A, useLocation } from "@solidjs/router";
import { For, Show, createEffect, createMemo, createSignal, onCleanup, type JSX } from "solid-js";

import type { AssignmentReference } from "../../generated/api/AssignmentReference";
import type { CourseId } from "../../generated/api/CourseId";
import type { CourseMembershipReference } from "../../generated/api/CourseMembershipReference";
import type { CourseReference } from "../../generated/api/CourseReference";
import type { GradingOperationReference } from "../../generated/api/GradingOperationReference";
import type {
  CalculatedAssignmentCell,
  CalculatedGradebookRow,
} from "../api/decoders/calculated_gradebook";
import type { AssignmentInspectionChoice } from "../api/decoders/gradebook_selection";
import { useApiRuntime } from "../api/runtime";
import {
  courseRouteData,
  useCourseThemeRouteData,
} from "../features/course_appearance/course_theme_context";
import { assignmentRouteReference, courseRouteReference } from "../navigation/public_route";
import { formatPercentScore } from "../score_format";
import { assignmentWorkspacePath } from "./assignment_workspace/assignment_workspace_paths";
import {
  gradebookCellFocusId,
  inspectedStudentWorkUrl,
  parseGradebookRouteSearch,
  type GradebookRouteFilter,
  type GradebookRouteSearchResult,
} from "./gradebook_navigation";
import {
  GradebookPageSession,
  type GradebookPageGradebookState,
  type GradebookPageSelectionState,
  type GradebookPageState,
} from "./gradebook_page_model";
import { GradebookRunChooser } from "./gradebook_run_chooser";
import "./instructor_data_tables.css";

interface RunChooserRequest {
  readonly membership: CourseMembershipReference;
  readonly assignment: AssignmentReference;
  readonly operation?: GradingOperationReference;
  readonly studentLabel: string;
  readonly assignmentTitle: string;
  readonly trigger: HTMLButtonElement;
}

function formatActivity(timestamp: number): string {
  return new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" }).format(
    new Date(timestamp),
  );
}
function unavailableOutcome(reason: string): string {
  const labels: Readonly<Record<string, string>> = {
    noIncludedAssignments: "No assignments included",
    recalculating: "Recalculating",
    failed: "Needs attention",
    emptyAfterDrop: "No score after drop rules",
    zeroPossiblePoints: "No possible points",
  };
  return labels[reason] ?? "Unavailable";
}
function invalidRouteReason(
  result: Extract<GradebookRouteSearchResult, { readonly kind: "invalid" }>,
): string {
  if (result.reason === "malformedSearch") return "The Gradebook address is malformed.";
  if (result.reason === "unknownKey")
    return `The Gradebook address contains an unsupported filter: ${result.key ?? "unknown"}.`;
  if (result.reason === "duplicateKey")
    return `The Gradebook address repeats the ${result.key ?? "selected"} filter.`;
  if (result.reason === "multipleFilters") return "Choose one Gradebook filter at a time.";
  return `The ${result.key ?? "selected"} reference is not a valid public course reference.`;
}
function filterLabel(filter: GradebookRouteFilter | undefined): string {
  if (filter === undefined) return "All active Students";
  if (filter.kind === "assignment") return `Assignment ${filter.assignment}`;
  if (filter.kind === "student") return `Student ${filter.membership}`;
  return `Grading operation ${filter.operation}`;
}
function scoreLabel(cell: CalculatedAssignmentCell): string {
  if (cell.availability === "unavailable") return "Not assigned";
  if (cell.scoringStatus === "recalculating") return "Recalculating";
  if (cell.scoringStatus === "failed") return "Needs attention";
  return cell.selectedScore === null ? "No score" : formatPercentScore(cell.selectedScore);
}
function studentLabel(
  rows: ReadonlyArray<CalculatedGradebookRow>,
  membership: CourseMembershipReference,
): string {
  return rows.find((row) => row.membership === membership)?.displayLabel ?? `Student ${membership}`;
}
function assignmentTitle(
  rows: ReadonlyArray<CalculatedGradebookRow>,
  assignment: AssignmentReference,
): string {
  return (
    rows[0]?.assignmentCells.find((cell) => cell.assignment === assignment)?.title ??
    `Assignment ${assignment}`
  );
}

function InspectionChoiceActions(props: {
  readonly course: CourseReference;
  readonly membership: CourseMembershipReference;
  readonly assignment: AssignmentReference;
  readonly operation?: GradingOperationReference;
  readonly inspectionChoice: AssignmentInspectionChoice;
  readonly studentLabel: string;
  readonly assignmentTitle: string;
  readonly onChooseRun: (
    request: Omit<RunChooserRequest, "trigger">,
    trigger: HTMLButtonElement,
  ) => void;
}): JSX.Element {
  const selectedRun = ():
    Extract<AssignmentInspectionChoice, { readonly kind: "selectedRun" }> | undefined =>
    props.inspectionChoice.kind === "selectedRun" ? props.inspectionChoice : undefined;
  const chooseRun = ():
    Extract<AssignmentInspectionChoice, { readonly kind: "chooseRun" }> | undefined =>
    props.inspectionChoice.kind === "chooseRun" ? props.inspectionChoice : undefined;
  return (
    <>
      <Show when={selectedRun()}>
        {(choice) => (
          <A
            class="gradebook-inspection-link"
            href={inspectedStudentWorkUrl(
              props.course,
              props.membership,
              props.assignment,
              choice().run,
              props.operation,
            )}
          >
            Inspect submitted work
          </A>
        )}
      </Show>
      <Show when={chooseRun()}>
        {(choice) => (
          <button
            class="quiet-action gradebook-choose-run"
            type="button"
            onClick={(event) =>
              props.onChooseRun(
                {
                  membership: props.membership,
                  assignment: props.assignment,
                  operation: props.operation,
                  studentLabel: props.studentLabel,
                  assignmentTitle: props.assignmentTitle,
                },
                event.currentTarget,
              )
            }
          >
            Choose one of {choice().completedRunCount} submitted runs
          </button>
        )}
      </Show>
      <Show when={props.inspectionChoice.kind === "noSubmittedRun"}>
        <span class="gradebook-cell-next-step">No submitted work</span>
      </Show>
    </>
  );
}
function AssignmentCell(props: {
  readonly course: CourseReference;
  readonly row: CalculatedGradebookRow;
  readonly cell: CalculatedAssignmentCell;
  readonly operation?: GradingOperationReference;
  readonly onChooseRun: (
    request: Omit<RunChooserRequest, "trigger">,
    trigger: HTMLButtonElement,
  ) => void;
}): JSX.Element {
  const operationsHref = createMemo(() =>
    assignmentWorkspacePath(
      courseRouteReference(props.course),
      assignmentRouteReference(props.cell.assignment),
      "gradingOperations",
    ),
  );
  return (
    <td
      id={gradebookCellFocusId(props.row.membership, props.cell.assignment)}
      class="gradebook-assignment-cell"
      tabindex="-1"
      data-label={props.cell.title}
    >
      <span class="gradebook-cell-score">{scoreLabel(props.cell)}</span>
      <InspectionChoiceActions
        course={props.course}
        membership={props.row.membership}
        assignment={props.cell.assignment}
        operation={props.operation}
        inspectionChoice={props.cell.inspectionChoice}
        studentLabel={props.row.displayLabel}
        assignmentTitle={props.cell.title}
        onChooseRun={props.onChooseRun}
      />
      <Show when={props.cell.scoringStatus === "failed"}>
        <A class="gradebook-operations-link" href={operationsHref()}>
          Open grading operations
        </A>
      </Show>
    </td>
  );
}

function GradebookCoursePage(props: {
  readonly courseId: CourseId;
  readonly courseReference: CourseReference;
}): JSX.Element {
  const runtime = useApiRuntime();
  const location = useLocation();
  const [pageState, setPageState] = createSignal<GradebookPageState>({
    route: { kind: "valid", filter: undefined },
    gradebook: { kind: "loading" },
    operationSelection: { kind: "idle" },
  });
  const session = new GradebookPageSession(props.courseId, runtime.client, setPageState);
  const [runChooser, setRunChooser] = createSignal<RunChooserRequest>();
  const [announcement, setAnnouncement] = createSignal("");
  const routeSearch = createMemo(() => parseGradebookRouteSearch(location.search));
  const gradebook = createMemo(() => pageState().gradebook);
  const operationSelection = createMemo(() => pageState().operationSelection);
  const ready = createMemo(
    (): Extract<GradebookPageGradebookState, { readonly kind: "ready" }> | undefined => {
      const state = gradebook();
      return state.kind === "ready" ? state : undefined;
    },
  );
  const columns = createMemo(
    () => ready()?.rows[0]?.assignmentCells ?? ([] as ReadonlyArray<CalculatedAssignmentCell>),
  );
  const selectedOperation = createMemo(() => {
    const route = pageState().route;
    const filter = route.kind === "valid" ? route.filter : undefined;
    return filter?.kind === "operation" ? filter.operation : undefined;
  });
  const invalidRoute = createMemo(
    (): Extract<GradebookRouteSearchResult, { readonly kind: "invalid" }> | undefined => {
      const route = pageState().route;
      return route.kind === "invalid" ? route : undefined;
    },
  );
  const reloadRequired = createMemo(
    (): Extract<GradebookPageGradebookState, { readonly kind: "reloadRequired" }> | undefined => {
      const state = gradebook();
      return state.kind === "reloadRequired" ? state : undefined;
    },
  );
  const singleSelection = createMemo(
    (): Extract<GradebookPageSelectionState, { readonly kind: "singleStudent" }> | undefined => {
      const state = operationSelection();
      return state.kind === "singleStudent" ? state : undefined;
    },
  );
  const studentSelection = createMemo(
    (): Extract<GradebookPageSelectionState, { readonly kind: "studentSelection" }> | undefined => {
      const state = operationSelection();
      return state.kind === "studentSelection" ? state : undefined;
    },
  );
  let restoreHeadingFocus = false;
  let gradebookHeading: HTMLHeadingElement | undefined;
  function reloadGradebook(): void {
    restoreHeadingFocus = true;
    session.reload();
  }
  function openRunChooser(
    request: Omit<RunChooserRequest, "trigger">,
    trigger: HTMLButtonElement,
  ): void {
    setRunChooser({ ...request, trigger });
  }
  function dismissRunChooser(): void {
    const current = runChooser();
    setRunChooser(undefined);
    if (current?.trigger.isConnected)
      queueMicrotask(() => current.trigger.focus({ preventScroll: true }));
  }
  createEffect(() => {
    session.reset(routeSearch());
  });
  createEffect(() => {
    const route = pageState().route;
    const state = gradebook();
    if (route.kind === "invalid") {
      setAnnouncement("The Gradebook address needs correction before it can load.");
    } else if (state.kind === "loading") {
      setAnnouncement("Loading the current calculated Gradebook.");
    } else if (state.kind === "error") {
      setAnnouncement("The Gradebook could not load. You can try again.");
    } else if (state.kind === "reloadRequired") {
      setAnnouncement("The Gradebook changed and needs a fresh load.");
    } else if (state.kind === "ready" && state.moreError) {
      setAnnouncement("More Students could not load. The visible Gradebook remains available.");
    } else if (state.kind === "ready" && state.loadingMore) {
      setAnnouncement("Loading more Students.");
    } else if (state.kind === "ready") {
      setAnnouncement(
        `Loaded current course totals for ${state.rows.length} Student${state.rows.length === 1 ? "" : "s"}.`,
      );
    }
    if (restoreHeadingFocus && state.kind !== "loading") {
      restoreHeadingFocus = false;
      window.requestAnimationFrame(() => gradebookHeading?.focus({ preventScroll: true }));
    }
  });
  onCleanup(() => {
    session.dispose();
  });
  createEffect(() => {
    if (ready() === undefined) return;
    const target = window.location.hash.slice(1);
    if (!/^gradebook-cell-M-[1-9][0-9]{0,9}-A-[1-9][0-9]{0,9}$/u.test(target)) return;
    window.requestAnimationFrame(() =>
      document.getElementById(target)?.focus({ preventScroll: true }),
    );
  });
  function operationChoice(
    membership: CourseMembershipReference,
    assignment: AssignmentReference,
    inspectionChoice: AssignmentInspectionChoice,
    label?: string,
  ): JSX.Element {
    const rows = ready()?.rows ?? [];
    const visibleLabel = label ?? studentLabel(rows, membership);
    const title = assignmentTitle(rows, assignment);
    return (
      <article class="gradebook-selection-choice">
        <h3>{visibleLabel}</h3>
        <p>{title}</p>
        <InspectionChoiceActions
          course={props.courseReference}
          membership={membership}
          assignment={assignment}
          operation={selectedOperation()}
          inspectionChoice={inspectionChoice}
          studentLabel={visibleLabel}
          assignmentTitle={title}
          onChooseRun={openRunChooser}
        />
      </article>
    );
  }
  return (
    <section class="page gradebook-page" data-route-surface="gradebook">
      <p class="eyebrow">Course progress</p>
      <h1 ref={(element) => (gradebookHeading = element)} tabindex="-1">
        Gradebook
      </h1>
      <p class="page-lede">
        Current course totals and assignment scores calculated by the server. Open a submitted run
        to inspect the Student&apos;s exact response with solution-free grading evidence.
      </p>
      <p class="gradebook-status" role="status" aria-live="polite" aria-atomic="true">
        {announcement()}
      </p>
      <Show when={invalidRoute()}>
        {(invalid) => (
          <section class="route-error" role="alert">
            <p class="eyebrow">Gradebook address needs correction</p>
            <h2>Choose one valid Gradebook view</h2>
            <p>{invalidRouteReason(invalid())}</p>
            <A
              class="primary-action"
              href={`/instructor/courses/${props.courseReference}/gradebook`}
            >
              Open all active Students
            </A>
          </section>
        )}
      </Show>
      <Show when={gradebook().kind === "loading"}>
        <p class="loading-state">Loading calculated Gradebook...</p>
      </Show>
      <Show when={gradebook().kind === "error"}>
        <section class="route-error" role="alert">
          <p class="eyebrow">Gradebook unavailable</p>
          <h2>Course progress is still safely recorded</h2>
          <p>Check the live stack, then load the Gradebook again.</p>
          <button class="primary-action" type="button" onClick={reloadGradebook}>
            Try again
          </button>
        </section>
      </Show>
      <Show when={reloadRequired()}>
        {(reload) => (
          <section class="route-error" role="alert">
            <p class="eyebrow">Gradebook updated</p>
            <h2>Load the current course structure</h2>
            <p>{reload().reason}</p>
            <button class="primary-action" type="button" onClick={reloadGradebook}>
              Reload Gradebook
            </button>
          </section>
        )}
      </Show>
      <Show when={ready()}>
        {(current) => (
          <>
            <section class="gradebook-witness" aria-label="Gradebook calculation status">
              <div>
                <span>Calculation</span>
                <strong>
                  {current().page.mode === "totalPoints" ? "Total points" : "Weighted categories"}
                </strong>
              </div>
              <div>
                <span>Viewing</span>
                <strong>{filterLabel(current().filter)}</strong>
              </div>
              <div>
                <span>Observed</span>
                <strong>{formatActivity(current().page.observationTime)}</strong>
              </div>
              <div>
                <span>Students shown</span>
                <strong>{current().rows.length}</strong>
              </div>
            </section>
            <Show when={selectedOperation()}>
              {(operation) => (
                <section class="gradebook-selection" aria-labelledby="gradebook-selection-heading">
                  <p class="eyebrow">Grading operation</p>
                  <h2 id="gradebook-selection-heading">Select Student work to inspect</h2>
                  <p>
                    Choose one named Student and one exact submitted run for operation {operation()}
                    .
                  </p>
                  <Show when={operationSelection().kind === "loading"}>
                    <p class="loading-state">Loading affected Students...</p>
                  </Show>
                  <Show when={operationSelection().kind === "error"}>
                    <section class="inline-error" role="alert">
                      <p>Affected Students could not load. Try again to make an exact choice.</p>
                      <button
                        class="quiet-action"
                        type="button"
                        onClick={() => session.retrySelection()}
                      >
                        Try again
                      </button>
                    </section>
                  </Show>
                  <Show when={singleSelection()}>
                    {(selection) => {
                      return operationChoice(
                        selection().membership,
                        selection().assignment,
                        selection().inspectionChoice,
                      );
                    }}
                  </Show>
                  <Show when={studentSelection()}>
                    {(selection) => {
                      return (
                        <>
                          <div class="gradebook-selection-list">
                            <For each={selection().rows}>
                              {(row) =>
                                operationChoice(
                                  row.membership,
                                  row.assignment,
                                  row.inspectionChoice,
                                  row.displayLabel,
                                )
                              }
                            </For>
                          </div>
                          <Show when={selection().moreError}>
                            <div class="inline-error" role="alert">
                              <p>
                                More affected Students could not load. The listed choices remain
                                available.
                              </p>
                              <button
                                class="quiet-action"
                                type="button"
                                onClick={() => session.loadMoreSelection()}
                              >
                                Try loading more Students
                              </button>
                            </div>
                          </Show>
                          <Show when={selection().nextCursor !== null && !selection().moreError}>
                            <button
                              class="quiet-action"
                              type="button"
                              disabled={selection().loadingMore}
                              onClick={() => session.loadMoreSelection()}
                            >
                              {selection().loadingMore
                                ? "Loading more Students..."
                                : "Load more affected Students"}
                            </button>
                          </Show>
                        </>
                      );
                    }}
                  </Show>
                </section>
              )}
            </Show>
            <Show
              when={current().rows.length > 0}
              fallback={
                <section class="gradebook-empty" aria-label="No active Students">
                  <h2>No active Students yet</h2>
                  <p>Students will appear after they are connected to this live course.</p>
                </section>
              }
            >
              <div class="gradebook-table-wrap" role="region" aria-label="Calculated Gradebook">
                <table class="gradebook-table gradebook-table--calculated">
                  <thead>
                    <tr>
                      <th scope="col">Student</th>
                      <th scope="col">Course total</th>
                      <For each={columns()}>
                        {(cell) => (
                          <th scope="col">
                            <span>{cell.title}</span>
                            <Show when={!cell.included}>
                              <small>Excluded from total</small>
                            </Show>
                          </th>
                        )}
                      </For>
                    </tr>
                  </thead>
                  <tbody>
                    <For each={current().rows}>
                      {(row) => (
                        <tr class="gradebook-row">
                          <th scope="row">{row.displayLabel}</th>
                          <td data-label="Course total" class="gradebook-course-total">
                            <Show
                              when={row.outcome.status === "available" ? row.outcome : undefined}
                              fallback={
                                <span>
                                  {row.outcome.status === "unavailable"
                                    ? unavailableOutcome(row.outcome.reason)
                                    : "Unavailable"}
                                </span>
                              }
                            >
                              {(outcome) => (
                                <>
                                  <strong>{formatPercentScore(outcome().score)}</strong>
                                  <Show when={outcome().letter}>
                                    {(letter) => <span>{letter()}</span>}
                                  </Show>
                                </>
                              )}
                            </Show>
                          </td>
                          <For each={row.assignmentCells}>
                            {(cell) => (
                              <AssignmentCell
                                course={props.courseReference}
                                row={row}
                                cell={cell}
                                operation={selectedOperation()}
                                onChooseRun={openRunChooser}
                              />
                            )}
                          </For>
                        </tr>
                      )}
                    </For>
                  </tbody>
                </table>
              </div>
            </Show>
            <Show when={current().moreError}>
              <div class="inline-error" role="alert">
                <p>More Students could not load. The rows already shown remain current.</p>
                <button
                  class="quiet-action"
                  type="button"
                  onClick={() => session.loadMoreGradebook()}
                >
                  Try loading more Students
                </button>
              </div>
            </Show>
            <Show when={current().page.nextCursor !== null && !current().moreError}>
              <button
                class="quiet-action gradebook-load-more"
                type="button"
                disabled={current().loadingMore}
                onClick={() => session.loadMoreGradebook()}
              >
                {current().loadingMore ? "Loading more Students..." : "Load more Students"}
              </button>
            </Show>
          </>
        )}
      </Show>
      <Show when={runChooser()} keyed>
        {(request) => (
          <GradebookRunChooser
            client={runtime.client}
            courseId={props.courseId}
            course={props.courseReference}
            membership={request.membership}
            assignment={request.assignment}
            operation={request.operation}
            studentLabel={request.studentLabel}
            assignmentTitle={request.assignmentTitle}
            onDismiss={dismissRunChooser}
          />
        )}
      </Show>
    </section>
  );
}
/** Recreates course-owned Gradebook state whenever the route course changes. */
export function GradebookPage(): JSX.Element {
  const scopedRoute = useCourseThemeRouteData();
  const course = scopedRoute?.kind === "course" ? courseRouteData(scopedRoute).summary : undefined;
  return (
    <Show
      when={course}
      keyed
      fallback={
        <section class="page gradebook-page" data-route-surface="gradebook">
          <section class="route-error" role="alert">
            <p class="eyebrow">Gradebook unavailable</p>
            <h1>Course route is missing</h1>
            <p>Return to your course list, then open the Gradebook again.</p>
          </section>
        </section>
      }
    >
      {(loadedCourse) => (
        <GradebookCoursePage courseId={loadedCourse.id} courseReference={loadedCourse.reference} />
      )}
    </Show>
  );
}
