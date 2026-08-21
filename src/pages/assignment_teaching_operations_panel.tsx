import { For, Show, createEffect, createSignal, type JSX } from "solid-js";

import type { InstructorAssignmentTeachingSettingsLocal } from "../../generated/api/InstructorAssignmentTeachingSettingsLocal";
import type { InstructorAssignmentCurrentState } from "../../generated/api/InstructorAssignmentCurrentState";

export interface AssignmentTeachingOperationsPanelProps {
  readonly settings: () => InstructorAssignmentTeachingSettingsLocal | undefined;
  readonly currentState: () => InstructorAssignmentCurrentState | undefined;
  readonly busy: () => boolean;
  readonly message: () => string;
  readonly failureField: () => string | undefined;
  readonly latestSettings: () => InstructorAssignmentTeachingSettingsLocal | undefined;
  readonly onAdoptLatest: () => void;
  readonly onSave: (settings: InstructorAssignmentTeachingSettingsLocal) => Promise<void>;
}

function canonicalLocalTime(value: string): string | null {
  if (value === "") return null;
  const normalized = value.length === 16 ? `${value}:00.000` : value;
  return /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}$/u.test(normalized) ? normalized : null;
}

function controlValue(value: string | null): string {
  return value === null ? "" : value.slice(0, 16);
}

function boundedPositiveInteger(value: string): number | null {
  if (!/^[1-9][0-9]*$/u.test(value)) return null;
  const parsed = Number(value);
  return Number.isSafeInteger(parsed) && parsed <= 2_147_483_647 ? parsed : null;
}

function lifecycle(value: string): InstructorAssignmentTeachingSettingsLocal["lifecycle"] {
  if (value === "draft" || value === "published" || value === "closed" || value === "archived")
    return value;
  throw new Error("Invalid lifecycle selection");
}

function lateSubmission(
  value: string,
): InstructorAssignmentTeachingSettingsLocal["lateSubmission"] {
  if (value === "accept" || value === "markLate" || value === "reject") return value;
  throw new Error("Invalid late-work selection");
}

function lifecycleChoices(
  value: InstructorAssignmentTeachingSettingsLocal["lifecycle"],
): ReadonlyArray<InstructorAssignmentTeachingSettingsLocal["lifecycle"]> {
  if (value === "draft") return ["draft", "published", "archived"];
  if (value === "published") return ["published", "closed", "archived"];
  if (value === "closed") return ["closed", "published", "archived"];
  return ["archived"];
}

function lifecycleLabel(value: InstructorAssignmentTeachingSettingsLocal["lifecycle"]): string {
  return value === "draft"
    ? "Draft - students cannot access it"
    : value[0]!.toUpperCase() + value.slice(1);
}

function displayCourseLocalTime(value: string): string {
  return `${value.slice(0, 10)} ${value.slice(11, 16)}`;
}

export function assignmentCurrentStateCopy(
  lifecycle: InstructorAssignmentTeachingSettingsLocal["lifecycle"],
  current: InstructorAssignmentCurrentState,
  timeZone: string,
): string {
  if (current.state === "draft") return "Draft. Students cannot access this assignment.";
  if (current.state === "archived") return "Archived. Students cannot access this assignment.";
  if (current.state === "scheduled")
    return `Published, scheduled to open at ${displayCourseLocalTime(current.availableAt)} ${timeZone}.`;
  if (current.state === "open") return "Published, open now.";
  if (lifecycle === "published" && current.closedAt !== null)
    return `Published, closed since ${displayCourseLocalTime(current.closedAt)} ${timeZone}.`;
  return "Closed by instructor. Students cannot start new work.";
}

/** Separate instructor transaction so ordinary content saves cannot overwrite delivery policy. */
export function AssignmentTeachingOperationsPanel(
  props: AssignmentTeachingOperationsPanelProps,
): JSX.Element {
  const [draft, setDraft] = createSignal<InstructorAssignmentTeachingSettingsLocal>();
  const controls = new Map<string, HTMLElement>();
  const current = (): InstructorAssignmentTeachingSettingsLocal | undefined =>
    draft() ?? props.settings();

  function update(next: Partial<InstructorAssignmentTeachingSettingsLocal>): void {
    const base = current();
    if (base !== undefined) setDraft({ ...base, ...next });
  }

  async function save(): Promise<void> {
    const value = current();
    if (value !== undefined) await props.onSave(value);
  }
  createEffect(() => {
    const field = props.failureField();
    if (field !== undefined) {
      const focusField = field === "schedule" ? "dueAt" : field;
      queueMicrotask(() => controls.get(focusField)?.focus());
    }
  });
  createEffect(() => {
    // A normalized server response replaces a prior local edit.  During a CAS
    // conflict, retain the local draft until the instructor explicitly adopts.
    if (props.settings() !== undefined && props.latestSettings() === undefined) setDraft(undefined);
  });

  function invalid(field: string): boolean {
    return props.failureField() === field;
  }
  function describedBy(field: string): string | undefined {
    return invalid(field) ? "teaching-settings-error" : undefined;
  }

  return (
    <Show when={current()} keyed>
      {(settings) => (
        <section
          class="assignment-editor-policy-panel"
          role="region"
          aria-labelledby="teaching-operations-heading"
        >
          <h2 id="teaching-operations-heading">Teaching operations</h2>
          <Show when={props.currentState()} keyed>
            {(currentState) => (
              <p
                class="assignment-editor-note"
                role="status"
                data-testid="assignment-current-state"
              >
                {assignmentCurrentStateCopy(
                  props.settings()?.lifecycle ?? settings.lifecycle,
                  currentState,
                  settings.timeZone,
                )}
              </p>
            )}
          </Show>
          <p class="assignment-editor-note">
            Course time zone: {settings.timeZone}. Times use this course wall clock.
          </p>
          <fieldset class="assignment-editor-policy-set">
            <legend>Release and instructions</legend>
            <label class="assignment-editor-field">
              Lifecycle
              <select
                ref={(element) => controls.set("lifecycle", element)}
                value={settings.lifecycle}
                aria-invalid={invalid("lifecycle")}
                aria-describedby={describedBy("lifecycle")}
                disabled={props.settings()?.lifecycle === "archived"}
                onChange={(event) =>
                  update({
                    lifecycle: lifecycle(event.currentTarget.value),
                  })
                }
              >
                <For each={lifecycleChoices(props.settings()?.lifecycle ?? settings.lifecycle)}>
                  {(choice) => <option value={choice}>{lifecycleLabel(choice)}</option>}
                </For>
              </select>
            </label>
            <Show when={props.settings()?.lifecycle === "archived"}>
              <p class="assignment-editor-note">
                Archived assignments are terminal and cannot be reopened.
              </p>
            </Show>
            <label class="assignment-editor-field">
              Learner instructions
              <textarea
                ref={(element) => controls.set("instructions", element)}
                rows="4"
                value={settings.instructions}
                aria-invalid={invalid("instructions")}
                aria-describedby={describedBy("instructions")}
                onInput={(event) => update({ instructions: event.currentTarget.value })}
              />
            </label>
          </fieldset>
          <fieldset class="assignment-editor-policy-set">
            <legend>Schedule and limits</legend>
            <label class="assignment-editor-field">
              Available{" "}
              <input
                type="datetime-local"
                ref={(element) => controls.set("availableAt", element)}
                step="0.001"
                value={controlValue(settings.availableAt)}
                aria-invalid={invalid("availableAt") || invalid("schedule")}
                aria-describedby={describedBy(invalid("schedule") ? "schedule" : "availableAt")}
                onChange={(event) =>
                  update({ availableAt: canonicalLocalTime(event.currentTarget.value) })
                }
              />
            </label>
            <label class="assignment-editor-field">
              Due{" "}
              <input
                type="datetime-local"
                ref={(element) => controls.set("dueAt", element)}
                step="0.001"
                value={controlValue(settings.dueAt)}
                aria-invalid={invalid("dueAt") || invalid("schedule")}
                aria-describedby={describedBy(invalid("schedule") ? "schedule" : "dueAt")}
                onChange={(event) =>
                  update({ dueAt: canonicalLocalTime(event.currentTarget.value) })
                }
              />
            </label>
            <label class="assignment-editor-field">
              Closes{" "}
              <input
                type="datetime-local"
                ref={(element) => controls.set("closesAt", element)}
                step="0.001"
                value={controlValue(settings.closesAt)}
                aria-invalid={invalid("closesAt") || invalid("schedule")}
                aria-describedby={describedBy(invalid("schedule") ? "schedule" : "closesAt")}
                onChange={(event) =>
                  update({ closesAt: canonicalLocalTime(event.currentTarget.value) })
                }
              />
            </label>
            <label class="assignment-editor-field">
              Whole-run seconds{" "}
              <input
                type="number"
                ref={(element) => controls.set("timeLimitSeconds", element)}
                min="1"
                value={settings.timeLimitSeconds ?? ""}
                aria-invalid={invalid("timeLimitSeconds")}
                aria-describedby={describedBy("timeLimitSeconds")}
                onInput={(event) =>
                  update({
                    timeLimitSeconds:
                      event.currentTarget.value === ""
                        ? null
                        : (boundedPositiveInteger(event.currentTarget.value) ??
                          settings.timeLimitSeconds),
                  })
                }
              />
            </label>
            <label class="assignment-editor-field">
              Attempt limit{" "}
              <input
                type="number"
                ref={(element) => controls.set("attemptLimit", element)}
                min="1"
                value={settings.attemptLimit ?? ""}
                aria-invalid={invalid("attemptLimit")}
                aria-describedby={describedBy("attemptLimit")}
                onInput={(event) =>
                  update({
                    attemptLimit:
                      event.currentTarget.value === ""
                        ? null
                        : (boundedPositiveInteger(event.currentTarget.value) ??
                          settings.attemptLimit),
                  })
                }
              />
            </label>
            <label class="assignment-editor-field">
              Late work{" "}
              <select
                value={settings.lateSubmission}
                onChange={(event) =>
                  update({
                    lateSubmission: lateSubmission(event.currentTarget.value),
                  })
                }
              >
                <option value="accept">Accept</option>
                <option value="markLate">Accept and mark late</option>
                <option value="reject">Reject</option>
              </select>
            </label>
            <p class="assignment-editor-note">
              The server automatically submits work at its effective deadline.
            </p>
          </fieldset>
          <button
            class="primary-action"
            type="button"
            disabled={props.busy()}
            onClick={() => void save()}
          >
            {props.busy() ? "Saving teaching operations..." : "Save teaching operations"}
          </button>
          <Show when={props.latestSettings()}>
            <button
              type="button"
              disabled={props.busy()}
              onClick={() => {
                const latest = props.latestSettings();
                if (latest !== undefined) {
                  setDraft(latest);
                  props.onAdoptLatest();
                }
              }}
            >
              Adopt latest teaching operations
            </button>
          </Show>
          <Show when={props.message()}>
            {(message) => (
              <p
                id="teaching-settings-error"
                role={props.failureField() === undefined ? "status" : "alert"}
              >
                {message()}
              </p>
            )}
          </Show>
        </section>
      )}
    </Show>
  );
}
