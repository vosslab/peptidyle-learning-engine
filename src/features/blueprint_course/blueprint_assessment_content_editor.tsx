// reusable_content_editor.tsx - accessible authoring surface for one Blueprint Assessment.

import { For, Show, createSignal, type JSX } from "solid-js";

import type { BlueprintAssessmentContentInput } from "../../../generated/api/BlueprintAssessmentContentInput";
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
  updateReusableText,
  type ReusableEntryDirection,
} from "./blueprint_course_model";

export interface BlueprintAssessmentContentEditorProps {
  readonly content: BlueprintAssessmentContentInput;
  readonly editable: boolean;
  readonly pickerRepository: QuestionPickerSourceRepository;
  readonly pickerSources: ReadonlyArray<QuestionPickerSource>;
  readonly onChange: (content: BlueprintAssessmentContentInput, message: string) => void;
}

type PickerIntent = "fixed" | "pool";

function plural(count: number, singular: string): string {
  return `${count} ${singular}${count === 1 ? "" : "s"}`;
}

function entrySummary(entry: BlueprintAssessmentContentInput["entries"][number]): string {
  if (entry.kind === "fixed") return `Fixed Question ${entry.question_id}`;
  return `Question Pool: select ${entry.selection_count} from ${plural(entry.items.length, "Item")}`;
}

function lateWorkRuleFromValue(
  value: string,
): BlueprintAssessmentContentInput["defaults"]["late_work_rule"] | undefined {
  return value === "accept" || value === "mark_late" || value === "reject" ? value : undefined;
}

function assessmentAttemptGradeRuleFromValue(
  value: string,
):
  | BlueprintAssessmentContentInput["defaults"]["activity_rules"]["assessmentAttemptGradeRule"]
  | undefined {
  return value === "first" ||
    value === "latest" ||
    value === "highest" ||
    value === "instructorSelected"
    ? value
    : undefined;
}

/** Form fields keep the reusable content visible and progressively explain the next useful edit. */
export function BlueprintAssessmentContentEditor(
  props: BlueprintAssessmentContentEditorProps,
): JSX.Element {
  const [pickerIntent, setPickerIntent] = createSignal<PickerIntent>();
  let pickerTrigger: HTMLButtonElement | undefined;

  function changeText(field: "title" | "instructions", value: string): void {
    const change = field === "title" ? { title: value } : { instructions: value };
    props.onChange(
      updateReusableText(props.content, change),
      "Local working state updated. Add Questions or review the reusable defaults next.",
    );
  }

  function changeNumber(
    field: "assessment_attempt_time_limit_seconds" | "attempt_limit",
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
      "Reusable defaults updated. Questions and defaults remain unsaved until you Save the Blueprint Course.",
    );
  }

  function moveEntry(index: number, direction: ReusableEntryDirection): void {
    props.onChange(
      moveReusableEntry(props.content, index, direction),
      "Question order updated. Review the next entry or save the Blueprint Assessment.",
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
    <section class="blueprint-course-content-editor" aria-label="Blueprint Assessment content">
      <fieldset disabled={!props.editable}>
        <legend>Blueprint Assessment</legend>
        <div class="blueprint-course-form-grid">
          <label>
            Assessment title
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
                        Draw each Assessment Attempt
                        <input
                          type="number"
                          min="1"
                          max={entry.kind === "pool" ? entry.items.length : 1}
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
          These defaults apply when this content becomes a teaching-course assessment.
        </p>
        <div class="blueprint-course-form-grid">
          <label>
            Whole Assessment Attempt time limit (seconds)
            <input
              type="number"
              min="1"
              value={props.content.defaults.assessment_attempt_time_limit_seconds ?? ""}
              onInput={(event) =>
                changeNumber("assessment_attempt_time_limit_seconds", event.currentTarget.value)
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
                  "Late-work default updated. Review the defaults or save.",
                );
              }}
            >
              <option value="accept">Accept</option>
              <option value="mark_late">Accept and mark late</option>
              <option value="reject">Reject</option>
            </select>
          </label>
          <label>
            Assessment Attempt grade rule
            <select
              value={props.content.defaults.activity_rules.assessmentAttemptGradeRule}
              onChange={(event) => {
                const rule = assessmentAttemptGradeRuleFromValue(event.currentTarget.value);
                if (rule === undefined) return;
                props.onChange(
                  updateReusableDefaults(props.content, {
                    ...props.content.defaults,
                    activity_rules: {
                      ...props.content.defaults.activity_rules,
                      assessmentAttemptGradeRule: rule,
                    },
                  }),
                  "Assessment Attempt grade-rule default updated. Review the defaults or save.",
                );
              }}
            >
              <option value="first">First completed Assessment Attempt</option>
              <option value="latest">Latest completed Assessment Attempt</option>
              <option value="highest">Highest score</option>
              <option value="instructorSelected">Instructor-selected Assessment Attempt</option>
            </select>
          </label>
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
            title={intent === "pool" ? "Choose Question Pool Items" : "Choose fixed Questions"}
            confirmLabel={intent === "pool" ? "Add pool" : "Add fixed questions"}
            onConfirm={confirmPicker}
            onCancel={() => setPickerIntent(undefined)}
          />
        )}
      </Show>
    </section>
  );
}
