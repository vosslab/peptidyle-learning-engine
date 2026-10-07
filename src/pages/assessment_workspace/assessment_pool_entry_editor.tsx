// Current reusable Pool detail and Assessment-local selection count.

import { Show, type JSX } from "solid-js";

import type { AssessmentEntry } from "../../../generated/api/AssessmentEntry";
import type { QuestionPoolView } from "../../../generated/api/QuestionPoolView";
import type { RecordContent } from "../../components/record_list/record_list";
import { RecordSequence } from "../../components/record_list/record_sequence";
import { CourseClassificationSummary } from "../../components/course_classification_summary";

export interface AssessmentPoolEntryEditorProps {
  readonly entry: Extract<AssessmentEntry, { readonly kind: "questionPool" }>;
  readonly pool: QuestionPoolView | undefined;
  readonly poolUnavailable: boolean;
  readonly mutationsEnabled: boolean;
  readonly busy: boolean;
  readonly onSelectionCount: (selectionCount: number) => void;
}

/** Renders one shared Pool and edits only the Assessment's requested count. */
export function AssessmentPoolEntryEditor(props: AssessmentPoolEntryEditorProps): JSX.Element {
  function submitSelectionCount(value: string): void {
    const selectionCount = Number(value);
    if (!Number.isSafeInteger(selectionCount) || selectionCount < 1 || props.pool === undefined) {
      return;
    }
    if (selectionCount !== props.entry.selectionCount) props.onSelectionCount(selectionCount);
  }

  return (
    <section
      class="assessment-pool-reference"
      aria-labelledby={`assessment-pool-${props.entry.id}`}
    >
      <h3 id={`assessment-pool-${props.entry.id}`}>
        {props.pool?.metadata.title ?? "Question Pool"}
      </h3>
      <p>
        Question Pool ID {props.entry.questionPoolId}, Edit {props.entry.questionPoolEditNumber}
      </p>
      <Show
        when={props.pool}
        fallback={
          <p class="assessment-editor-note">
            {props.poolUnavailable
              ? "This reusable Question Pool could not load. Reload the Assessment and try again."
              : "Loading the current reusable Question Pool..."}
          </p>
        }
      >
        {(pool) => (
          <>
            <p>{pool().metadata.description}</p>
            <CourseClassificationSummary value={pool().metadata} />
            <p>
              Owner {pool().ownerAccountId}; {pool().members.length} current Questions; Bloom
              Cognitive Process: {pool().bloom?.cognitiveProcess ?? "Not assigned"}; Bloom Knowledge
              Dimension: {pool().bloom?.knowledgeDimension ?? "Not assigned"}.
            </p>
            <p>
              This Assessment selects {props.entry.selectionCount} Questions from this shared Pool.
              Pool edits affect future selections; existing Attempts keep their selected Questions.
            </p>
            <RecordSequence
              rows={pool().members.map((member, index) => ({ member, index }))}
              content={(record): RecordContent => ({
                title: record.member.question.question_library.summary.metadata.questionTitle,
                details: [
                  {
                    kind: "text",
                    label: "Question ID",
                    value: record.member.publishedQuestionRevisionTuple.publishedQuestionId,
                  },
                  {
                    kind: "text",
                    label: "Revision",
                    value: String(record.member.publishedQuestionRevisionTuple.revisionNumber),
                  },
                ],
                actions: [],
              })}
              recordId={(record) =>
                record.member.publishedQuestionRevisionTuple.publishedQuestionId
              }
              state={{ kind: "ready" }}
              ariaLabel="Current reusable Pool members"
              emptyState={{ title: "This Question Pool has no current members." }}
            />
            <fieldset disabled={!props.mutationsEnabled || props.busy}>
              <legend>Questions selected for each Attempt</legend>
              <label class="assessment-editor-field">
                Selection count
                <input
                  type="number"
                  min="1"
                  value={props.entry.selectionCount}
                  onChange={(event) => submitSelectionCount(event.currentTarget.value)}
                />
              </label>
              <p class="assessment-editor-note">
                This count belongs to this Assessment. Save Questions to keep the change.
              </p>
            </fieldset>
          </>
        )}
      </Show>
      <Show when={!props.mutationsEnabled}>
        <p class="assessment-editor-note">
          {props.entry.availability === "available"
            ? "Save or reload the pending Assessment changes before editing its selection count."
            : "This retained unavailable Pool Entry is read-only."}
        </p>
      </Show>
    </section>
  );
}
