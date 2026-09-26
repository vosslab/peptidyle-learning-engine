// Exact-member editor for one Assessment-owned Question Pool fork.

import { Show, createSignal, type JSX } from "solid-js";

import { MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY } from "../../../generated/api/MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY";
import type { AssessmentEntry } from "../../../generated/api/AssessmentEntry";
import type { AssessmentQuestionPoolForkView } from "../../../generated/api/AssessmentQuestionPoolForkView";
import type { PublishedQuestionRevisionTuple } from "../../../generated/api/PublishedQuestionRevisionTuple";
import {
  QuestionPicker,
  type QuestionPickerSelection,
  type QuestionPickerSource,
  type QuestionPickerSourceRepository,
} from "../../features/question_picker";
import { questionRevisionKey } from "./assessment_workspace_questions_model";
import { CourseClassificationSummary } from "../../components/course_classification_summary";
import type { RecordContent } from "../../components/record_list/record_list";
import { reorderedRecordListRows } from "../../components/record_list/record_list_reorder";
import { RecordSequence } from "../../components/record_list/record_sequence";

export interface AssessmentPoolEntryEditorProps {
  readonly entry: Extract<AssessmentEntry, { readonly kind: "questionPool" }>;
  readonly fork: AssessmentQuestionPoolForkView | undefined;
  readonly exactMembersUnavailable: boolean;
  readonly pickerRepository: QuestionPickerSourceRepository;
  readonly pickerSources: ReadonlyArray<QuestionPickerSource>;
  readonly mutationsEnabled: boolean;
  readonly busy: boolean;
  readonly onSelectionCount: (selectionCount: number) => void;
  readonly onReplaceMembers: (
    members: ReadonlyArray<PublishedQuestionRevisionTuple>,
  ) => Promise<void>;
}

/** Renders exact immutable fork members and only the two permitted Assessment-owned mutations. */
export function AssessmentPoolEntryEditor(props: AssessmentPoolEntryEditorProps): JSX.Element {
  const [attested, setAttested] = createSignal(false);
  const [pickerOpen, setPickerOpen] = createSignal(false);
  let pickerTrigger: HTMLButtonElement | undefined;

  function submitSelectionCount(value: string): void {
    const selectionCount = Number(value);
    if (
      !Number.isSafeInteger(selectionCount) ||
      selectionCount < 1 ||
      props.fork === undefined ||
      selectionCount > props.fork.members.length
    ) {
      return;
    }
    if (selectionCount !== props.entry.selectionCount) props.onSelectionCount(selectionCount);
  }

  function append(selection: QuestionPickerSelection): void {
    setPickerOpen(false);
    const candidate = selection.questions[0];
    if (candidate === undefined || !attested() || !props.mutationsEnabled || props.busy) return;
    const publishedQuestionRevisionTuple = candidate.row.publishedQuestionRevisionTuple;
    const members =
      props.fork?.members.map((member) => member.publishedQuestionRevisionTuple) ?? [];
    if (
      members.some(
        (member) =>
          questionRevisionKey(member) === questionRevisionKey(publishedQuestionRevisionTuple),
      )
    ) {
      return;
    }
    void props.onReplaceMembers([...members, publishedQuestionRevisionTuple]);
    setAttested(false);
  }

  function replaceMembers(members: ReadonlyArray<PublishedQuestionRevisionTuple>): Promise<void> {
    if (!attested() || !props.mutationsEnabled || props.busy) return Promise.resolve();
    const replacement = props.onReplaceMembers(members);
    setAttested(false);
    return replacement;
  }

  return (
    <section class="assessment-pool-fork" aria-labelledby={`assessment-pool-${props.entry.id}`}>
      <h3 id={`assessment-pool-${props.entry.id}`}>
        Question Pool {props.entry.questionPoolId} Edit {props.entry.questionPoolEditNumber}
      </h3>
      <Show
        when={props.fork}
        fallback={
          <p class="assessment-editor-note">
            {props.exactMembersUnavailable
              ? "This Assessment's exact pinned Pool members could not load. Reload the Assessment and try again."
              : "Loading this Assessment's exact pinned Pool members..."}
          </p>
        }
      >
        {(fork) => (
          <>
            <h4>{fork().metadata.title}</h4>
            <p>{fork().metadata.description}</p>
            <CourseClassificationSummary value={fork().metadata} />
            <Show when={fork().bloom}>
              {(bloom) => (
                <p>
                  Bloom Cognitive Process: {bloom().cognitiveProcess}; Bloom Knowledge Dimension:{" "}
                  {bloom().knowledgeDimension}
                </p>
              )}
            </Show>
            <p>
              This Assessment owns this imported Pool fork. It selects {props.entry.selectionCount}{" "}
              of {fork().members.length} exact pinned Questions.
            </p>
            <label>
              <input
                type="checkbox"
                disabled={!props.mutationsEnabled || props.busy}
                checked={attested()}
                onChange={(event) => setAttested(event.currentTarget.checked)}
              />
              These Questions are interchangeable assessments of the intended learning.
            </label>
            <RecordSequence
              rows={fork().members.map((member, index) => ({ member, index }))}
              content={(record): RecordContent => ({
                // ASVS 1.2.1: ordinary JSX rendering keeps IDs inert; no HTML or raw JSON rendering.
                title: `Question ${record.member.publishedQuestionRevisionTuple.publishedQuestionId}, Revision ${record.member.publishedQuestionRevisionTuple.revisionNumber}`,
                description: record.member.question.question_library.summary.metadata.questionTitle,
                details: [],
                actions: [
                  {
                    id: "remove-member",
                    kind: "command",
                    label: "Remove",
                    disabled:
                      !props.mutationsEnabled ||
                      props.busy ||
                      !attested() ||
                      fork().members.length <= props.entry.selectionCount,
                    onClick: (): void =>
                      void replaceMembers(
                        fork()
                          .members.filter((_member, index) => index !== record.index)
                          .map((member) => member.publishedQuestionRevisionTuple),
                      ),
                  },
                ],
              })}
              recordId={(record) =>
                questionRevisionKey(record.member.publishedQuestionRevisionTuple)
              }
              state={{ kind: "ready" }}
              ariaLabel="Ordered Assessment Pool members"
              emptyState={{ title: "No Pool members are available." }}
              reorder={{
                onMove: (sourceIndex, destinationIndex): Promise<void> =>
                  replaceMembers(
                    reorderedRecordListRows(fork().members, sourceIndex, destinationIndex).map(
                      (member) => member.publishedQuestionRevisionTuple,
                    ),
                  ),
                recordLabel: (record): string =>
                  `Question ${record.member.publishedQuestionRevisionTuple.publishedQuestionId}, Revision ${record.member.publishedQuestionRevisionTuple.revisionNumber}`,
                isDisabled: (): boolean => !props.mutationsEnabled || props.busy || !attested(),
              }}
            />
            <Show when={fork().members.length <= props.entry.selectionCount}>
              <p class="assessment-editor-note">
                Lower the selection count before removing a Pool member.
              </p>
            </Show>
            <fieldset disabled={!props.mutationsEnabled || props.busy}>
              <legend>Questions selected for each Attempt</legend>
              <label class="assessment-editor-field">
                Selection count
                <input
                  type="number"
                  min="1"
                  max={fork().members.length}
                  value={props.entry.selectionCount}
                  onChange={(event) => submitSelectionCount(event.currentTarget.value)}
                />
              </label>
              <p class="assessment-editor-note">
                This changes only this Assessment-owned Pool entry.
              </p>
            </fieldset>
            <fieldset
              disabled={
                !props.mutationsEnabled ||
                props.busy ||
                fork().members.length >= MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY
              }
            >
              <legend>Add one pinned Question</legend>
              <button
                type="button"
                ref={(element) => (pickerTrigger = element)}
                disabled={!attested()}
                onClick={() => setPickerOpen(true)}
              >
                Choose a published Question
              </button>
              <Show when={pickerOpen()}>
                <QuestionPicker
                  repository={props.pickerRepository}
                  sources={props.pickerSources}
                  mode="one"
                  maximumSelection={1}
                  trigger={pickerTrigger}
                  title="Add one pinned Question"
                  confirmLabel="Add pinned Question"
                  instructions="The selected Question is appended with its exact Published Revision. Cancel leaves this Pool unchanged."
                  onConfirm={append}
                  onCancel={() => setPickerOpen(false)}
                />
              </Show>
            </fieldset>
          </>
        )}
      </Show>
      <Show when={!props.mutationsEnabled}>
        <p class="assessment-editor-note">
          {props.entry.availability === "available"
            ? "Save or reload the pending Assessment changes before changing this Pool."
            : "This retained unavailable Pool Entry is read-only."}
        </p>
      </Show>
    </section>
  );
}
