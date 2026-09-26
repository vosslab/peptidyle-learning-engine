// blueprint_assessment_content_editor.tsx - task-focused editing for one Blueprint Assessment.

import { Show, createEffect, createMemo, createSignal, onCleanup, type JSX } from "solid-js";
import {
  ASSESSMENT_DURATION_OVERRIDE_MAXIMUM_MINUTES,
  assessmentDurationDefaultDescription,
  assessmentDurationOverrideMinutesDraft,
  assessmentDurationOverrideMinutesError,
  assessmentDurationOverrideSecondsFromMinutesDraft,
} from "../../assessment_duration";

import type { BlueprintAssessmentContentInput } from "../../../generated/api/BlueprintAssessmentContentInput";
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
  removeReusableEntry,
  updateReusableDefaults,
  updateReusablePoolSelectionCount,
  updateReusableText,
} from "./blueprint_course_model";
import {
  QuestionPoolPicker,
  type QuestionPoolPickerSelection,
} from "../question_pool_picker/question_pool_picker";
import { BlueprintPoolMembersEditor } from "./blueprint_pool_members_editor";
import { BlueprintAssessmentFeedbackFields } from "./blueprint_assessment_feedback_fields";
import { RecordSequence } from "../../components/record_list/record_sequence";
import type { RecordContent } from "../../components/record_list/record_list";
import { reorderedRecordListRows } from "../../components/record_list/record_list_reorder";

export interface BlueprintAssessmentContentEditorProps {
  readonly content: BlueprintAssessmentContentInput;
  readonly blueprintCourseId?: string;
  readonly retainedAssessmentId?: string;
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

function entrySummary(entry: BlueprintAssessmentContentInput["entries"][number]): string {
  if (entry.kind === "fixed") {
    const revision = entry.published_question_revision_tuple;
    return `Fixed Question ${revision.publishedQuestionId}, Revision ${revision.revisionNumber}`;
  }
  return `Question Pool ${entry.pool.question_pool_id}, Edit ${entry.pool.question_pool_edit_number}`;
}

function entryContent(
  entry: BlueprintAssessmentContentInput["entries"][number],
  remove: (() => void) | undefined,
): RecordContent {
  if (entry.kind === "fixed") {
    return {
      title: entrySummary(entry),
      details: [
        { kind: "text", label: "Points possible", value: entry.points_possible },
        { kind: "text", label: "Scoring", value: entry.scoring_rule },
      ],
      actions:
        remove === undefined
          ? []
          : [{ id: "remove-entry", kind: "command", label: "Remove", onClick: remove }],
    };
  }
  return {
    title: entrySummary(entry),
    details: [
      {
        kind: "text",
        label: "Draw each Assessment Attempt",
        value: String(entry.selection_count),
      },
      { kind: "text", label: "Points per Question", value: entry.points_per_item },
      { kind: "text", label: "Scoring", value: entry.scoring_rule },
    ],
    actions:
      remove === undefined
        ? []
        : [{ id: "remove-entry", kind: "command", label: "Remove", onClick: remove }],
  };
}

function lateWorkRuleFromValue(
  value: string,
): BlueprintAssessmentContentInput["defaults"]["late_work_rule"] | undefined {
  return value === "accept" || value === "mark_late" || value === "reject" ? value : undefined;
}

/** Form fields keep the reusable content visible and progressively explain the next useful edit. */
export function BlueprintAssessmentContentEditor(
  props: BlueprintAssessmentContentEditorProps,
): JSX.Element {
  const [editingTask, setEditingTask] = createSignal<"questions" | "properties">("questions");
  const [fixedPickerOpen, setFixedPickerOpen] = createSignal(false);
  const [poolPickerOpen, setPoolPickerOpen] = createSignal(false);
  const [memberPoolEntryId, setMemberPoolEntryId] = createSignal<string>();
  const [invalidMembers, setInvalidMembers] = createSignal(false);
  const [timeLimit, setTimeLimit] = createSignal(
    assessmentDurationOverrideMinutesDraft(
      props.content.defaults.assessment_attempt_time_limit_seconds,
    ),
  );
  const [legacyTimeLimitSeconds, setLegacyTimeLimitSeconds] = createSignal<number | null>(null);
  // Draft entries can repeat the same published identity, so list keys stay UI-local and travel with moves.
  type EntryInput = BlueprintAssessmentContentInput["entries"][number];
  type EntryRecord = { readonly entry: EntryInput; readonly index: number; readonly id: string };
  type PoolEntryRecord = EntryRecord & {
    readonly entry: Extract<EntryInput, { kind: "pool" }>;
  };
  let nextEntryIdentity = 0;
  const newEntryIdentity = (): string => `blueprint-entry-${++nextEntryIdentity}`;
  let trackedEntries = props.content.entries;
  let entryIdentities: ReadonlyArray<string> = trackedEntries.map(() => newEntryIdentity());
  const entryRecords = createMemo<ReadonlyArray<EntryRecord>>(() => {
    const entries = props.content.entries;
    if (entries !== trackedEntries) {
      trackedEntries = entries;
      entryIdentities = entries.map(() => newEntryIdentity());
    }
    return entries.map((entry, index) => ({ entry, index, id: entryIdentities[index]! }));
  });
  const questionPoolClient = props.questionPoolClient ?? createQuestionPoolLibraryClient();
  let fixedPickerTrigger: HTMLButtonElement | undefined;
  let poolPickerTrigger: HTMLButtonElement | undefined;
  let editor!: HTMLElement;
  onCleanup(() => props.onInvalidDraftChange?.(false));

  createEffect(() => {
    const seconds = props.content.defaults.assessment_attempt_time_limit_seconds;
    setTimeLimit(assessmentDurationOverrideMinutesDraft(seconds));
    setLegacyTimeLimitSeconds(seconds !== null && seconds % 60 !== 0 ? seconds : null);
  });

  function changeEntries(
    content: BlueprintAssessmentContentInput,
    message: string,
    nextIdentities: ReadonlyArray<string> = entryIdentities,
  ): void {
    trackedEntries = content.entries;
    entryIdentities = nextIdentities;
    props.onChange(content, message);
  }

  function appendEntries(content: BlueprintAssessmentContentInput, message: string): void {
    const additionalCount = content.entries.length - entryIdentities.length;
    changeEntries(content, message, [
      ...entryIdentities,
      ...Array.from({ length: additionalCount }, newEntryIdentity),
    ]);
  }

  function removeEntry(record: EntryRecord): void {
    if (memberPoolEntryId() === record.id) setMemberPoolEntryId(undefined);
    changeEntries(
      removeReusableEntry(props.content, record.index),
      "Entry removed. Add another question or save the revised content.",
      entryRecords()
        .filter((candidate) => candidate.id !== record.id)
        .map((candidate) => candidate.id),
    );
  }

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
        field === "assessment_attempt_time_limit_seconds"
          ? "Use 1 to 43200 seconds, or clear the override to use the calculated default."
          : "Use a positive whole number or clear the field for unlimited Attempts.",
      );
      return;
    }
    props.onChange(
      updateReusableDefaults(props.content, { ...props.content.defaults, [field]: parsed }),
      "Reusable defaults updated. Questions and defaults remain unsaved until you Save the Blueprint Course.",
    );
  }

  function changeDuration(input: HTMLInputElement): void {
    const minutes = input.value;
    const seconds = assessmentDurationOverrideSecondsFromMinutesDraft(minutes);
    const error = assessmentDurationOverrideMinutesError(minutes, null);
    setTimeLimit(minutes);
    setLegacyTimeLimitSeconds(null);
    input.setCustomValidity(error ?? "");
    if (seconds === undefined) {
      props.onChange(props.content, error ?? "Enter a whole number of minutes.");
      return;
    }
    props.onChange(
      updateReusableDefaults(props.content, {
        ...props.content.defaults,
        assessment_attempt_time_limit_seconds: seconds,
      }),
      "Reusable duration default updated. Questions and defaults remain unsaved until you Save the Blueprint Course.",
    );
  }

  function confirmFixedQuestions(selection: Parameters<typeof appendPickedFixedEntries>[1]): void {
    const next = appendPickedFixedEntries(props.content, selection);
    setFixedPickerOpen(false);
    appendEntries(
      next,
      `Added ${plural(selection.questionIds.length, "selected question")} as fixed entries. Continue arranging the content or save the Blueprint Course.`,
    );
  }

  function confirmQuestionPool(selection: QuestionPoolPickerSelection): void {
    setPoolPickerOpen(false);
    appendEntries(
      appendPickedPool(props.content, selection.questionPoolId, selection.questionPoolEditNumber),
      `Added Question Pool ${selection.questionPoolId}, Edit ${selection.questionPoolEditNumber}, with ${plural(selection.memberCount, "published member")}. Set its selection count or save the Blueprint Course.`,
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
          <RecordSequence
            rows={entryRecords()}
            content={(record): RecordContent =>
              entryContent(
                record.entry,
                props.editable ? (): void => removeEntry(record) : undefined,
              )
            }
            renderBody={(record) => {
              const current = record();
              const entry = current.entry;
              const index = current.index;
              return (
                <>
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
                          changeEntries(
                            updateReusablePoolSelectionCount(props.content, index, selectionCount),
                            "Question Pool selection count updated. The server validates it against current Pool membership.",
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
                        props.retainedAssessmentId &&
                        props.blueprintCourseId &&
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
                          if (entry.kind === "pool") setMemberPoolEntryId(current.id);
                        }}
                      >
                        {props.editable ? "Edit Pool members" : "View Pool members"}
                      </button>
                    </Show>
                  </Show>
                </>
              );
            }}
            recordId={(record) => record.id}
            reorder={{
              onMove: (sourceIndex, destinationIndex) => {
                const records = reorderedRecordListRows(
                  entryRecords(),
                  sourceIndex,
                  destinationIndex,
                );
                changeEntries(
                  {
                    ...props.content,
                    entries: records.map((record) => record.entry),
                  },
                  "Question order updated. Review the next entry or save the Blueprint Assessment.",
                  records.map((record) => record.id),
                );
              },
              recordLabel: (record) => entrySummary(record.entry),
              isDisabled: () => !props.editable,
            }}
            state={{ kind: "ready" }}
            ariaLabel="Ordered Blueprint Assessment entries"
            emptyState={{ title: "No Blueprint Assessment entries are selected." }}
          />
        </Show>
      </section>

      <div hidden={!memberPoolEntryId() || editingTask() !== "questions"}>
        <Show when={memberPoolEntryId()} keyed>
          {(entryId) => {
            const entry = (): PoolEntryRecord | undefined => {
              const selected = entryRecords().find((record) => record.id === entryId);
              if (selected === undefined || selected.entry.kind !== "pool") return undefined;
              return { ...selected, entry: selected.entry };
            };
            return (
              <Show when={entry()}>
                {(selected) => (
                  <Show
                    when={
                      selected().entry.kind === "pool" &&
                      selected().entry.pool.kind === "retained" &&
                      props.blueprintClient &&
                      props.blueprintCourseId &&
                      props.retainedAssessmentId
                    }
                  >
                    <BlueprintPoolMembersEditor
                      entry={selected().entry}
                      blueprintCourseId={props.blueprintCourseId!}
                      assessmentId={props.retainedAssessmentId!}
                      client={props.blueprintClient!}
                      editable={props.editable}
                      pickerRepository={props.pickerRepository}
                      pickerSources={props.pickerSources}
                      onClose={() => setMemberPoolEntryId(undefined)}
                      onInvalidDraftChange={(invalid) => {
                        setInvalidMembers(invalid);
                        props.onInvalidDraftChange?.(
                          invalid || editor.querySelector("input:invalid") !== null,
                        );
                      }}
                      onChange={(pool, message) => {
                        const records = entryRecords();
                        const nextEntries = records.map((record) =>
                          record.id === entryId &&
                          record.entry.kind === "pool" &&
                          record.entry.pool.kind === "retained"
                            ? { ...record.entry, pool }
                            : record.entry,
                        );
                        changeEntries({ ...props.content, entries: nextEntries }, message);
                      }}
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
            <input
              type="checkbox"
              checked={
                props.content.defaults.activity_rules.assessmentQuestionOrderRule === "shuffled"
              }
              onChange={(event) => {
                // ASVS 2.2.1: map the Boolean control only to the two permitted order rules.
                const assessmentQuestionOrderRule = event.currentTarget.checked
                  ? "shuffled"
                  : "authoredOrder";
                props.onChange(
                  updateReusableDefaults(props.content, {
                    ...props.content.defaults,
                    activity_rules: {
                      ...props.content.defaults.activity_rules,
                      assessmentQuestionOrderRule,
                    },
                  }),
                  "Question-order default updated. Save the Blueprint Course to keep this change.",
                );
              }}
            />
            <span>Randomize question order</span>
          </label>
          <label>
            Assessment duration override in minutes (optional, maximum 720 minutes / 12 hours)
            <input
              type="number"
              min="1"
              max={ASSESSMENT_DURATION_OVERRIDE_MAXIMUM_MINUTES}
              step="1"
              inputmode="numeric"
              value={timeLimit()}
              aria-invalid={
                assessmentDurationOverrideMinutesError(timeLimit(), legacyTimeLimitSeconds()) !==
                undefined
              }
              aria-describedby={
                assessmentDurationOverrideMinutesError(timeLimit(), legacyTimeLimitSeconds()) ===
                undefined
                  ? undefined
                  : "blueprint-assessment-duration-override-error"
              }
              onInput={(event) => changeDuration(event.currentTarget)}
            />
            <small>
              {assessmentDurationDefaultDescription(
                props.content.entries.reduce(
                  (count, entry) => count + (entry.kind === "fixed" ? 1 : entry.selection_count),
                  0,
                ),
              )}{" "}
              Leave the override blank to use this calculated default. Enter a whole number of
              minutes from 1 to {ASSESSMENT_DURATION_OVERRIDE_MAXIMUM_MINUTES} for a specific
              override.
            </small>
            <Show
              when={assessmentDurationOverrideMinutesError(timeLimit(), legacyTimeLimitSeconds())}
            >
              {(error) => (
                <small id="blueprint-assessment-duration-override-error" role="alert">
                  {error()}
                </small>
              )}
            </Show>
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
        </div>
      </fieldset>

      <div hidden={editingTask() !== "properties"}>
        <BlueprintAssessmentFeedbackFields
          content={props.content}
          editable={props.editable}
          onChange={props.onChange}
        />
      </div>

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
