// reusable_definition_editor.tsx - accessible authoring surface for one reusable assignment.

import { For, Show, createSignal, type JSX } from "solid-js";

import type { ReusableAssignmentDefinitionInput } from "../../../generated/api/ReusableAssignmentDefinitionInput";
import type { RelativeScheduleMoment } from "../../../generated/api/RelativeScheduleMoment";
import {
  ProblemPicker,
  type ProblemPickerSource,
  type ProblemPickerSourceRepository,
} from "../problem_picker";
import {
  appendPickedFixedEntries,
  appendPickedPool,
  moveReusableEntry,
  removeReusableEntry,
  updateReusableDefaults,
  updateReusablePoolDrawCount,
  updateReusableSchedule,
  updateReusableText,
  type ReusableEntryDirection,
  type ReusableScheduleField,
} from "./reusable_curriculum_model";

export interface ReusableDefinitionEditorProps {
  readonly definition: ReusableAssignmentDefinitionInput;
  readonly editable: boolean;
  readonly pickerRepository: ProblemPickerSourceRepository;
  readonly pickerSources: ReadonlyArray<ProblemPickerSource>;
  readonly onChange: (definition: ReusableAssignmentDefinitionInput, message: string) => void;
}

type PickerIntent = "fixed" | "pool";

function plural(count: number, singular: string): string {
  return `${count} ${singular}${count === 1 ? "" : "s"}`;
}

function displayMoment(moment: RelativeScheduleMoment | null): string {
  return moment === null ? "" : `${moment.dayOffset}|${moment.localTime}`;
}

function readMoment(value: string): RelativeScheduleMoment | null {
  if (value.trim() === "") return null;
  const [offset, localTime, extra] = value.split("|");
  if (offset === undefined || localTime === undefined || extra !== undefined) return null;
  const dayOffset = Number(offset);
  if (!Number.isSafeInteger(dayOffset)) return null;
  return { dayOffset, localTime };
}

function entrySummary(entry: ReusableAssignmentDefinitionInput["entries"][number]): string {
  if (entry.kind === "fixed") return `Fixed question ${entry.questionId}`;
  return `Pool: draw ${entry.drawCount} from ${plural(entry.candidates.length, "candidate")}`;
}

function lateSubmissionFromValue(
  value: string,
): ReusableAssignmentDefinitionInput["defaults"]["lateSubmission"] | undefined {
  return value === "accept" || value === "markLate" || value === "reject" ? value : undefined;
}

function gradePolicyFromValue(
  value: string,
): ReusableAssignmentDefinitionInput["defaults"]["runPolicies"]["grade"] | undefined {
  return value === "first" ||
    value === "latest" ||
    value === "highest" ||
    value === "instructorSelected"
    ? value
    : undefined;
}

/** Form fields keep the reusable definition visible and progressively explain the next useful edit. */
export function ReusableDefinitionEditor(props: ReusableDefinitionEditorProps): JSX.Element {
  const [pickerIntent, setPickerIntent] = createSignal<PickerIntent>();
  let pickerTrigger: HTMLButtonElement | undefined;

  function changeText(field: "title" | "instructions", value: string): void {
    const change = field === "title" ? { title: value } : { instructions: value };
    props.onChange(
      updateReusableText(props.definition, change),
      "Draft updated. Add questions or review the reusable defaults next.",
    );
  }

  function changeSchedule(field: ReusableScheduleField, value: string): void {
    const moment = readMoment(value);
    if (value.trim() !== "" && moment === null) {
      props.onChange(
        props.definition,
        "Use a whole day offset, a vertical bar, and HH:MM:SS.sss, for example 7|09:00:00.000.",
      );
      return;
    }
    props.onChange(
      updateReusableSchedule(props.definition, field, moment),
      "Schedule draft updated. Review the calendar order before saving.",
    );
  }

  function changeNumber(field: "timeLimitSeconds" | "attemptLimit", value: string): void {
    const parsed = value.trim() === "" ? null : Number(value);
    if (parsed !== null && (!Number.isSafeInteger(parsed) || parsed < 1)) {
      props.onChange(
        props.definition,
        "Use a positive whole number or clear the field to leave this reusable default open.",
      );
      return;
    }
    props.onChange(
      updateReusableDefaults(props.definition, { ...props.definition.defaults, [field]: parsed }),
      "Reusable defaults updated. Questions and schedule stay in this draft.",
    );
  }

  function moveEntry(index: number, direction: ReusableEntryDirection): void {
    props.onChange(
      moveReusableEntry(props.definition, index, direction),
      "Question order updated. Review the next entry or save the reusable assignment.",
    );
  }

  function confirmPicker(selection: Parameters<typeof appendPickedFixedEntries>[1]): void {
    const intent = pickerIntent();
    const next =
      intent === "pool"
        ? appendPickedPool(props.definition, selection)
        : appendPickedFixedEntries(props.definition, selection);
    setPickerIntent(undefined);
    const kind = intent === "pool" ? "a pool" : "fixed entries";
    props.onChange(
      next,
      `Added ${plural(selection.questionIds.length, "selected question")} as ${kind}. Set pool draw count or continue arranging the definition.`,
    );
  }

  function openPicker(intent: PickerIntent, trigger: HTMLButtonElement): void {
    pickerTrigger = trigger;
    setPickerIntent(intent);
  }

  return (
    <section class="curriculum-definition-editor" aria-label="Reusable assignment definition">
      <fieldset disabled={!props.editable}>
        <legend>Reusable assignment</legend>
        <div class="curriculum-form-grid">
          <label>
            Assignment title
            <input
              value={props.definition.title}
              maxlength="200"
              onInput={(event) => changeText("title", event.currentTarget.value)}
            />
          </label>
          <label class="curriculum-form-wide">
            Instructions for learners
            <textarea
              value={props.definition.instructions}
              rows="4"
              maxlength="50000"
              onInput={(event) => changeText("instructions", event.currentTarget.value)}
            />
          </label>
        </div>
      </fieldset>

      <section class="curriculum-entry-section" aria-labelledby="curriculum-question-heading">
        <div class="curriculum-section-heading">
          <div>
            <h3 id="curriculum-question-heading">Questions and pools</h3>
            <p>Fixed questions and pools stay in the order shown here.</p>
          </div>
          <Show when={props.editable}>
            <div class="curriculum-inline-actions">
              <button type="button" onClick={(event) => openPicker("fixed", event.currentTarget)}>
                Add fixed questions
              </button>
              <button
                type="button"
                class="quiet-action"
                onClick={(event) => openPicker("pool", event.currentTarget)}
              >
                Add a pool
              </button>
            </div>
          </Show>
        </div>
        <Show
          when={props.definition.entries.length > 0}
          fallback={
            <p class="curriculum-empty-copy">
              Choose published questions to create the first reusable entry.
            </p>
          }
        >
          <ol class="curriculum-entry-list">
            <For each={props.definition.entries}>
              {(entry, index) => (
                <li>
                  <div>
                    <strong>{entrySummary(entry)}</strong>
                    <Show when={entry.kind === "pool"}>
                      <label class="curriculum-small-field">
                        Draw each run
                        <input
                          type="number"
                          min="1"
                          max={entry.kind === "pool" ? entry.candidates.length : 1}
                          value={entry.kind === "pool" ? entry.drawCount : 1}
                          disabled={!props.editable}
                          onInput={(event) => {
                            const drawCount = Number(event.currentTarget.value);
                            props.onChange(
                              updateReusablePoolDrawCount(props.definition, index(), drawCount),
                              "Pool draw count updated. It must not exceed the candidate count.",
                            );
                          }}
                        />
                      </label>
                    </Show>
                  </div>
                  <Show when={props.editable}>
                    <div
                      class="curriculum-reorder-actions"
                      aria-label={`Actions for entry ${index() + 1}`}
                    >
                      <button
                        type="button"
                        class="quiet-action"
                        disabled={index() === 0}
                        onClick={() => moveEntry(index(), -1)}
                      >
                        Move earlier
                      </button>
                      <button
                        type="button"
                        class="quiet-action"
                        disabled={index() === props.definition.entries.length - 1}
                        onClick={() => moveEntry(index(), 1)}
                      >
                        Move later
                      </button>
                      <button
                        type="button"
                        class="danger-action"
                        onClick={() =>
                          props.onChange(
                            removeReusableEntry(props.definition, index()),
                            "Entry removed. Add another question or save the revised definition.",
                          )
                        }
                      >
                        Remove
                      </button>
                    </div>
                  </Show>
                </li>
              )}
            </For>
          </ol>
        </Show>
      </section>

      <fieldset disabled={!props.editable}>
        <legend>Reusable defaults</legend>
        <p class="curriculum-field-help">
          These defaults apply when this definition becomes a teaching-course assignment.
        </p>
        <div class="curriculum-form-grid">
          <label>
            Whole-run time limit (seconds)
            <input
              type="number"
              min="1"
              value={props.definition.defaults.timeLimitSeconds ?? ""}
              onInput={(event) => changeNumber("timeLimitSeconds", event.currentTarget.value)}
            />
          </label>
          <label>
            Attempt limit
            <input
              type="number"
              min="1"
              value={props.definition.defaults.attemptLimit ?? ""}
              onInput={(event) => changeNumber("attemptLimit", event.currentTarget.value)}
            />
          </label>
          <label>
            Late work
            <select
              value={props.definition.defaults.lateSubmission}
              onChange={(event) => {
                const lateSubmission = lateSubmissionFromValue(event.currentTarget.value);
                if (lateSubmission === undefined) return;
                props.onChange(
                  updateReusableDefaults(props.definition, {
                    ...props.definition.defaults,
                    lateSubmission,
                  }),
                  "Late-work default updated. Continue with schedule or save.",
                );
              }}
            >
              <option value="accept">Accept</option>
              <option value="markLate">Accept and mark late</option>
              <option value="reject">Reject</option>
            </select>
          </label>
          <label>
            Grade policy
            <select
              value={props.definition.defaults.runPolicies.grade}
              onChange={(event) => {
                const grade = gradePolicyFromValue(event.currentTarget.value);
                if (grade === undefined) return;
                props.onChange(
                  updateReusableDefaults(props.definition, {
                    ...props.definition.defaults,
                    runPolicies: { ...props.definition.defaults.runPolicies, grade },
                  }),
                  "Grade-policy default updated. Continue with schedule or save.",
                );
              }}
            >
              <option value="first">First completed run</option>
              <option value="latest">Latest completed run</option>
              <option value="highest">Highest score</option>
              <option value="instructorSelected">Instructor-selected run</option>
            </select>
          </label>
        </div>
      </fieldset>

      <fieldset disabled={!props.editable}>
        <legend>Optional relative schedule</legend>
        <p class="curriculum-field-help">
          Use day offset and local time, such as 7|09:00:00.000. Leave any moment blank when the
          future course should decide it.
        </p>
        <div class="curriculum-form-grid">
          <ScheduleField
            label="Available"
            field="availableAt"
            definition={props.definition}
            onChange={changeSchedule}
          />
          <ScheduleField
            label="Due"
            field="dueAt"
            definition={props.definition}
            onChange={changeSchedule}
          />
          <ScheduleField
            label="Close"
            field="closesAt"
            definition={props.definition}
            onChange={changeSchedule}
          />
        </div>
      </fieldset>

      <Show when={pickerIntent()} keyed>
        {(intent) => (
          <ProblemPicker
            repository={props.pickerRepository}
            sources={props.pickerSources}
            mode="many"
            maximumSelection={1024}
            trigger={pickerTrigger}
            title={intent === "pool" ? "Choose pool candidates" : "Choose fixed questions"}
            confirmLabel={intent === "pool" ? "Add pool" : "Add fixed questions"}
            onConfirm={confirmPicker}
            onCancel={() => setPickerIntent(undefined)}
          />
        )}
      </Show>
    </section>
  );
}

interface ScheduleFieldProps {
  readonly label: string;
  readonly field: ReusableScheduleField;
  readonly definition: ReusableAssignmentDefinitionInput;
  readonly onChange: (field: ReusableScheduleField, value: string) => void;
}

function ScheduleField(props: ScheduleFieldProps): JSX.Element {
  return (
    <label>
      {props.label} relative moment
      <input
        value={displayMoment(props.definition.schedule[props.field])}
        placeholder="7|09:00:00.000"
        onInput={(event) => props.onChange(props.field, event.currentTarget.value)}
      />
    </label>
  );
}
