// blueprint_assessment_content_editor.tsx - task-focused editing for one Blueprint Assessment.

import { For, Show, createSignal, onCleanup, type JSX } from "solid-js";

import type { BlueprintAssessmentContentInput } from "../../../generated/api/BlueprintAssessmentContentInput";
import type { BlueprintAssessmentContentView } from "../../../generated/api/BlueprintAssessmentContentView";
import type { QuestionPoolLibraryClient } from "../../api/question_pool_library";
import type { BlueprintCourseClient } from "../../api/blueprint_course";
import { createQuestionPoolLibraryClient } from "../../api/http_client/question_pool_library";
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
import { QuestionPoolPicker, type QuestionPoolPickerSelection } from "./question_pool_picker";
import { BlueprintPoolMembersEditor } from "./blueprint_pool_members_editor";

export interface BlueprintAssessmentContentEditorProps {
  readonly content: BlueprintAssessmentContentInput;
  /** Saved server view used only to present exact immutable Pool Revision pins. */
  readonly savedContent?: BlueprintAssessmentContentView;
  readonly blueprintRef?: string;
  readonly retainedAssessmentRef?: string;
  readonly blueprintClient?: BlueprintCourseClient;
  readonly editable: boolean;
  readonly pickerRepository: QuestionPickerSourceRepository;
  readonly pickerSources: ReadonlyArray<QuestionPickerSource>;
  readonly questionPoolClient?: QuestionPoolLibraryClient;
  readonly onChange: (content: BlueprintAssessmentContentInput, message: string) => void;
  readonly onInvalidDraftChange?: (invalid: boolean) => void;
}

function plural(count: number, singular: string): string {
  return `${count} ${singular}${count === 1 ? "" : "s"}`;
}

function entrySummary(
  entry: BlueprintAssessmentContentInput["entries"][number],
  _savedEntry: BlueprintAssessmentContentView["entries"][number] | undefined,
): string {
  if (entry.kind === "fixed") {
    const revision = entry.published_question;
    return `Fixed Question ${revision.questionId}, Revision ${revision.revisionNumber}`;
  }
  const revision = entry.pool.questionPoolRevision;
  return `Question Pool ${revision.questionPoolId}, Revision ${revision.revisionNumber}: select ${entry.selection_count}`;
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
  const [editingTask, setEditingTask] = createSignal<"questions" | "properties">("questions");
  const [fixedPickerOpen, setFixedPickerOpen] = createSignal(false);
  const [poolPickerOpen, setPoolPickerOpen] = createSignal(false);
  const [memberPoolId, setMemberPoolId] = createSignal<string>();
  const [invalidMembers, setInvalidMembers] = createSignal(false);
  const questionPoolClient = props.questionPoolClient ?? createQuestionPoolLibraryClient();
  let fixedPickerTrigger: HTMLButtonElement | undefined;
  let poolPickerTrigger: HTMLButtonElement | undefined;
  let editor!: HTMLElement;
  onCleanup(() => props.onInvalidDraftChange?.(false));

  function validNumber(input: HTMLInputElement): boolean {
    input.setCustomValidity("");
    if (input.value !== "" && !Number.isSafeInteger(Number(input.value))) {
      input.setCustomValidity("Use a positive whole number.");
    }
    return input.validity.valid;
  }

  function changeText(field: "title" | "instructions", value: string): void {
    const change = field === "title" ? { title: value } : { instructions: value };
    props.onChange(
      updateReusableText(props.content, change),
      "Local working state updated. Add Questions or review the reusable defaults next.",
    );
  }

  function changeNumber(
    field: "assessment_attempt_time_limit_seconds" | "assessment_attempt_limit",
    input: HTMLInputElement,
  ): void {
    const value = input.value;
    const parsed = value.trim() === "" ? null : Number(value);
    if (!validNumber(input)) {
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

  function confirmFixedQuestions(selection: Parameters<typeof appendPickedFixedEntries>[1]): void {
    const next = appendPickedFixedEntries(props.content, selection);
    setFixedPickerOpen(false);
    props.onChange(
      next,
      `Added ${plural(selection.questionIds.length, "selected question")} as fixed entries. Continue arranging the content or save the Blueprint Course.`,
    );
  }

  function confirmQuestionPool(selection: QuestionPoolPickerSelection): void {
    setPoolPickerOpen(false);
    props.onChange(
      appendPickedPool(props.content, selection.questionPoolRevision),
      `Added Question Pool ${selection.questionPoolRevision.questionPoolId}, Revision ${selection.questionPoolRevision.revisionNumber}, with ${plural(selection.memberCount, "published member")}. Set its selection count or save the Blueprint Course.`,
    );
  }

  return (
    <section
      ref={(element) => {
        editor = element;
      }}
      class="blueprint-course-content-editor"
      aria-label="Blueprint Assessment content"
      onInput={() =>
        props.onInvalidDraftChange?.(
          invalidMembers() || editor.querySelector("input:invalid") !== null,
        )
      }
    >
      <div
        class="blueprint-course-inline-actions"
        role="group"
        aria-label="Assessment editing task"
      >
        <button
          type="button"
          classList={{ "quiet-action": editingTask() !== "questions" }}
          aria-pressed={editingTask() === "questions"}
          onClick={() => setEditingTask("questions")}
        >
          Questions
        </button>
        <button
          type="button"
          classList={{ "quiet-action": editingTask() !== "properties" }}
          aria-pressed={editingTask() === "properties"}
          onClick={() => setEditingTask("properties")}
        >
          Properties
        </button>
      </div>
      <p class="blueprint-course-field-help">
        Questions and Properties share your local working state. Save the Blueprint Course to keep
        changes from either task.
      </p>
      {/* Keep fields mounted so switching tasks preserves invalid input and Pool-member drafts. */}
      <fieldset disabled={!props.editable} hidden={editingTask() !== "properties"}>
        <legend>Blueprint Assessment Properties</legend>
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
        hidden={editingTask() !== "questions"}
        aria-labelledby="blueprint-course-question-heading"
      >
        <div class="blueprint-course-section-heading">
          <div>
            <h3 id="blueprint-course-question-heading">Blueprint Assessment Questions</h3>
            <p>Fixed questions and pools stay in the order shown here.</p>
          </div>
          <Show when={props.editable}>
            <div class="blueprint-course-inline-actions">
              <button
                type="button"
                onClick={(event) => {
                  fixedPickerTrigger = event.currentTarget;
                  setFixedPickerOpen(true);
                }}
              >
                Add fixed questions
              </button>
              <button
                type="button"
                class="quiet-action"
                onClick={(event) => {
                  poolPickerTrigger = event.currentTarget;
                  setPoolPickerOpen(true);
                }}
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
                    <strong>{entrySummary(entry, props.savedContent?.entries[index()])}</strong>
                    <Show when={entry.kind === "pool"}>
                      <label class="blueprint-course-small-field">
                        Draw each Assessment Attempt
                        <input
                          type="number"
                          min="1"
                          required
                          value={entry.kind === "pool" ? entry.selection_count : 1}
                          disabled={!props.editable}
                          onInput={(event) => {
                            if (!validNumber(event.currentTarget)) {
                              props.onChange(
                                props.content,
                                "Use a positive whole number for the Pool selection count.",
                              );
                              return;
                            }
                            const selectionCount = Number(event.currentTarget.value);
                            props.onChange(
                              updateReusablePoolSelectionCount(
                                props.content,
                                index(),
                                selectionCount,
                              ),
                              "Question Pool selection count updated. The server validates it against the selected Pool Revision.",
                            );
                          }}
                        />
                      </label>
                    </Show>
                    <Show when={entry.kind === "pool"}>
                      <Show
                        when={
                          entry.kind === "pool" &&
                          entry.pool.kind === "retained" &&
                          props.retainedAssessmentRef &&
                          props.blueprintRef &&
                          props.blueprintClient
                        }
                        fallback={
                          <p class="blueprint-course-field-help">
                            Save the Blueprint Course before editing this Assessment-owned Pool's
                            members.
                          </p>
                        }
                      >
                        <button
                          type="button"
                          class="quiet-action"
                          onClick={() => {
                            if (entry.kind === "pool")
                              setMemberPoolId(entry.pool.questionPoolRevision.questionPoolId);
                          }}
                        >
                          {props.editable ? "Edit Pool members" : "View Pool members"}
                        </button>
                      </Show>
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

      <div hidden={!memberPoolId() || editingTask() !== "questions"}>
        <Show when={memberPoolId()} keyed>
          {(poolId) => {
            const entry = (): BlueprintAssessmentContentInput["entries"][number] | undefined =>
              props.content.entries.find(
                (candidate) =>
                  candidate.kind === "pool" &&
                  candidate.pool.kind === "retained" &&
                  candidate.pool.questionPoolRevision.questionPoolId === poolId,
              );
            return (
              <Show when={entry()}>
                {(selected) => (
                  <Show
                    when={
                      selected().kind === "pool" &&
                      props.blueprintClient &&
                      props.blueprintRef &&
                      props.retainedAssessmentRef
                    }
                  >
                    <BlueprintPoolMembersEditor
                      entry={
                        selected() as Extract<
                          BlueprintAssessmentContentInput["entries"][number],
                          { kind: "pool" }
                        >
                      }
                      blueprintRef={props.blueprintRef!}
                      assessmentRef={props.retainedAssessmentRef!}
                      client={props.blueprintClient!}
                      editable={props.editable}
                      pickerRepository={props.pickerRepository}
                      pickerSources={props.pickerSources}
                      onClose={() => setMemberPoolId(undefined)}
                      onInvalidDraftChange={(invalid) => {
                        setInvalidMembers(invalid);
                        props.onInvalidDraftChange?.(
                          invalid || editor.querySelector("input:invalid") !== null,
                        );
                      }}
                      onChange={(pool, message) =>
                        props.onChange(
                          {
                            ...props.content,
                            entries: props.content.entries.map((candidate) =>
                              candidate.kind === "pool" &&
                              candidate.pool.kind === "retained" &&
                              candidate.pool.questionPoolRevision.questionPoolId === poolId
                                ? { ...candidate, pool }
                                : candidate,
                            ),
                          },
                          message,
                        )
                      }
                    />
                  </Show>
                )}
              </Show>
            );
          }}
        </Show>
      </div>

      <fieldset disabled={!props.editable} hidden={editingTask() !== "properties"}>
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
                changeNumber("assessment_attempt_time_limit_seconds", event.currentTarget)
              }
            />
          </label>
          <label>
            Assessment Attempt limit
            <input
              type="number"
              min="1"
              value={props.content.defaults.assessment_attempt_limit ?? ""}
              disabled={
                props.content.assessment_type === "quiz" || props.content.assessment_type === "exam"
              }
              onInput={(event) => changeNumber("assessment_attempt_limit", event.currentTarget)}
            />
            <Show
              when={
                props.content.assessment_type === "quiz" || props.content.assessment_type === "exam"
              }
            >
              <small>Quiz and Exam permit exactly one Assessment Attempt.</small>
            </Show>
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

      <Show when={fixedPickerOpen()}>
        <QuestionPicker
          repository={props.pickerRepository}
          sources={props.pickerSources}
          mode="many"
          maximumSelection={1024}
          trigger={fixedPickerTrigger}
          title="Choose fixed Questions"
          confirmLabel="Add fixed questions"
          onConfirm={confirmFixedQuestions}
          onCancel={() => setFixedPickerOpen(false)}
        />
      </Show>
      <Show when={poolPickerOpen()}>
        <QuestionPoolPicker
          client={questionPoolClient}
          trigger={poolPickerTrigger}
          onConfirm={confirmQuestionPool}
          onCancel={() => setPoolPickerOpen(false)}
        />
      </Show>
    </section>
  );
}
