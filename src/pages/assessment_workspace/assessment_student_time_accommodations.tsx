// Lazy, explicit time-only saves for existing active Course roster Students.
import { For, Show, createEffect, createSignal, on, onCleanup, type JSX } from "solid-js";
import type { AssessmentStudentTimeAccommodation } from "../../../generated/api/AssessmentStudentTimeAccommodation";
import type { CourseRosterEntry } from "../../api/course_roster";
import { useApplicationApi } from "../../api/application_api";
import { ApiRequestError } from "../../api/http_client/error";
import { useAssessmentWorkspace } from "./assessment_workspace_live_page";

type TimeChoice = "standard" | "1.5" | "2" | "custom";

function durationDescription(seconds: number | null): string {
  if (seconds === null) return "Not calculated until this Assessment contains 1 to 250 Questions.";
  if (seconds === 86_400) return "24 hours";
  if (seconds % 3600 === 0) return `${seconds / 3600} hours`;
  if (seconds % 60 === 0) return `${seconds / 60} minutes`;
  return `${seconds} seconds`;
}

/** Does not load private teaching configuration until the Instructor opens it. */
export function AssessmentStudentTimeAccommodations(props: {
  readonly ready: boolean;
}): JSX.Element {
  const workspace = useAssessmentWorkspace();
  const api = useApplicationApi().client;
  const [open, setOpen] = createSignal(false);
  const [roster, setRoster] = createSignal<ReadonlyArray<CourseRosterEntry>>([]);
  const [rosterLoading, setRosterLoading] = createSignal(false);
  const [rosterLoaded, setRosterLoaded] = createSignal(false);
  const [student, setStudent] = createSignal("");
  const [configuration, setConfiguration] = createSignal<AssessmentStudentTimeAccommodation>();
  const [choice, setChoice] = createSignal<TimeChoice>("standard");
  const [custom, setCustom] = createSignal("");
  const [busy, setBusy] = createSignal(false);
  const [stale, setStale] = createSignal(false);
  const [message, setMessage] = createSignal("");
  let requestNumber = 0;
  let rosterRequestNumber = 0;

  function multiplier(): number | null | undefined {
    if (choice() === "standard") return null;
    const text = choice() === "custom" ? custom().trim() : choice();
    if (text === "") return undefined;
    const value = Number(text);
    return Number.isFinite(value) && value >= 1 ? value : undefined;
  }

  function accept(value: AssessmentStudentTimeAccommodation): void {
    setConfiguration(value);
    const time = value.timeMultiplier;
    setChoice(time === null ? "standard" : time === 1.5 ? "1.5" : time === 2 ? "2" : "custom");
    setCustom(time === null ? "" : String(time));
    setStale(false);
  }

  async function loadRoster(): Promise<void> {
    const request = ++rosterRequestNumber;
    setRosterLoading(true);
    setMessage("");
    try {
      const value = await api.getLiveCourseRoster(workspace.courseInstanceId);
      if (request !== rosterRequestNumber) return;
      setRoster(value.filter((entry) => entry.state === "activeStudent"));
      setRosterLoaded(true);
    } catch {
      if (request === rosterRequestNumber)
        setMessage("The active Student roster could not load. Try again.");
    } finally {
      if (request === rosterRequestNumber) setRosterLoading(false);
    }
  }

  async function loadConfiguration(rosterId: string): Promise<void> {
    const request = ++requestNumber;
    setConfiguration(undefined);
    setMessage("");
    setStale(false);
    if (rosterId === "") {
      setBusy(false);
      return;
    }
    setBusy(true);
    try {
      const value = await api.getAssessmentStudentTimeAccommodation(
        workspace.courseInstanceId,
        workspace.assessmentId,
        rosterId,
      );
      if (request === requestNumber && student() === rosterId) accept(value);
    } catch {
      if (request === requestNumber)
        setMessage("Student time settings could not load. Reload settings to try again.");
    } finally {
      if (request === requestNumber) setBusy(false);
    }
  }

  async function save(): Promise<void> {
    const current = configuration();
    const timeMultiplier = multiplier();
    if (current === undefined || timeMultiplier === undefined || !props.ready || busy() || stale())
      return;
    const rosterId = student();
    const request = ++requestNumber;
    setBusy(true);
    setMessage("");
    try {
      const value = await api.saveAssessmentStudentTimeAccommodation(
        workspace.courseInstanceId,
        workspace.assessmentId,
        rosterId,
        { timeMultiplier, expectedEditNumber: current.editNumber },
      );
      if (request !== requestNumber || student() !== rosterId) return;
      accept(value);
      setMessage(
        "Student time settings saved. Existing active Attempts keep their original time limit and deadline.",
      );
    } catch (error) {
      if (request !== requestNumber) return;
      if (error instanceof ApiRequestError && error.status === 412) {
        setStale(true);
        setMessage(
          "Another edit changed this Student's settings. Reload settings before saving again.",
        );
      } else {
        setMessage("Student time settings could not save. Reload settings before retrying.");
        setStale(true);
      }
    } finally {
      if (request === requestNumber) setBusy(false);
    }
  }

  // A saved base-policy change invalidates the finite preview. Reload from the
  // shared server calculation; never retain a derived-duration cache.
  createEffect(
    on([(): string => workspace.assessment().etag, open, student], () => {
      if (open() && student() !== "") void loadConfiguration(student());
    }),
  );
  onCleanup(() => {
    requestNumber += 1;
    rosterRequestNumber += 1;
  });

  return (
    <section
      class="assessment-editor-policy-panel"
      aria-labelledby="student-time-accommodations-heading"
    >
      <h2 id="student-time-accommodations-heading">Student time accommodations</h2>
      <button
        type="button"
        aria-expanded={open()}
        disabled={busy()}
        onClick={() => {
          setOpen(!open());
          if (open() && !rosterLoaded()) void loadRoster();
        }}
      >
        {open() ? "Hide Student time settings" : "Manage Student time settings"}
      </button>
      <Show when={open()}>
        <p>
          Apply a time multiplier after the Assessment's base time limit. Effective time is rounded
          up to a whole second and capped at 24 hours. Due and closing dates still apply separately.
        </p>
        <Show when={rosterLoading()}>
          <p role="status">Loading active Students...</p>
        </Show>
        <Show when={!rosterLoading() && !rosterLoaded()}>
          <button type="button" onClick={() => void loadRoster()}>
            Reload active Student roster
          </button>
        </Show>
        <Show when={rosterLoaded() && roster().length === 0}>
          <p>No active Students are available in this Course roster.</p>
        </Show>
        <Show when={roster().length > 0}>
          <label>
            Student roster ID
            <select
              value={student()}
              disabled={busy()}
              onChange={(event) => setStudent(event.currentTarget.value)}
            >
              <option value="">Select an active Student</option>
              <For each={roster()}>
                {(entry) => <option value={entry.rosterId}>{entry.rosterId}</option>}
              </For>
            </select>
          </label>
        </Show>
        <Show when={student() !== ""}>
          <button type="button" disabled={busy()} onClick={() => void loadConfiguration(student())}>
            Reload Student time settings
          </button>
        </Show>
        <Show when={configuration()}>
          {(current) => (
            <fieldset disabled={busy() || stale() || !props.ready}>
              <legend>Time for Student {current().rosterId}</legend>
              <label>
                Time multiplier
                <select
                  value={choice()}
                  onChange={(event) => {
                    const value = event.currentTarget.value;
                    if (
                      value === "standard" ||
                      value === "1.5" ||
                      value === "2" ||
                      value === "custom"
                    )
                      setChoice(value);
                    setMessage("");
                  }}
                >
                  <option value="standard">Standard time</option>
                  <option value="1.5">1.5X time</option>
                  <option value="2">2X time</option>
                  <option value="custom">Custom multiplier</option>
                </select>
              </label>
              <Show when={choice() === "custom"}>
                <label>
                  Custom multiplier (at least 1)
                  <input
                    type="number"
                    min="1"
                    step="any"
                    value={custom()}
                    onInput={(event) => {
                      setCustom(event.currentTarget.value);
                      setMessage("");
                    }}
                  />
                </label>
                <Show when={multiplier() === undefined}>
                  <p role="alert">Enter a finite multiplier of at least 1.</p>
                </Show>
              </Show>
              <p>Current saved base time: {durationDescription(current().baseDurationSeconds)}</p>
              <p>
                Current saved effective time:{" "}
                {durationDescription(current().effectiveDurationSeconds)}
              </p>
              <Show when={current().cappedAt24Hours}>
                <p>The saved multiplier reaches the 24-hour cap; effective time is 24 hours.</p>
              </Show>
              <Show when={multiplier() !== current().timeMultiplier}>
                <p>Save to calculate the effective time for this selection.</p>
              </Show>
              <button
                type="button"
                disabled={multiplier() === undefined}
                onClick={() => void save()}
              >
                Save Student time settings
              </button>
            </fieldset>
          )}
        </Show>
        <Show when={!props.ready}>
          <p>
            Finish saving or reload the Assessment Properties before saving Student time settings.
          </p>
        </Show>
        <Show when={busy()}>
          <p role="status">Loading or saving Student time settings...</p>
        </Show>
        <p role={stale() ? "alert" : "status"}>{message()}</p>
      </Show>
    </section>
  );
}
