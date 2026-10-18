// Current reusable Pool detail and Assessment-local selection count.

import { Show, createMemo, type JSX } from "solid-js";

import type { AssessmentEntry } from "../../../generated/api/AssessmentEntry";
import type { QuestionPoolView } from "../../../generated/api/QuestionPoolView";
import type { RecordContent } from "../../components/data_sheet/record_content";
import { SearchBlock } from "../../components/search_block/search_block";
import { SearchControls } from "../../components/search_block/search_controls";
import { createLocalCompleteSearch } from "../../components/search_block/local_complete_search_state";
import {
  questionPoolMemberFields,
  poolMemberRowId,
  comparePoolMemberFields,
} from "../../components/question_pool_member_fields";
import { questionNativePreview } from "../../components/question_native_preview";
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
  const memberRows = createMemo(() =>
    (props.pool?.members ?? []).map((member) => ({
      publishedQuestionRevisionTuple: member.publishedQuestionRevisionTuple,
      questionTitle: member.question.question_library.summary.metadata.questionTitle,
      questionLicense: member.question.question_library.summary.metadata.questionLicense,
      nativePrompt: member.question.question_library.nativePrompt,
    })),
  );
  type MemberRow = ReturnType<typeof memberRows>[number];
  const memberFields = questionPoolMemberFields<MemberRow>();
  const memberSearch = createLocalCompleteSearch(
    { rows: memberRows },
    {
      fields: memberFields,
      rowId: poolMemberRowId,
      compare: comparePoolMemberFields,
      keyword: (member) =>
        `${member.questionTitle} ${member.publishedQuestionRevisionTuple.publishedQuestionId}`,
      content: (member): RecordContent => ({
        title: member.questionTitle,
        preview: questionNativePreview(member),
        details: [
          {
            kind: "text",
            label: "Question ID",
            value: member.publishedQuestionRevisionTuple.publishedQuestionId,
          },
          {
            kind: "text",
            label: "Revision",
            value: String(member.publishedQuestionRevisionTuple.revisionNumber),
          },
        ],
        actions: [],
      }),
    },
  );

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
            <SearchBlock
              layout="above"
              state={memberSearch.state}
              controls={
                <SearchControls
                  state={memberSearch.state}
                  textLabel="Search referenced Pool members"
                />
              }
              collection={{
                ariaLabel: "Current reusable Pool members",
                fields: memberFields,
                query: memberSearch.query,
                emptyState: { title: "No current Pool members match this search." },
              }}
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
