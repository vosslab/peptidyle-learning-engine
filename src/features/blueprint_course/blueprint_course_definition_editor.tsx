// reusable_definition_editor.tsx - accessible authoring surface for one Blueprint Assignment.

import { For, Show, createSignal, type JSX } from "solid-js";

import type { BlueprintAssignmentDefinitionInput } from "../../../generated/api/BlueprintAssignmentDefinitionInput";
import type { RelativeAssignmentScheduleMoment } from "../../../generated/api/RelativeAssignmentScheduleMoment";
import {
  QuestionPicker,
  type QuestionPickerSource,
  type QuestionPickerSourceRepository,
} from "../question_picker";
import {
  appendPickedFixedEntries,
  appendPickedPool,
  moveReusableEntry,
  removeReusableEntry,
  updateReusableDefaults,
  updateReusablePoolSelectionCount,
  updateReusableSchedule,
  updateReusableText,
  type ReusableEntryDirection,
  type ReusableScheduleField,
} from "./blueprint_course_model";

export interface ReusableDefinitionEditorProps {
  readonly definition: BlueprintAssignmentDefinitionInput;
  readonly editable: boolean;
  readonly pickerRepository: QuestionPickerSourceRepository;
  readonly pickerSources: ReadonlyArray<QuestionPickerSource>;
  readonly onChange: (definition: BlueprintAssignmentDefinitionInput, message: string) => void;
}

type PickerIntent = "fixed" | "pool";

function plural(count: number, singular: string): string {
  return `${count} ${singular}${count === 1 ? "" : "s"}`;
}

function displayMoment(moment: RelativeAssignmentScheduleMoment | null): string {
  return moment === null ? "" : `${moment.day_offset}|${moment.local_time}`;
}

function readMoment(value: string): RelativeAssignmentScheduleMoment | null {
  if (value.trim() === "") return null;
  const [offset, localTime, extra] = value.split("|");
  if (offset === undefined || localTime === undefined || extra !== undefined) return null;
  const dayOffset = Number(offset);
  if (!Number.isSafeInteger(dayOffset)) return null;
  return { day_offset: dayOffset, local_time: localTime };
}

function entrySummary(entry: BlueprintAssignmentDefinitionInput["entries"][number]): string {
  if (entry.kind === "fixed") return `Fixed Question ${entry.question_id}`;
  return `Question Pool: select ${entry.selection_count} from ${plural(entry.entries.length, "entry")}`;
}

function lateWorkRuleFromValue(
  value: string,
): BlueprintAssignmentDefinitionInput["defaults"]["late_work_rule"] | undefined {
  return value === "accept" || value === "markLate" || value === "reject" ? value : undefined;
}

function assignmentAttemptGradeRuleFromValue(
  value: string,
):
  | BlueprintAssignmentDefinitionInput["defaults"]["activity_rules"]["assignmentAttemptGradeRule"]
  | undefined {
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

  function changeNumber(
    field: "assignment_attempt_time_limit_seconds" | "attempt_limit",
    value: string,
  ): void {
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
      "Question order updated. Review the next entry or save the Blueprint Assignment.",
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
      `Added ${plural(selection.questionIds.length, "selected question")} as ${kind}. Set Question Pool selection count or continue arranging the definition.`,
    );
  }

  function openPicker(intent: PickerIntent, trigger: HTMLButtonElement): void {
    pickerTrigger = trigger;
    setPickerIntent(intent);
  }

  return (
    <section class="curriculum-definition-editor" aria-label="Blueprint Assignment definition">
      <fieldset disabled={!props.editable}>
        <legend>Blueprint Assignment</legend>
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
            Instructions for students
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
                          max={entry.kind === "pool" ? entry.entries.length : 1}
                          value={entry.kind === "pool" ? entry.selection_count : 1}
                          disabled={!props.editable}
                          onInput={(event) => {
                            const selectionCount = Number(event.currentTarget.value);
                            props.onChange(
                              updateReusablePoolSelectionCount(
                                props.definition,
                                index(),
                                selectionCount,
                              ),
                              "Question Pool selection count updated. It must not exceed the entry count.",
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
            Whole Assignment Attempt time limit (seconds)
            <input
              type="number"
              min="1"
              value={props.definition.defaults.assignment_attempt_time_limit_seconds ?? ""}
              onInput={(event) =>
                changeNumber("assignment_attempt_time_limit_seconds", event.currentTarget.value)
              }
            />
          </label>
          <label>
            Attempt limit
            <input
              type="number"
              min="1"
              value={props.definition.defaults.attempt_limit ?? ""}
              onInput={(event) => changeNumber("attempt_limit", event.currentTarget.value)}
            />
          </label>
          <label>
            Late work
            <select
              value={props.definition.defaults.late_work_rule}
              onChange={(event) => {
                const late_work_rule = lateWorkRuleFromValue(event.currentTarget.value);
                if (late_work_rule === undefined) return;
                props.onChange(
                  updateReusableDefaults(props.definition, {
                    ...props.definition.defaults,
                    late_work_rule,
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
            Assignment Attempt grade rule
            <select
              value={props.definition.defaults.activity_rules.assignmentAttemptGradeRule}
              onChange={(event) => {
                const rule = assignmentAttemptGradeRuleFromValue(event.currentTarget.value);
                if (rule === undefined) return;
                props.onChange(
                  updateReusableDefaults(props.definition, {
                    ...props.definition.defaults,
                    activity_rules: {
                      ...props.definition.defaults.activity_rules,
                      assignmentAttemptGradeRule: rule,
                    },
                  }),
                  "Assignment Attempt grade-rule default updated. Continue with schedule or save.",
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
            field="available_at"
            definition={props.definition}
            onChange={changeSchedule}
          />
          <ScheduleField
            label="Due"
            field="due_at"
            definition={props.definition}
            onChange={changeSchedule}
          />
          <ScheduleField
            label="Close"
            field="closes_at"
            definition={props.definition}
            onChange={changeSchedule}
          />
        </div>
      </fieldset>

      <Show when={pickerIntent()} keyed>
        {(intent) => (
          <QuestionPicker
            repository={props.pickerRepository}
            sources={props.pickerSources}
            mode="many"
            maximumSelection={1024}
            trigger={pickerTrigger}
            title={intent === "pool" ? "Choose pool entries" : "Choose fixed questions"}
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
  readonly definition: BlueprintAssignmentDefinitionInput;
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
