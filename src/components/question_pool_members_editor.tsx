// Owner/Sysadmin editor for one ordinary Pool's complete exact tuple set.

import { Show, createEffect, createMemo, createSignal, type JSX } from "solid-js";

import { MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY } from "../../generated/api/MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY";
import type { PublishedQuestionId } from "../../generated/api/PublishedQuestionId";
import type { QuestionPoolView } from "../../generated/api/QuestionPoolView";
import type { QuestionPoolMembersClient } from "../api/question_pool_members";
import type { QuestionBulkMetadataClient } from "../api/question_bulk_metadata";
import type { QuestionDetails } from "../../generated/api/QuestionDetails";
import type { QuestionLibraryBrowseRepository } from "../pages/library_page_model";
import {
  QuestionPicker,
  questionLibraryPickerSources,
  type QuestionPickerEligibility,
  type QuestionPickerSelection,
  type QuestionPickerSourceRepository,
} from "../features/question_picker";
import {
  questionPoolMemberTuples,
  questionPoolSourcePickerRepository,
} from "./question_pool_create_model";
import {
  addQuestionPoolMembers,
  createQuestionPoolMembersDraft,
  questionPoolMembersAreDirty,
  questionPoolMembersDraftAfterReload,
  removeQuestionPoolMember,
  saveQuestionPoolMembersDraft,
  sortQuestionPoolMemberDraftDisplay,
} from "./question_pool_members_model";
import { RecordSortControl } from "./record_list/record_sort_control";
import { RecordTable, type RecordTableColumn } from "./record_list/record_table";
import {
  questionPoolMemberSortOptions,
  type QuestionPoolMemberSort,
} from "./record_list/question_pool_member_sort";
import "./question_pool_members_editor.css";

export interface QuestionPoolMembersEditorProps {
  readonly pool: QuestionPoolView;
  readonly client: QuestionPoolMembersClient;
  readonly questionLibrary: QuestionLibraryBrowseRepository;
  readonly getQuestionDetails: (questionId: PublishedQuestionId) => Promise<QuestionDetails>;
  readonly getCurrentQuestionSharedMetadata: QuestionBulkMetadataClient["getCurrentQuestionSharedMetadata"];
  readonly reloadPool: () => Promise<QuestionPoolView | null | undefined>;
  readonly onSaved: () => void;
}

/** Saves complete unordered member sets and keeps local drafts through failures. */
export function QuestionPoolMembersEditor(props: QuestionPoolMembersEditorProps): JSX.Element {
  const [draft, setDraft] = createSignal(createQuestionPoolMembersDraft(props.pool));
  const [pickerOpen, setPickerOpen] = createSignal(false);
  const [saving, setSaving] = createSignal(false);
  const [adding, setAdding] = createSignal(false);
  const [reloading, setReloading] = createSignal(false);
  const [error, setError] = createSignal<string>();
  const [notice, setNotice] = createSignal<string>();
  const [memberSort, setMemberSort] = createSignal<QuestionPoolMemberSort>("as-loaded");
  const sortedDraftMembers = createMemo(() =>
    sortQuestionPoolMemberDraftDisplay(draft().members, memberSort()),
  );
  const draftMemberColumns: ReadonlyArray<
    RecordTableColumn<ReturnType<typeof sortedDraftMembers>[number]>
  > = [
    {
      id: "published-question-id",
      header: "Published Question ID",
      cell: (member) => member.publishedQuestionRevisionTuple.publishedQuestionId,
    },
    {
      id: "revision",
      header: "Revision",
      cell: (member) => String(member.publishedQuestionRevisionTuple.revisionNumber),
    },
    {
      id: "question-license",
      header: "Question License",
      cell: (member) => member.questionLicense ?? "Unavailable",
    },
    {
      id: "remove",
      header: "Actions",
      cell: (member) => (
        <button
          class="quiet-action"
          type="button"
          disabled={draft().members.length <= 1 || saving() || adding()}
          aria-label={`Remove ${member.questionTitle} from this Pool`}
          onClick={() => {
            if (saving() || adding()) return;
            setDraft((current) =>
              removeQuestionPoolMember(
                current,
                member.publishedQuestionRevisionTuple.publishedQuestionId,
              ),
            );
            setError(undefined);
            setNotice(undefined);
          }}
        >
          Remove
        </button>
      ),
    },
  ];
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
  const pickerRepository = (): QuestionPickerSourceRepository =>
    questionPoolSourcePickerRepository(props.questionLibrary, eligibility());

  async function addSelection(selection: QuestionPickerSelection): Promise<void> {
    if (adding() || saving() || reloading()) return;
    setPickerOpen(false);
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
        return;
      }
      const resolved = await questionPoolMemberTuples(
        { questionIds: additions.map((item) => item.questionId), questions: additions },
        props.getQuestionDetails,
        props.getCurrentQuestionSharedMetadata,
        eligibility(),
      );
      const selectedById = new Map(additions.map((item) => [item.questionId, item.row]));
      const addedMembers = resolved.map((tuple) => ({
        publishedQuestionRevisionTuple: tuple,
        questionTitle:
          selectedById.get(tuple.publishedQuestionId)?.questionTitle ?? tuple.publishedQuestionId,
        questionLicense: selectedById.get(tuple.publishedQuestionId)?.questionLicense ?? null,
      }));
      const nextDraft = addQuestionPoolMembers(draft(), addedMembers);
      if (nextDraft.members.length > MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY) {
        setError("The Pool member limit has been reached. Remove a Question before adding more.");
        return;
      }
      setDraft(nextDraft);
    } catch {
      setError(
        "The selected Questions no longer meet this Pool's requirements. Review and try again.",
      );
    } finally {
      setAdding(false);
    }
  }

  async function save(): Promise<void> {
    const current = draft();
    if (!questionPoolMembersAreDirty(current) || saving() || adding() || reloading()) return;
    setSaving(true);
    setError(undefined);
    setNotice(undefined);
    try {
      const result = await saveQuestionPoolMembersDraft(
        current,
        props.client.saveQuestionPoolMembers,
      );
      if (result.kind === "failed") {
        setError(
          "Pool Questions could not be saved. Your draft is still here; retry or reload the Pool.",
        );
        return;
      }
      setDraft(result.draft);
      setNotice("Pool Questions saved.");
      props.onSaved();
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

  return (
    <section
      class="question-pool-members-editor"
      aria-labelledby="question-pool-members-editor-heading"
    >
      <h3 id="question-pool-members-editor-heading">Edit Pool Questions</h3>
      <p>Changes take effect when you save.</p>
      <p class="question-pool-members-editor-version">
        Based on Pool Edit Number {draft().acknowledgedEditNumber}
      </p>
      <div class="question-pool-members-editor-actions">
        <button
          class="quiet-action"
          type="button"
          disabled={
            draft().members.length >= MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY ||
            saving() ||
            adding() ||
            reloading()
          }
          onClick={() => {
            setError(undefined);
            setNotice(undefined);
            setPickerOpen(true);
          }}
        >
          {adding() ? "Adding Questions..." : "Add Questions"}
        </button>
        <button
          class="primary-action"
          type="button"
          disabled={!questionPoolMembersAreDirty(draft()) || saving() || adding() || reloading()}
          onClick={() => void save()}
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
      <RecordSortControl
        label="Sort Pool Questions"
        options={questionPoolMemberSortOptions}
        value={memberSort()}
        onChange={setMemberSort}
      />
      <RecordTable
        rows={sortedDraftMembers()}
        columns={draftMemberColumns}
        rowId={(member) =>
          `${member.publishedQuestionRevisionTuple.publishedQuestionId}:${member.publishedQuestionRevisionTuple.revisionNumber}`
        }
        state={{ kind: "ready" }}
        rowHeader={{
          id: "question-title",
          header: "Question Title",
          content: (member) => member.questionTitle,
        }}
        ariaLabel="Questions being edited"
        emptyState={{ title: "No Questions are in this Pool." }}
      />
      <Show when={pickerOpen()}>
        <QuestionPicker
          repository={pickerRepository()}
          sources={questionLibraryPickerSources(false)}
          eligibility={eligibility()}
          mode="many"
          maximumSelection={MAX_QUESTION_POOL_ITEMS_PER_ASSESSMENT_ENTRY - draft().members.length}
          title="Add published Questions to this Pool"
          instructions="Questions added here use their current eligible Revisions. Existing Pool Revisions stay exact."
          confirmLabel="Add selected Questions"
          trigger={undefined}
          onConfirm={(selection) => void addSelection(selection)}
          onCancel={() => setPickerOpen(false)}
        />
      </Show>
    </section>
  );
}
