// Owner/Sysadmin editor for one ordinary Pool's complete exact tuple set.

import { createMemo, Show, createEffect, createSignal, onCleanup, type JSX } from "solid-js";

import { MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY } from "../../generated/api/MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY";
import type { QuestionPoolView } from "../../generated/api/QuestionPoolView";
import type { ContentClassificationClient } from "../api/content_classification";
import type { QuestionPoolMembersClient } from "../api/question_pool_members";
import type { QuestionBulkMetadataClient } from "../api/question_bulk_metadata";
import type { QuestionAvailabilityClient } from "../api/question_availability";
import type {
  QuestionLibraryBrowseRepository,
  LibraryObjectQuestionRow,
  LibrarySearchRow,
} from "../pages/library_page_model";
import {
  type QuestionPickerEligibility,
  type QuestionPickerSelection,
} from "../features/question_picker";
import { questionPoolMemberTuples } from "./question_pool_create_model";
import {
  addQuestionPoolMembers,
  createQuestionPoolMembersDraft,
  questionPoolMembersAreDirty,
  questionPoolMembersDraftAfterReload,
  removeQuestionPoolMember,
  saveQuestionPoolMembersDraft,
  type QuestionPoolMemberDraft,
} from "./question_pool_members_model";
import { questionNativePreview } from "./question_native_preview";
import type { RecordContent } from "./data_sheet/record_content";
import {
  RecordTransferProvider,
  type RecordTransferBinding,
  type RecordTransferCommand,
} from "./data_sheet/record_interactions";
import {
  comparePoolMemberFields,
  poolMemberRowId,
  questionPoolMemberFields,
} from "./question_pool_member_fields";
import { SearchBlock } from "./search_block/search_block";
import { SearchControls } from "./search_block/search_controls";
import { createLocalCompleteSearch } from "./search_block/local_complete_search_state";
import { QuestionPoolAvailableQuestions } from "./question_pool_available_questions";
import { UnsavedChangesGuard, trackEditorSave } from "./unsaved_changes_guard";
import "./question_pool_members_editor.css";

export interface QuestionPoolMembersEditorProps {
  readonly pool: QuestionPoolView;
  readonly client: QuestionPoolMembersClient;
  readonly questionLibrary: QuestionLibraryBrowseRepository;
  readonly classificationClient: ContentClassificationClient;
  readonly getQuestionRevision: QuestionAvailabilityClient["getQuestionRevision"];
  readonly getCurrentQuestionSharedMetadata: QuestionBulkMetadataClient["getCurrentQuestionSharedMetadata"];
  readonly reloadPool: () => Promise<QuestionPoolView | null | undefined>;
  readonly onSaved: () => void;
}

/** Saves complete unordered member sets and keeps local drafts through failures. */
export function QuestionPoolMembersEditor(props: QuestionPoolMembersEditorProps): JSX.Element {
  const [draft, setDraft] = createSignal(createQuestionPoolMembersDraft(props.pool));
  let disposed = false;
  onCleanup(() => {
    disposed = true;
  });
  const [saving, setSaving] = createSignal(false);
  const [adding, setAdding] = createSignal(false);
  const [reloading, setReloading] = createSignal(false);
  const [error, setError] = createSignal<string>();
  const [notice, setNotice] = createSignal<string>();
  const memberSource = createMemo(() => draft().members);
  const memberFields = questionPoolMemberFields<QuestionPoolMemberDraft>();
  const removeMemberCommand = (member: QuestionPoolMemberDraft): RecordTransferCommand => ({
    label: "Remove",
    disabled: draft().members.length <= 1 || saving() || adding() || reloading(),
    run: (): void => {
      if (saving() || adding() || reloading() || draft().members.length <= 1) return;
      setDraft((current) =>
        removeQuestionPoolMember(
          current,
          member.publishedQuestionRevisionTuple.publishedQuestionId,
        ),
      );
      setError(undefined);
      setNotice(undefined);
    },
  });
  const memberContent = (member: QuestionPoolMemberDraft): RecordContent => {
    const command = removeMemberCommand(member);
    return {
      title: member.questionTitle,
      preview: questionNativePreview(member),
      details: [
        {
          kind: "questionId",
          questionTitle: member.questionTitle,
          displayId: member.publishedQuestionRevisionTuple.publishedQuestionId,
        },
        {
          kind: "text",
          label: "Revision",
          value: String(member.publishedQuestionRevisionTuple.revisionNumber),
        },
        {
          kind: "text",
          label: "Question License",
          value: member.questionLicense ?? "Unavailable",
        },
      ],
      actions: [
        {
          id: "remove",
          kind: "command",
          label: command.label,
          disabled: command.disabled,
          onClick: (): void => {
            void command.run();
          },
        },
      ],
    };
  };
  const memberSearch = createLocalCompleteSearch(
    { rows: memberSource },
    {
      fields: memberFields,
      rowId: poolMemberRowId,
      keyword: (member) =>
        `${member.questionTitle} ${member.publishedQuestionRevisionTuple.publishedQuestionId}`,
      compare: comparePoolMemberFields,
      content: memberContent,
    },
  );
  createEffect(() => {
    const pool = props.pool;
    if (draft().questionPoolId !== pool.questionPoolId) {
      setDraft(createQuestionPoolMembersDraft(pool));
      setError(undefined);
      setNotice(undefined);
    }
  });
  const eligibility = (): QuestionPickerEligibility => ({
    disciplineUuid: props.pool.metadata.disciplineUuid,
    subjectUuid: props.pool.metadata.subjectUuid,
    questionType: props.pool.questionType,
    backend: props.pool.backend,
  });

  async function addSelection(selection: QuestionPickerSelection): Promise<boolean> {
    if (adding() || saving() || reloading()) return false;
    const poolId = draft().questionPoolId;
    setAdding(true);
    setError(undefined);
    setNotice(undefined);
    try {
      const existingIds = new Set(
        draft().members.map((member) => member.publishedQuestionRevisionTuple.publishedQuestionId),
      );
      const additions = selection.questions.filter(
        (question) => !existingIds.has(question.questionId),
      );
      if (additions.length === 0) {
        setNotice("Those Questions are already selected.");
        return false;
      }
      const resolved = await questionPoolMemberTuples(
        { questionIds: additions.map((item) => item.questionId), questions: additions },
        props.getQuestionRevision,
        props.getCurrentQuestionSharedMetadata,
        eligibility(),
      );
      if (disposed || draft().questionPoolId !== poolId) return false;
      const selectedById = new Map(additions.map((item) => [item.questionId, item.row]));
      const addedMembers = resolved.map((tuple) => ({
        publishedQuestionRevisionTuple: tuple,
        nativePrompt: selectedById.get(tuple.publishedQuestionId)?.nativePrompt ?? null,
        questionTitle:
          selectedById.get(tuple.publishedQuestionId)?.questionTitle ?? tuple.publishedQuestionId,
        questionLicense: selectedById.get(tuple.publishedQuestionId)?.questionLicense ?? null,
      }));
      const nextDraft = addQuestionPoolMembers(draft(), addedMembers);
      if (nextDraft.members.length > MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY) {
        setError("The Pool member limit has been reached. Remove a Question before adding more.");
        return false;
      }
      setDraft(nextDraft);
      return true;
    } catch {
      if (disposed || draft().questionPoolId !== poolId) return false;
      setError(
        "The selected Questions no longer meet this Pool's requirements. Review and try again.",
      );
      return false;
    } finally {
      setAdding(false);
    }
  }

  async function addQuestion(row: LibraryObjectQuestionRow): Promise<boolean> {
    return addSelection({
      questionIds: [row.displayId],
      questions: [{ questionId: row.displayId, row }],
    });
  }

  const memberTransfer: RecordTransferBinding<QuestionPoolMemberDraft> = {
    panelKey: "pool-members",
    targetPanelKey: "available-questions",
    command: (member) => removeMemberCommand(member),
  };
  const availableTransfer: RecordTransferBinding<LibrarySearchRow> = {
    panelKey: "available-questions",
    targetPanelKey: "pool-members",
    command: (row) => {
      if (row.kind !== "question") {
        return { label: "Add", disabled: true, run: (): void => undefined };
      }
      const included = (): boolean =>
        draft().members.some(
          (member) => member.publishedQuestionRevisionTuple.publishedQuestionId === row.displayId,
        );
      const disabled = (): boolean =>
        included() ||
        saving() ||
        adding() ||
        reloading() ||
        draft().members.length >= MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY;
      return {
        label: included() ? "Included" : "Add",
        disabled: disabled(),
        run: (): void => {
          if (disabled()) return;
          void addQuestion(row);
        },
      };
    },
  };

  async function save(): Promise<boolean> {
    const current = draft();
    if (saving() || adding() || reloading()) return false;
    if (!questionPoolMembersAreDirty(current)) return true;
    setSaving(true);
    setError(undefined);
    setNotice(undefined);
    try {
      const result = await saveQuestionPoolMembersDraft(
        current,
        props.client.saveQuestionPoolMembers,
      );
      if (disposed || draft() !== current) return false;
      if (result.kind === "failed") {
        setError(
          "Pool Questions could not be saved. Your draft is still here; retry or reload the Pool.",
        );
        return false;
      }
      setDraft(result.draft);
      setNotice("Pool Questions saved.");
      props.onSaved();
      return true;
    } finally {
      setSaving(false);
    }
  }

  async function reload(): Promise<void> {
    if (reloading() || adding() || saving()) return;
    const draftAtReloadStart = draft();
    setReloading(true);
    setError(undefined);
    setNotice(undefined);
    try {
      const latest = await props.reloadPool();
      if (
        latest === null ||
        latest === undefined ||
        latest.questionPoolId !== draftAtReloadStart.questionPoolId
      ) {
        throw new Error("Current Pool detail is unavailable.");
      }
      if (disposed || draft().questionPoolId !== draftAtReloadStart.questionPoolId) return;
      const result = questionPoolMembersDraftAfterReload(draft(), draftAtReloadStart, latest);
      setDraft(result.draft);
      if (result.kind === "stale") {
        setNotice("Pool Questions changed while loading. Your newer draft is still here.");
      }
    } catch {
      setError("Current Pool Questions could not be loaded. Your draft is still here.");
    } finally {
      setReloading(false);
    }
  }

  const trackedSave = trackEditorSave(save);

  return (
    <RecordTransferProvider>
      <section
        class="question-pool-members-editor"
        aria-labelledby="question-pool-members-editor-heading"
      >
        <UnsavedChangesGuard
          dirty={() => questionPoolMembersAreDirty(draft())}
          save={trackedSave.save}
          inFlightSave={trackedSave.inFlightSave}
          copy={{
            heading: "Discard Pool Question changes?",
            description: "Your unsaved Pool membership changes will be discarded.",
          }}
        />
        <h3 id="question-pool-members-editor-heading">Edit Pool Questions</h3>
        <p>Changes take effect when you save.</p>
        <p class="question-pool-members-editor-version">
          Based on Pool Edit Number {draft().acknowledgedEditNumber}
        </p>
        <div class="question-pool-members-editor-actions">
          <button
            class="primary-action"
            type="button"
            disabled={!questionPoolMembersAreDirty(draft()) || saving() || adding() || reloading()}
            onClick={() => void trackedSave.save()}
          >
            {saving() ? "Saving..." : "Save Pool Questions"}
          </button>
          <Show when={questionPoolMembersAreDirty(draft())}>
            <button
              class="quiet-action"
              type="button"
              disabled={saving() || adding() || reloading()}
              onClick={() => void reload()}
            >
              {reloading() ? "Loading Pool Questions..." : "Reload Pool Questions"}
            </button>
          </Show>
        </div>
        <Show when={error()}>
          {(message) => (
            <p class="question-pool-members-editor-error" role="alert">
              {message()}
            </p>
          )}
        </Show>
        <Show when={notice()}>
          {(message) => (
            <p class="question-pool-members-editor-notice" role="status">
              {message()}
            </p>
          )}
        </Show>
        <div class="question-pool-members-editor-panels">
          <section aria-labelledby="question-pool-current-heading">
            <h4 id="question-pool-current-heading">Current Pool Questions</h4>
            <SearchBlock
              layout="above"
              controls={
                <SearchControls state={memberSearch.state} textLabel="Search Pool Questions" />
              }
              state={memberSearch.state}
              collection={{
                ariaLabel: "Questions being edited",
                fields: memberFields,
                query: memberSearch.query,
                transfer: memberTransfer,
                emptyState: { title: "No Pool Questions match this search." },
              }}
            />
          </section>
          <section aria-labelledby="question-pool-available-heading">
            <h4 id="question-pool-available-heading">Library Questions</h4>
            <Show when={props.pool.questionPoolId} keyed>
              {(_poolId) => (
                <QuestionPoolAvailableQuestions
                  repository={props.questionLibrary}
                  classificationClient={props.classificationClient}
                  eligibility={eligibility()}
                  includedIds={
                    new Set(
                      draft().members.map(
                        (member) => member.publishedQuestionRevisionTuple.publishedQuestionId,
                      ),
                    )
                  }
                  disabled={
                    saving() ||
                    adding() ||
                    reloading() ||
                    draft().members.length >= MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY
                  }
                  onAdd={addQuestion}
                  transfer={availableTransfer}
                />
              )}
            </Show>
          </section>
        </div>
      </section>
    </RecordTransferProvider>
  );
}
