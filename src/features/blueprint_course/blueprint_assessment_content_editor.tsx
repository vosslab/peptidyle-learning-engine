// blueprint_assessment_content_editor.tsx - task-focused editing for one Blueprint Assessment.

import { For, Show, createEffect, createMemo, createSignal, onCleanup, type JSX } from "solid-js";
import {
  ASSESSMENT_DURATION_OVERRIDE_MAXIMUM_MINUTES,
  assessmentDurationDefaultDescription,
  assessmentDurationOverrideMinutesDraft,
  assessmentDurationOverrideMinutesError,
  assessmentDurationOverrideSecondsFromMinutesDraft,
} from "../../assessment_duration";

import type { BlueprintAssessmentContentInput } from "../../../generated/api/BlueprintAssessmentContentInput";
import { assessmentPointValueDraft } from "../../assessment_point_value";
import type { BlueprintCourseClient } from "../../api/blueprint_course";
import { createHttpApiClient } from "../../api/http_client";
import { createQuestionLibraryRepository } from "../../api/question_library_repository";
import { type QuestionPickerSource, type QuestionPickerSourceRepository } from "../question_picker";
import {
  appendPickedPool,
  removeReusableEntry,
  updateReusableDefaults,
  updateReusableEntryScoring,
  updateReusablePoolSelectionCount,
  updateReusableText,
} from "./blueprint_course_model";
import {
  AssessmentContentPicker,
  type AssessmentContentPickerSelection,
} from "../assessment_content_picker";
import { BlueprintAssessmentFeedbackFields } from "./blueprint_assessment_feedback_fields";
import { RecordSequence } from "../../components/record_list/record_sequence";
import type { RecordContent } from "../../components/record_list/record_list";
import { reorderedRecordListRows } from "../../components/record_list/record_list_reorder";
import type { RecognitionTitleMaps } from "../../api/recognition_titles";
import { recognitionTitlesResource } from "../recognition_titles_load";
import {
  blueprintAssessmentEntryContent,
  blueprintAssessmentEntryLabel,
} from "./blueprint_assessment_entry_content";

export interface BlueprintAssessmentContentEditorProps {
  readonly content: BlueprintAssessmentContentInput;
  readonly blueprintClient?: BlueprintCourseClient;
  readonly editable: boolean;
  readonly pickerRepository: QuestionPickerSourceRepository;
  readonly pickerSources: ReadonlyArray<QuestionPickerSource>;
  readonly onChange: (content: BlueprintAssessmentContentInput, message: string) => void;
  readonly onInvalidDraftChange?: (invalid: boolean) => void;
}

function plural(count: number, singular: string): string {
  return `${count} ${singular}${count === 1 ? "" : "s"}`;
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
  const [contentPickerOpen, setContentPickerOpen] = createSignal(false);
  const [pointDrafts, setPointDrafts] = createSignal<Readonly<Record<string, string>>>({});
  const [timeLimit, setTimeLimit] = createSignal(
    assessmentDurationOverrideMinutesDraft(
      props.content.defaults.assessment_attempt_time_limit_seconds,
    ),
  );
  // Draft entries can repeat the same published identity, so list keys stay UI-local and travel with moves.
  type EntryInput = BlueprintAssessmentContentInput["entries"][number];
  type EntryRecord = { readonly entry: EntryInput; readonly index: number; readonly id: string };
  let nextEntryIdentity = 0;
  const newEntryIdentity = (): string => `blueprint-entry-${++nextEntryIdentity}`;
  let trackedEntries = props.content.entries;
  let entryIdentities: ReadonlyArray<string> = trackedEntries.map(() => newEntryIdentity());
  const entryRecords = createMemo<ReadonlyArray<EntryRecord>>(() => {
    const entries = props.content.entries;
    if (entries !== trackedEntries) {
      entryIdentities = entries.map(() => newEntryIdentity());
      trackedEntries = entries;
    }
    return entries.map((entry, index) => ({ entry, index, id: entryIdentities[index]! }));
  });
  const assessmentContentRepository = createQuestionLibraryRepository(createHttpApiClient());
  let contentPickerTrigger: HTMLButtonElement | undefined;
  let editor: HTMLElement | undefined;
  onCleanup(() => props.onInvalidDraftChange?.(false));

  function notifyInvalidDraft(): void {
    if (editor === undefined) return;
    props.onInvalidDraftChange?.(
      Object.keys(pointDrafts()).length > 0 ||
        editor.querySelector("input:invalid:not([data-point-entry-id])") !== null,
    );
  }

  createEffect(() => {
    const liveIds = new Set(entryRecords().map((record) => record.id));
    setPointDrafts((current) => {
      const next = Object.fromEntries(
        Object.entries(current).filter(([identity]) => liveIds.has(identity)),
      );
      return Object.keys(next).length === Object.keys(current).length ? current : next;
    });
    notifyInvalidDraft();
  });

  createEffect(() => {
    const seconds = props.content.defaults.assessment_attempt_time_limit_seconds;
    setTimeLimit(assessmentDurationOverrideMinutesDraft(seconds));
  });

  function changeEntries(
    content: BlueprintAssessmentContentInput,
    message: string,
    nextIdentities: ReadonlyArray<string> = entryIdentities,
  ): void {
    trackedEntries = content.entries;
    entryIdentities = nextIdentities;
    const liveIds = new Set(nextIdentities);
    setPointDrafts((current) => {
      const next = Object.fromEntries(
        Object.entries(current).filter(([identity]) => liveIds.has(identity)),
      );
      return Object.keys(next).length === Object.keys(current).length ? current : next;
    });
    props.onChange(content, message);
    notifyInvalidDraft();
    queueMicrotask(notifyInvalidDraft);
  }

  function appendEntries(content: BlueprintAssessmentContentInput, message: string): void {
    const additionalCount = content.entries.length - entryIdentities.length;
    changeEntries(content, message, [
      ...entryIdentities,
      ...Array.from({ length: additionalCount }, newEntryIdentity),
    ]);
  }

  function changeScoring(
    record: EntryRecord,
    change: { readonly points?: string; readonly scoringRule?: string },
  ): void {
    const updated = updateReusableEntryScoring(props.content, record.index, change);
    if (updated === props.content) return;
    changeEntries(updated, "Entry scoring updated. Save the Blueprint Course to keep this change.");
  }

  function changePoints(record: EntryRecord, input: HTMLInputElement): void {
    const value = input.value;
    const parsed = assessmentPointValueDraft(value);
    if (parsed === undefined) {
      setPointDrafts((current) => ({ ...current, [record.id]: value }));
      input.setCustomValidity(
        "Enter a point value from 0 to 1,000,000,000.9999 with at most four decimal places.",
      );
      return;
    }
    input.setCustomValidity("");
    setPointDrafts((current) => {
      const next = { ...current };
      delete next[record.id];
      return next;
    });
    changeScoring(record, { points: value });
  }

  function removeEntry(record: EntryRecord): void {
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
    const error = assessmentDurationOverrideMinutesError(minutes);
    setTimeLimit(minutes);
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

  const recognitionRequest = createMemo(() => {
    const questionIds = new Set<string>();
    const poolIds = new Set<string>();
    for (const entry of props.content.entries) {
      if (entry.kind === "fixed") {
        questionIds.add(entry.published_question_revision_tuple.publishedQuestionId);
      } else {
        poolIds.add(entry.question_pool_id);
      }
    }
    return { questionIds: [...questionIds], poolIds: [...poolIds] };
  });
  const recognition = recognitionTitlesResource(() => props.blueprintClient, recognitionRequest);

  function rememberTitles(incoming: RecognitionTitleMaps): void {
    recognition.remember(incoming);
  }

  function confirmFixedQuestions(
    selection: Extract<AssessmentContentPickerSelection, { kind: "questions" }>,
  ): void {
    rememberTitles({
      questions: new Map(
        selection.questions.map((question) => [
          question.publishedQuestionRevisionTuple.publishedQuestionId,
          question.title,
        ]),
      ),
      pools: new Map(),
    });
    appendEntries(
      {
        ...props.content,
        entries: [
          ...props.content.entries,
          ...selection.questions.map((question) => ({
            kind: "fixed" as const,
            published_question_revision_tuple: question.publishedQuestionRevisionTuple,
            points_possible: "1",
            scoring_rule: "normal" as const,
            question_attempt_limit: { maxAttempts: null },
            question_attempt_time_limit: { kind: "unlimited" as const },
          })),
        ],
      },
      `Added ${plural(selection.questions.length, "selected question")} as fixed entries. Continue arranging the content or save the Blueprint Course.`,
    );
  }

  function confirmQuestionPool(
    selection: Extract<AssessmentContentPickerSelection, { kind: "pool" }>,
  ): void {
    rememberTitles({
      questions: new Map(),
      pools: new Map([[selection.questionPoolId, selection.title]]),
    });
    appendEntries(
      appendPickedPool(props.content, selection.questionPoolId),
      `Added ${selection.title} (${selection.questionPoolId}, Edit ${selection.questionPoolEditNumber}), with ${plural(selection.memberCount, "published member")}. Set its selection count or save the Blueprint Course.`,
    );
  }

  function confirmAssessmentContent(selection: AssessmentContentPickerSelection): void {
    setContentPickerOpen(false);
    if (selection.kind === "pool") {
      confirmQuestionPool(selection);
      return;
    }
    confirmFixedQuestions(selection);
  }

  return (
    <section
      ref={(element) => {
        editor = element;
      }}
      class="blueprint-course-content-editor"
      aria-label="Blueprint Assessment content"
      onInput={notifyInvalidDraft}
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
            <Show when={recognition.status()}>
              <p role="status">{recognition.status()}</p>
            </Show>
          </div>
          <Show when={props.editable}>
            <div class="blueprint-course-inline-actions">
              <button
                type="button"
                onClick={(event) => {
                  contentPickerTrigger = event.currentTarget;
                  setContentPickerOpen(true);
                }}
              >
                Add Assessment content
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
              blueprintAssessmentEntryContent(
                record.entry,
                recognition.titles(),
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
                  <Show when={entry.kind === "pool" ? entry : undefined}>
                    {(poolEntry) => (
                      <a class="quiet-action" href={`/library/${poolEntry().question_pool_id}`}>
                        Open ordinary Pool
                      </a>
                    )}
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
              recordLabel: (record) =>
                blueprintAssessmentEntryLabel(record.entry, recognition.titles()),
              isDisabled: () => !props.editable,
            }}
            state={{ kind: "ready" }}
            ariaLabel="Ordered Blueprint Assessment entries"
            emptyState={{ title: "No Blueprint Assessment entries are selected." }}
          />
        </Show>
      </section>

      <fieldset disabled={!props.editable} hidden={editingTask() !== "properties"}>
        <legend>Reusable defaults</legend>
        <p class="blueprint-course-field-help">
          These defaults apply when this content becomes a teaching-course assessment.
        </p>
        <div class="blueprint-course-form-grid">
          <label class="blueprint-course-form-checkbox">
            <input
              type="checkbox"
              checked={props.content.defaults.activity_rules.partialCreditEnabled}
              onChange={(event) => {
                props.onChange(
                  updateReusableDefaults(props.content, {
                    ...props.content.defaults,
                    activity_rules: {
                      ...props.content.defaults.activity_rules,
                      partialCreditEnabled: event.currentTarget.checked,
                    },
                  }),
                  "Partial-credit default updated. Save the Blueprint Course to keep this change.",
                );
              }}
            />
            <span>Award partial credit</span>
            <small>
              When disabled, fractional Question credit earns zero points. Full Credit entries still
              award their points for any submitted response.
            </small>
          </label>
          <label class="blueprint-course-form-checkbox">
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
              aria-invalid={assessmentDurationOverrideMinutesError(timeLimit()) !== undefined}
              aria-describedby={
                assessmentDurationOverrideMinutesError(timeLimit()) === undefined
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
            <Show when={assessmentDurationOverrideMinutesError(timeLimit())}>
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

      <fieldset disabled={!props.editable} hidden={editingTask() !== "properties"}>
        <legend>Scoring</legend>
        <Show
          when={entryRecords().length > 0}
          fallback={
            <p class="blueprint-course-field-help">
              Add a Question before setting its points and scoring.
            </p>
          }
        >
          <For each={entryRecords()}>
            {(record) => (
              <fieldset>
                <legend>{blueprintAssessmentEntryLabel(record.entry, recognition.titles())}</legend>
                <label>
                  {record.entry.kind === "fixed" ? "Points possible" : "Points per Question"}
                  <input
                    type="text"
                    data-point-entry-id={record.id}
                    inputMode="decimal"
                    autocomplete="off"
                    value={
                      pointDrafts()[record.id] ??
                      (record.entry.kind === "fixed"
                        ? record.entry.points_possible
                        : record.entry.points_per_item)
                    }
                    aria-invalid={pointDrafts()[record.id] !== undefined}
                    onInput={(event) => changePoints(record, event.currentTarget)}
                  />
                  <Show when={pointDrafts()[record.id] !== undefined}>
                    <small role="alert">
                      Enter a point value from 0 to 1,000,000,000.9999 with at most four decimal
                      places.
                    </small>
                  </Show>
                </label>
                <label>
                  Scoring
                  <select
                    value={record.entry.scoring_rule}
                    onChange={(event) =>
                      changeScoring(record, { scoringRule: event.currentTarget.value })
                    }
                  >
                    <option value="normal">Normal</option>
                    <option value="fullCredit">Full credit</option>
                    <option value="extraCredit">Extra credit</option>
                    <option value="excluded">Excluded</option>
                  </select>
                </label>
              </fieldset>
            )}
          </For>
        </Show>
      </fieldset>

      <div hidden={editingTask() !== "properties"}>
        <BlueprintAssessmentFeedbackFields
          content={props.content}
          editable={props.editable}
          onChange={props.onChange}
        />
      </div>

      <Show when={contentPickerOpen()}>
        <AssessmentContentPicker
          repository={assessmentContentRepository}
          trigger={contentPickerTrigger}
          maximumQuestionSelection={1000}
          onConfirm={confirmAssessmentContent}
          onCancel={() => setContentPickerOpen(false)}
        />
      </Show>
    </section>
  );
}
