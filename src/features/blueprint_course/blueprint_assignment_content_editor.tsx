// reusable_content_editor.tsx - accessible authoring surface for one Blueprint Assignment.

import { For, Show, createSignal, type JSX } from "solid-js";

import type { BlueprintAssignmentContentInput } from "../../../generated/api/BlueprintAssignmentContentInput";
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

export interface BlueprintAssignmentContentEditorProps {
  readonly content: BlueprintAssignmentContentInput;
  readonly editable: boolean;
  readonly pickerRepository: QuestionPickerSourceRepository;
  readonly pickerSources: ReadonlyArray<QuestionPickerSource>;
  readonly onChange: (content: BlueprintAssignmentContentInput, message: string) => void;
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

function entrySummary(entry: BlueprintAssignmentContentInput["entries"][number]): string {
  if (entry.kind === "fixed") return `Fixed Question ${entry.question_id}`;
  return `Question Pool: select ${entry.selection_count} from ${plural(entry.entries.length, "entry")}`;
}

function lateWorkRuleFromValue(
  value: string,
): BlueprintAssignmentContentInput["defaults"]["late_work_rule"] | undefined {
  return value === "accept" || value === "markLate" || value === "reject" ? value : undefined;
}

function assignmentAttemptGradeRuleFromValue(
  value: string,
):
  | BlueprintAssignmentContentInput["defaults"]["activity_rules"]["assignmentAttemptGradeRule"]
  | undefined {
  return value === "first" ||
    value === "latest" ||
    value === "highest" ||
    value === "instructorSelected"
    ? value
    : undefined;
}

/** Form fields keep the reusable content visible and progressively explain the next useful edit. */
export function BlueprintAssignmentContentEditor(
  props: BlueprintAssignmentContentEditorProps,
): JSX.Element {
  const [pickerIntent, setPickerIntent] = createSignal<PickerIntent>();
  let pickerTrigger: HTMLButtonElement | undefined;

  function changeText(field: "title" | "instructions", value: string): void {
    const change = field === "title" ? { title: value } : { instructions: value };
    props.onChange(
      updateReusableText(props.content, change),
      "Draft updated. Add questions or review the reusable defaults next.",
    );
  }

  function changeSchedule(field: ReusableScheduleField, value: string): void {
    const moment = readMoment(value);
    if (value.trim() !== "" && moment === null) {
      props.onChange(
        props.content,
        "Use a whole day offset, a vertical bar, and HH:MM:SS.sss, for example 7|09:00:00.000.",
      );
      return;
    }
    props.onChange(
      updateReusableSchedule(props.content, field, moment),
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
        props.content,
        "Use a positive whole number or clear the field to leave this reusable default open.",
      );
      return;
    }
    props.onChange(
      updateReusableDefaults(props.content, { ...props.content.defaults, [field]: parsed }),
      "Reusable defaults updated. Questions and schedule stay in this draft.",
    );
  }

  function moveEntry(index: number, direction: ReusableEntryDirection): void {
    props.onChange(
      moveReusableEntry(props.content, index, direction),
      "Question order updated. Review the next entry or save the Blueprint Assignment.",
    );
  }

  function confirmPicker(selection: Parameters<typeof appendPickedFixedEntries>[1]): void {
    const intent = pickerIntent();
    const next =
      intent === "pool"
        ? appendPickedPool(props.content, selection)
        : appendPickedFixedEntries(props.content, selection);
    setPickerIntent(undefined);
    const kind = intent === "pool" ? "a pool" : "fixed entries";
    props.onChange(
      next,
      `Added ${plural(selection.questionIds.length, "selected question")} as ${kind}. Set Question Pool selection count or continue arranging the content.`,
    );
  }

  function openPicker(intent: PickerIntent, trigger: HTMLButtonElement): void {
    pickerTrigger = trigger;
    setPickerIntent(intent);
  }

  return (
    <section class="blueprint-course-content-editor" aria-label="Blueprint Assignment content">
      <fieldset disabled={!props.editable}>
        <legend>Blueprint Assignment</legend>
        <div class="blueprint-course-form-grid">
          <label>
            Assignment title
            <input
              value={props.content.title}
              maxlength="200"
              onInput={(event) => changeText("title", event.currentTarget.value)}
            />
          </label>
          <label class="blueprint-course-form-wide">
            Instructions for students
            <textarea
              value={props.content.instructions}
              rows="4"
              maxlength="50000"
              onInput={(event) => changeText("instructions", event.currentTarget.value)}
            />
          </label>
        </div>
      </fieldset>

      <section
        class="blueprint-course-entry-section"
        aria-labelledby="blueprint-course-question-heading"
      >
        <div class="blueprint-course-section-heading">
          <div>
            <h3 id="blueprint-course-question-heading">Questions and pools</h3>
            <p>Fixed questions and pools stay in the order shown here.</p>
          </div>
          <Show when={props.editable}>
            <div class="blueprint-course-inline-actions">
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
          when={props.content.entries.length > 0}
          fallback={
            <p class="blueprint-course-empty-copy">
              Choose published questions to create the first reusable entry.
            </p>
          }
        >
          <ol class="blueprint-course-entry-list">
            <For each={props.content.entries}>
              {(entry, index) => (
                <li>
                  <div>
                    <strong>{entrySummary(entry)}</strong>
                    <Show when={entry.kind === "pool"}>
                      <label class="blueprint-course-small-field">
                        Draw each Assignment Attempt
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
                                props.content,
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
                      class="blueprint-course-reorder-actions"
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
                        disabled={index() === props.content.entries.length - 1}
                        onClick={() => moveEntry(index(), 1)}
                      >
                        Move later
                      </button>
                      <button
                        type="button"
                        class="danger-action"
                        onClick={() =>
                          props.onChange(
                            removeReusableEntry(props.content, index()),
                            "Entry removed. Add another question or save the revised content.",
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
        <p class="blueprint-course-field-help">
          These defaults apply when this content becomes a teaching-course assignment.
        </p>
        <div class="blueprint-course-form-grid">
          <label>
            Whole Assignment Attempt time limit (seconds)
            <input
              type="number"
              min="1"
              value={props.content.defaults.assignment_attempt_time_limit_seconds ?? ""}
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
              value={props.content.defaults.attempt_limit ?? ""}
              onInput={(event) => changeNumber("attempt_limit", event.currentTarget.value)}
            />
          </label>
          <label>
            Late work
            <select
              value={props.content.defaults.late_work_rule}
              onChange={(event) => {
                const late_work_rule = lateWorkRuleFromValue(event.currentTarget.value);
                if (late_work_rule === undefined) return;
                props.onChange(
                  updateReusableDefaults(props.content, {
                    ...props.content.defaults,
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
              value={props.content.defaults.activity_rules.assignmentAttemptGradeRule}
              onChange={(event) => {
                const rule = assignmentAttemptGradeRuleFromValue(event.currentTarget.value);
                if (rule === undefined) return;
                props.onChange(
                  updateReusableDefaults(props.content, {
                    ...props.content.defaults,
                    activity_rules: {
                      ...props.content.defaults.activity_rules,
                      assignmentAttemptGradeRule: rule,
                    },
                  }),
                  "Assignment Attempt grade-rule default updated. Continue with schedule or save.",
                );
              }}
            >
              <option value="first">First completed Assignment Attempt</option>
              <option value="latest">Latest completed Assignment Attempt</option>
              <option value="highest">Highest score</option>
              <option value="instructorSelected">Instructor-selected Assignment Attempt</option>
            </select>
          </label>
        </div>
      </fieldset>

      <fieldset disabled={!props.editable}>
        <legend>Optional relative schedule</legend>
        <p class="blueprint-course-field-help">
          Use day offset and local time, such as 7|09:00:00.000. Leave any moment blank when the
          future course should decide it.
        </p>
        <div class="blueprint-course-form-grid">
          <ScheduleField
            label="Available"
            field="available_at"
            content={props.content}
            onChange={changeSchedule}
          />
          <ScheduleField
            label="Due"
            field="due_at"
            content={props.content}
            onChange={changeSchedule}
          />
          <ScheduleField
            label="Close"
            field="closes_at"
            content={props.content}
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
  readonly content: BlueprintAssignmentContentInput;
  readonly onChange: (field: ReusableScheduleField, value: string) => void;
}

function ScheduleField(props: ScheduleFieldProps): JSX.Element {
  return (
    <label>
      {props.label} relative moment
      <input
        value={displayMoment(props.content.schedule[props.field])}
        placeholder="7|09:00:00.000"
        onInput={(event) => props.onChange(props.field, event.currentTarget.value)}
      />
    </label>
  );
}
