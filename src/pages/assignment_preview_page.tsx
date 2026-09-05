// assignment_preview_page.tsx - Instructor-only, identity-safe delivery inspection surface.

import { A, useParams } from "@solidjs/router";
import { For, Match, Show, Switch, createSignal, onMount, type JSX } from "solid-js";

import type { AssignmentReference } from "../../generated/api/AssignmentReference";
import type { InstructorPreviewSchedulePage } from "../../generated/api/InstructorPreviewSchedulePage";
import type { PreviewPlaneResponse } from "../../generated/api/PreviewPlaneResponse";
import type { EffectiveAssignmentPolicyView } from "../../generated/api/EffectiveAssignmentPolicyView";
import type { AssignmentEditNumber } from "../../generated/api/AssignmentEditNumber";
import type { CourseRouteView } from "../api/contracts";
import { ApiRequestError, PreviewPlaneConflictError } from "../api/http_client";
import { useApplicationApi } from "../api/application_api";
import { courseRouteView } from "../features/course_appearance/course_theme_context";
import { parseAssignmentReference } from "../navigation/public_route";
import { resolveAssignmentRoute } from "../navigation/resolved_route";
import { useRouteScopeData } from "../ribbon/route_scope_context";
import {
  emptyPatchDraft,
  policyRequest,
  type ModifierMode,
  type ModifierPatchDraft,
} from "./assignment_access/model";
import "./assignment_preview_page.css";

type PageState = "loading" | "ready" | "unavailable" | "offline" | "error";
type StudentViewScenarioBuilder = "selectedStudent" | "hypothetical";

function failureState(error: unknown): PageState {
  if (error instanceof ApiRequestError && (error.status === 401 || error.status === 403))
    return "unavailable";
  if (error instanceof TypeError || !navigator.onLine) return "offline";
  return "error";
}

function courseMoment(startDate: string): string {
  return `${startDate}T09:00:00.000`;
}

function normalizeCourseLocalMoment(value: string): string {
  if (/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}$/u.test(value)) return `${value}:00.000`;
  if (/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}$/u.test(value)) return `${value}.000`;
  return value;
}

function safeModifierError(error: unknown): string {
  if (
    error instanceof Error &&
    /^(Whole Assignment Attempt seconds|Attempt limit) (must be a positive whole number|is too large)\.$/u.test(
      error.message,
    )
  ) {
    return error.message;
  }
  return "Enter valid whole Assignment Attempt seconds and attempt-limit values, or leave them blank.";
}

/** Assignment-editor ETags retain HTTP quotes; preview requests use the generated edit number. */
function previewEditNumber(value: string): AssignmentEditNumber {
  const match = /^"([1-9][0-9]*)"$/u.exec(value);
  if (match === null)
    throw new Error("The assignment edit number is unavailable for a delivery check.");
  return match[1]!;
}

function titleCase(value: string): string {
  const words = value
    .replace(/[A-Z]/gu, (letter) => ` ${letter.toLowerCase()}`)
    .replace(/[_-]+/gu, " ");
  return `${words.charAt(0).toUpperCase()}${words.slice(1)}`;
}

function timeValue(value: string | null): string {
  return value === null ? "No limit" : value.replace("T", " ").replace(/\.000$/u, "");
}

function scheduleRows(
  effective_assignment_policy: EffectiveAssignmentPolicyView,
): ReadonlyArray<readonly [string, string, string]> {
  return [
    [
      "Opens",
      timeValue(effective_assignment_policy.available_at.value),
      titleCase(effective_assignment_policy.available_at.source),
    ],
    [
      "Due",
      timeValue(effective_assignment_policy.due_at.value),
      titleCase(effective_assignment_policy.due_at.source),
    ],
    [
      "Closes",
      timeValue(effective_assignment_policy.closes_at.value),
      titleCase(effective_assignment_policy.closes_at.source),
    ],
    [
      "Time limit",
      effective_assignment_policy.assignment_attempt_time_limit_seconds.value === null
        ? "No limit"
        : `${effective_assignment_policy.assignment_attempt_time_limit_seconds.value} seconds`,
      titleCase(effective_assignment_policy.assignment_attempt_time_limit_seconds.source),
    ],
    [
      "Attempt limit",
      effective_assignment_policy.attempt_limit.value === null
        ? "No limit"
        : String(effective_assignment_policy.attempt_limit.value),
      titleCase(effective_assignment_policy.attempt_limit.source),
    ],
    [
      "Late work",
      titleCase(effective_assignment_policy.late_work_rule.value),
      titleCase(effective_assignment_policy.late_work_rule.source),
    ],
    [
      "Deadline",
      titleCase(effective_assignment_policy.assignment_deadline_rule.value),
      titleCase(effective_assignment_policy.assignment_deadline_rule.source),
    ],
  ];
}

function ScheduleTable(props: {
  readonly effective_assignment_policy: EffectiveAssignmentPolicyView;
  readonly label: string;
}): JSX.Element {
  return (
    <table class="preview-effective_assignment_policy-table" aria-label={props.label}>
      <tbody>
        <For each={scheduleRows(props.effective_assignment_policy)}>
          {(row) => (
            <tr>
              <th scope="row">{row[0]}</th>
              <td>{row[1]}</td>
              <td class="preview-source">{row[2]}</td>
            </tr>
          )}
        </For>
      </tbody>
    </table>
  );
}

function PreviewResult(props: {
  readonly response: PreviewPlaneResponse;
  readonly headingRef: (element: HTMLHeadingElement) => void;
}): JSX.Element {
  const evaluation = props.response.evaluation;
  const accommodation = props.response.accommodation;
  if (evaluation.kind === "denied") {
    return (
      <section class="preview-result" aria-labelledby="preview-result-heading">
        <h2 id="preview-result-heading" ref={props.headingRef} tabIndex={-1}>
          Delivery check
        </h2>
        <p role="alert">
          This Student View Scenario does not currently have access to this assignment.
        </p>
      </section>
    );
  }
  return (
    <section class="preview-result" aria-labelledby="preview-result-heading">
      <h2 id="preview-result-heading" ref={props.headingRef} tabIndex={-1}>
        Resolved delivery
      </h2>
      <div class="preview-result-grid">
        <section>
          <h3>Student View Scenario</h3>
          <p>
            {titleCase(evaluation.student_view_scenario.origin)} scenario; Student View Scenario
            Admission: {titleCase(evaluation.student_view_scenario_admission)}.
          </p>
          <p>This scenario resolves the Assignment policy without identifying a person.</p>
          <ScheduleTable
            label="Resolved delivery effective_assignment_policy and source layers"
            effective_assignment_policy={evaluation.effective_assignment_policy}
          />
        </section>
        <section>
          <h3>Disclosure</h3>
          <ul class="preview-student_feedback_release-list">
            <For each={evaluation.student_feedback_release}>
              {(availability) => (
                <li>
                  <strong>{titleCase(availability.moment)}</strong>
                  <Show
                    when={availability.kind === "available"}
                    fallback={<span> Disclosure boundary unavailable.</span>}
                  >
                    <span>
                      {availability.kind === "available"
                        ? ` Score ${availability.flags.score_shown ? "shown" : "withheld"}; correctness ${availability.flags.correctness_shown ? "shown" : "withheld"}; feedback ${availability.flags.feedback_shown ? "shown" : "withheld"}; Question Answer ${availability.flags.question_answer_shown ? "shown" : "withheld"}; Answer Explanation ${availability.flags.question_answer_explanation_shown ? "shown" : "withheld"}; statistics ${availability.flags.statistics_shown ? "shown" : "withheld"}.`
                        : ""}
                    </span>
                  </Show>
                </li>
              )}
            </For>
          </ul>
        </section>
      </div>
      <Show when={accommodation} keyed>
        {(comparison) => (
          <section class="preview-accommodation" aria-labelledby="preview-accommodation-heading">
            <h3 id="preview-accommodation-heading">Accommodation effect</h3>
            <div class="preview-before-after">
              <section>
                <h4>Before</h4>
                <ScheduleTable
                  label="Delivery before accommodation"
                  effective_assignment_policy={comparison.before}
                />
              </section>
              <section>
                <h4>After</h4>
                <ScheduleTable
                  label="Delivery after accommodation"
                  effective_assignment_policy={comparison.after}
                />
              </section>
            </div>
          </section>
        )}
      </Show>
    </section>
  );
}

interface AssignmentPreviewContentProps {
  readonly course: CourseRouteView["summary"];
}

/** The page sends only public C-/A-/M- locators; all delivery facts come back from the server. */
function AssignmentPreviewContent(props: AssignmentPreviewContentProps): JSX.Element {
  const runtime = useApplicationApi();
  const params = useParams();
  const course = (): CourseRouteView["summary"] => props.course;
  const assignment = (): AssignmentReference | undefined => {
    const reference = params["assignmentRef"];
    return reference === undefined ? undefined : (parseAssignmentReference(reference) ?? undefined);
  };
  const [state, setState] = createSignal<PageState>("loading");
  const [editNumber, setEditNumber] = createSignal<AssignmentEditNumber>();
  const [effective_assignment_policy, setSchedule] = createSignal<InstructorPreviewSchedulePage>();
  const [cursor, setCursor] = createSignal<string>();
  const [studentViewScenarioBuilder, setStudentViewScenarioBuilder] =
    createSignal<StudentViewScenarioBuilder>("selectedStudent");
  const [selectedStudentMembership, setSelectedStudentMembership] = createSignal("");
  const [moment, setMoment] = createSignal("");
  const [modifierMode, setModifierMode] = createSignal<ModifierMode>("extend_only");
  const [modifierDraft, setModifierDraft] = createSignal<ModifierPatchDraft>(emptyPatchDraft());
  const [result, setResult] = createSignal<PreviewPlaneResponse>();
  const [message, setMessage] = createSignal("");
  const [modifierError, setModifierError] = createSignal("");
  const [busy, setBusy] = createSignal(false);
  const [needsReload, setNeedsReload] = createSignal(false);
  let resultHeading: HTMLHeadingElement | undefined;

  async function load(nextCursor?: string): Promise<void> {
    const selectedCourse = course();
    const selectedAssignment = assignment();
    if (
      selectedCourse === undefined ||
      selectedAssignment === undefined ||
      selectedCourse.role !== "instructor"
    ) {
      setState("unavailable");
      return;
    }
    setState("loading");
    try {
      let activeEditNumber = editNumber();
      if (activeEditNumber === undefined) {
        const resolved = await resolveAssignmentRoute(runtime.client, selectedAssignment);
        if (resolved.courseId !== selectedCourse.id) {
          setState("unavailable");
          return;
        }
        const editor = await runtime.client.getAssignmentWorkspace(
          selectedCourse.id,
          resolved.assignmentId,
        );
        if (editor.courseId !== selectedCourse.id || editor.id !== resolved.assignmentId) {
          setState("unavailable");
          return;
        }
        activeEditNumber = previewEditNumber(editor.revision);
      }
      const page = await runtime.client.listPreviewSchedule(
        selectedCourse.reference,
        selectedAssignment,
        activeEditNumber,
        nextCursor,
        25,
      );
      setEditNumber(page.edit_number);
      setSchedule(page);
      setCursor(page.next_cursor ?? undefined);
      if (moment().length === 0) setMoment(courseMoment(selectedCourse.term.startDate));
      setNeedsReload(false);
      setState("ready");
    } catch (error: unknown) {
      setMessage("The delivery check could not load. Try again.");
      setState(failureState(error));
    }
  }

  async function reloadAfterConflict(): Promise<void> {
    setNeedsReload(false);
    setEditNumber(undefined);
    setResult(undefined);
    await load();
    if (state() === "ready") {
      setMessage("The latest assignment revision is loaded. Your hypothetical draft is preserved.");
    }
  }

  function updateModifierLimit(
    field: "assignmentAttemptTimeLimitSeconds" | "attemptLimit",
    value: string,
  ): void {
    if (field === "assignmentAttemptTimeLimitSeconds") {
      setModifierDraft((current) => ({
        ...current,
        assignmentAttemptTimeLimitSeconds:
          value.length === 0 ? { kind: "inherit", value } : { kind: "set", value },
      }));
      return;
    }
    setModifierDraft((current) => ({
      ...current,
      attemptLimit: value.length === 0 ? { kind: "inherit", value } : { kind: "set", value },
    }));
  }

  function buildHypotheticalStudentViewScenarioModifiers():
    ReturnType<typeof policyRequest> | undefined {
    try {
      return policyRequest(modifierMode(), modifierDraft());
    } catch (error: unknown) {
      setModifierError(safeModifierError(error));
      return undefined;
    }
  }

  async function resolvePreview(): Promise<void> {
    const selectedCourse = course();
    const selectedAssignment = assignment();
    const activeEditNumber = editNumber();
    if (
      selectedCourse === undefined ||
      selectedAssignment === undefined ||
      activeEditNumber === undefined
    )
      return;
    const selectedMoment = normalizeCourseLocalMoment(moment());
    setBusy(true);
    setMessage("");
    setModifierError("");
    try {
      let response: PreviewPlaneResponse;
      if (studentViewScenarioBuilder() === "selectedStudent") {
        response = await runtime.client.constructSelectedStudentViewScenario(
          selectedCourse.reference,
          selectedAssignment,
          activeEditNumber,
          {
            selected_student_membership: selectedStudentMembership(),
            selected_moment: { value: selectedMoment, time_zone: selectedCourse.term.timeZone },
          },
        );
      } else {
        const modifiers = buildHypotheticalStudentViewScenarioModifiers();
        if (modifiers === undefined) return;
        response = await runtime.client.constructHypotheticalStudentViewScenario(
          selectedCourse.reference,
          selectedAssignment,
          activeEditNumber,
          {
            selected_moment: { value: selectedMoment, time_zone: selectedCourse.term.timeZone },
            modifiers,
          },
        );
      }
      setResult(response);
      queueMicrotask(() => {
        resultHeading?.scrollIntoView({ block: "start" });
        resultHeading?.focus({ preventScroll: true });
      });
      setNeedsReload(false);
      setMessage("Delivery preview resolved from the current course policy.");
    } catch (error: unknown) {
      if (error instanceof PreviewPlaneConflictError) {
        setResult(undefined);
        setNeedsReload(true);
        setMessage(
          "This assignment changed elsewhere. Your hypothetical draft is preserved; reload the effective_assignment_policy, then retry.",
        );
      } else if (error instanceof TypeError || !navigator.onLine) {
        setMessage("Preview is unavailable while offline. Your hypothetical draft is preserved.");
      } else {
        setMessage(
          "The preview could not be resolved. Check the selected Student membership and try again.",
        );
      }
    } finally {
      setBusy(false);
    }
  }

  onMount(() => void load());
  return (
    <section
      class="page assignment-preview-page"
      data-route-surface="assignmentPreview"
      aria-live="polite"
    >
      <p class="preview-only-cue">Preview only - no Student work or grades are created.</p>
      <p class="eyebrow">Instructor delivery inspection</p>
      <h1>Assignment delivery check</h1>
      <Show when={course() && assignment()}>
        <A
          class="quiet-link"
          href={`/instructor/courses/${course().reference}/assignments/${assignment()!}/policies`}
        >
          Return to assignment policies
        </A>
      </Show>
      <p role="status" class="preview-status">
        {message()}
      </p>
      <Show when={needsReload()}>
        <button type="button" onClick={() => void reloadAfterConflict()}>
          Reload latest assignment revision
        </button>
      </Show>
      <Switch>
        <Match when={state() === "loading"}>
          <p role="status">Loading effective_assignment_policy inspection...</p>
        </Match>
        <Match when={state() === "offline"}>
          <button type="button" onClick={() => void load()}>
            Reconnect and retry
          </button>
        </Match>
        <Match when={state() === "unavailable"}>
          <p role="alert">This assignment delivery check is unavailable.</p>
        </Match>
        <Match when={state() === "error"}>
          <p role="alert">The assignment delivery check could not load.</p>
          <button class="primary-action" type="button" onClick={() => void load()}>
            Try again
          </button>
        </Match>
        <Match when={state() === "ready"}>
          <div class="preview-workspace">
            <section
              class="preview-panel"
              aria-labelledby="preview-effective_assignment_policy-heading"
            >
              <h2 id="preview-effective_assignment_policy-heading">
                Schedule and active_student_course_membership
              </h2>
              <table class="preview-roster-table">
                <thead>
                  <tr>
                    <th>Student</th>
                    <th>Assignment Access</th>
                    <th>Due</th>
                    <th>Source</th>
                  </tr>
                </thead>
                <tbody>
                  <For each={effective_assignment_policy()?.rows ?? []}>
                    {(row) => (
                      <tr>
                        <td>{row.display}</td>
                        <td>
                          {row.kind === "granted"
                            ? titleCase(row.active_student_course_membership)
                            : "No Assignment Access"}
                        </td>
                        <td>
                          {row.kind === "granted"
                            ? timeValue(row.effective_assignment_policy.due_at.value)
                            : "Withheld"}
                        </td>
                        <td>
                          {row.kind === "granted"
                            ? titleCase(row.effective_assignment_policy.due_at.source)
                            : "-"}
                        </td>
                      </tr>
                    )}
                  </For>
                </tbody>
              </table>
              <Show when={cursor()}>
                <button type="button" onClick={() => void load(cursor())}>
                  Load next effective_assignment_policy page
                </button>
              </Show>
            </section>
            <section class="preview-panel" aria-labelledby="student-view-scenario-heading">
              <h2 id="student-view-scenario-heading">Build a Student View Scenario</h2>
              <div class="student-view-scenario-form">
                <fieldset class="student-view-scenario-origin">
                  <legend>Scenario origin</legend>
                  <label>
                    <input
                      type="radio"
                      name="student-view-scenario-origin"
                      checked={studentViewScenarioBuilder() === "selectedStudent"}
                      onInput={() => setStudentViewScenarioBuilder("selectedStudent")}
                    />{" "}
                    Selected Student membership
                  </label>
                  <label>
                    <input
                      type="radio"
                      name="student-view-scenario-origin"
                      checked={studentViewScenarioBuilder() === "hypothetical"}
                      onInput={() => setStudentViewScenarioBuilder("hypothetical")}
                    />{" "}
                    Hypothetical Student View Scenario Modifiers
                  </label>
                </fieldset>
                <label class="preview-field preview-moment-field">
                  Selected course-local moment
                  <input
                    type="datetime-local"
                    step="1"
                    value={moment()}
                    onInput={(event) => setMoment(event.currentTarget.value)}
                    required
                  />
                  <span>{course()?.term.timeZone} (course zone)</span>
                </label>
                <div class="student-view-scenario-target">
                  <Show when={studentViewScenarioBuilder() === "selectedStudent"}>
                    <label class="preview-field">
                      Student membership reference
                      <select
                        value={selectedStudentMembership()}
                        onInput={(event) => setSelectedStudentMembership(event.currentTarget.value)}
                        required
                      >
                        <option value="">Select a Student membership</option>
                        <For each={effective_assignment_policy()?.rows ?? []}>
                          {(row) => <option value={row.membership}>{row.display}</option>}
                        </For>
                      </select>
                      <span>
                        The selected-Student membership is used only for this request; the returned
                        Student View Scenario identifies no person.
                      </span>
                    </label>
                  </Show>
                </div>
                <Show when={studentViewScenarioBuilder() === "hypothetical"}>
                  <fieldset class="preview-modifier-controls">
                    <legend>Hypothetical Student View Scenario Modifiers</legend>
                    <p id="preview-modifier-help">
                      Dates inherit the assignment policy. Leave either numeric value blank to
                      inherit it too.
                    </p>
                    <fieldset>
                      <legend>Modifier application rule</legend>
                      <label>
                        <input
                          type="radio"
                          name="preview-modifier-mode"
                          value="extend_only"
                          checked={modifierMode() === "extend_only"}
                          onInput={() => setModifierMode("extend_only")}
                        />{" "}
                        Extend only
                      </label>
                      <label>
                        <input
                          type="radio"
                          name="preview-modifier-mode"
                          value="replace"
                          checked={modifierMode() === "replace"}
                          onInput={() => setModifierMode("replace")}
                        />{" "}
                        Replace
                      </label>
                    </fieldset>
                    <label class="preview-field">
                      Whole Assignment Attempt seconds
                      <input
                        type="number"
                        min="1"
                        step="1"
                        inputmode="numeric"
                        value={modifierDraft().assignmentAttemptTimeLimitSeconds.value}
                        aria-describedby="preview-modifier-help preview-modifier-error"
                        aria-invalid={modifierError().length > 0}
                        onInput={(event) =>
                          updateModifierLimit(
                            "assignmentAttemptTimeLimitSeconds",
                            event.currentTarget.value,
                          )
                        }
                      />
                    </label>
                    <label class="preview-field">
                      Attempt limit
                      <input
                        type="number"
                        min="1"
                        step="1"
                        inputmode="numeric"
                        value={modifierDraft().attemptLimit.value}
                        aria-describedby="preview-modifier-help preview-modifier-error"
                        aria-invalid={modifierError().length > 0}
                        onInput={(event) =>
                          updateModifierLimit("attemptLimit", event.currentTarget.value)
                        }
                      />
                    </label>
                    <Show when={modifierError()}>
                      <p id="preview-modifier-error" role="alert">
                        {modifierError()}
                      </p>
                    </Show>
                  </fieldset>
                </Show>
                <button
                  class="primary-action preview-submit"
                  type="button"
                  disabled={
                    busy() ||
                    moment().length === 0 ||
                    (studentViewScenarioBuilder() === "selectedStudent" &&
                      selectedStudentMembership().length === 0)
                  }
                  onClick={() => void resolvePreview()}
                >
                  {busy() ? "Checking delivery..." : "Check assignment delivery"}
                </button>
              </div>
            </section>
          </div>
          <Show when={result()} keyed>
            {(response) => (
              <PreviewResult
                response={response}
                headingRef={(element) => {
                  resultHeading = element;
                }}
              />
            )}
          </Show>
        </Match>
      </Switch>
    </section>
  );
}

/** Content owns its deferred course boundary; the persistent shell never gates this loader. */
export function AssignmentPreviewPage(): JSX.Element {
  const routeData = useRouteScopeData();
  const course = (): CourseRouteView["summary"] | undefined => {
    const data = routeData();
    return data?.kind === "course" ? courseRouteView(data).summary : undefined;
  };
  return (
    <Show
      when={course()}
      keyed
      fallback={
        <section class="page assignment-preview-page" data-route-surface="assignmentPreview">
          <p class="loading-state" role="status">
            Loading assignment delivery check...
          </p>
        </section>
      }
    >
      {(loadedCourse) => <AssignmentPreviewContent course={loadedCourse} />}
    </Show>
  );
}
