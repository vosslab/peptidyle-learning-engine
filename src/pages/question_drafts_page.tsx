// question_drafts_page.tsx - private Authoring Workspace entry and Draft Question list.

import { useNavigate } from "@solidjs/router";
import { Show, createResource, createSignal, type JSX } from "solid-js";

import { PageFrame } from "../components/page_frame/page_frame";
import { QuestionImportScreen } from "./question_import_screen";
import type { RecordContent } from "../components/data_sheet/record_content";
import { SearchBlock } from "../components/search_block/search_block";
import { SearchControls } from "../components/search_block/search_controls";
import { createLocalCompleteSearch } from "../components/search_block/local_complete_search_state";
import {
  createQuestionDraftClient,
  isAllowedWebworkPgPath,
  isQuestionDraftCreationFormat,
  QUESTION_DRAFT_CREATION_FORMATS,
  type DraftQuestionSummary,
  type QuestionDraftCreationFormat,
} from "../api/question_draft_client";
import {
  compareDraftEditNumbers,
  questionDraftSearchFields,
  type DraftSearchSortKey,
} from "./question_drafts_search_fields";
import "./question_drafts_page.css";

type PageMessage = {
  readonly kind: "error" | "success";
  readonly text: string;
};

function shortContentPreview(questionDescription: string): string | undefined {
  const normalizedDescription = questionDescription.replace(/\s+/gu, " ").trim();
  if (normalizedDescription.length === 0) return undefined;
  const maximumPreviewLength = 180;
  if (normalizedDescription.length <= maximumPreviewLength) return normalizedDescription;
  return `${normalizedDescription.slice(0, maximumPreviewLength - 3).trimEnd()}...`;
}

function draftContent(
  draft: DraftQuestionSummary,
  deleting: () => boolean,
  requestDelete: (draft: DraftQuestionSummary) => void,
): RecordContent {
  return {
    title: draft.questionTitle,
    description: shortContentPreview(draft.questionDescription),
    details: [{ kind: "text", label: "Edit", value: draft.draftQuestionEditNumber }],
    actions: [
      {
        id: "open-draft",
        kind: "link",
        label: "Open draft",
        href: `/authoring/drafts/${encodeURIComponent(draft.draftQuestionId)}`,
        title: `Open draft: ${draft.questionTitle}`,
      },
      {
        id: "delete-draft",
        kind: "command",
        label: "Delete draft",
        title: `Delete draft: ${draft.questionTitle}`,
        disabled: deleting(),
        onClick: () => requestDelete(draft),
      },
    ],
  };
}

/** Lists only the signed-in Instructor's private Draft Questions. */
export function QuestionDraftsPage(): JSX.Element {
  const navigate = useNavigate();
  const draftClient = createQuestionDraftClient();
  const [drafts, { refetch }] = createResource(() => draftClient.listDrafts());
  const [creating, setCreating] = createSignal(false);
  const [importing, setImporting] = createSignal(false);
  const [creationFormat, setCreationFormat] =
    createSignal<QuestionDraftCreationFormat>("pleQuestionJson");
  const [webworkPgPath, setWebworkPgPath] = createSignal("");
  const [deleting, setDeleting] = createSignal(false);
  const [pendingDelete, setPendingDelete] = createSignal<DraftQuestionSummary>();
  const [deleteMessage, setDeleteMessage] = createSignal<string>();
  const [message, setMessage] = createSignal<PageMessage>();
  const draftSearch = createLocalCompleteSearch(
    {
      rows: drafts,
      loading: () => drafts.loading,
      error: (): unknown => drafts.error,
      refetchSource: refetch,
    },
    {
      fields: questionDraftSearchFields,
      rowId: (draft) => draft.draftQuestionId,
      keyword: (draft) => `${draft.questionTitle} ${draft.questionDescription}`,
      compare: (key: DraftSearchSortKey, left, right) =>
        key === "editNumber" ? compareDraftEditNumbers(left, right) : undefined,
      content: (draft) => draftContent(draft, deleting, requestDelete),
    },
  );

  async function createDraft(): Promise<void> {
    if (creating()) return;
    setCreating(true);
    setMessage(undefined);
    try {
      const created = await draftClient.createDraft({
        format: creationFormat(),
        ...(creationFormat() === "pleQuestionJson" ? {} : { webworkPgPath: webworkPgPath() }),
      });
      navigate(`/authoring/drafts/${encodeURIComponent(created.draftQuestionId)}`);
    } catch (error: unknown) {
      setMessage({
        kind: "error",
        text: error instanceof Error ? error.message : "A new private draft could not be created.",
      });
    } finally {
      setCreating(false);
    }
  }

  function cancelDelete(): void {
    if (deleting()) return;
    setDeleteMessage(undefined);
    setPendingDelete(undefined);
  }

  function requestDelete(draft: DraftQuestionSummary): void {
    if (deleting()) return;
    setDeleteMessage(undefined);
    setPendingDelete(draft);
  }

  async function refreshDrafts(): Promise<void> {
    if (deleting()) return;
    setDeleteMessage(undefined);
    setPendingDelete(undefined);
    await refetch();
  }

  async function deleteDraft(): Promise<void> {
    const draft = pendingDelete();
    if (draft === undefined || deleting()) return;
    setDeleting(true);
    setDeleteMessage(undefined);
    try {
      const result = await draftClient.deleteDraft(draft);
      if (result === "preconditionFailed") {
        setDeleteMessage("This Draft Question changed. Refresh the list before deleting it.");
        return;
      }
      await refetch();
      setPendingDelete(undefined);
      setMessage({ kind: "success", text: "Private Draft Question deleted." });
    } catch (error: unknown) {
      setDeleteMessage(
        error instanceof Error
          ? error.message
          : "This private Draft Question could not be deleted.",
      );
    } finally {
      setDeleting(false);
    }
  }

  return (
    <PageFrame routeSurface="questionDrafts">
      <Show when={!importing()}>
        <section class="question-draft-creation" aria-label="Create a Draft Question">
          <form
            class="question-draft-creation__form"
            onSubmit={(event) => {
              event.preventDefault();
              void createDraft();
            }}
          >
            <label class="question-draft-creation__field">
              <span>Question source format</span>
              <select
                value={creationFormat()}
                disabled={creating()}
                onChange={(event) => {
                  const selected = event.currentTarget.value;
                  if (isQuestionDraftCreationFormat(selected)) setCreationFormat(selected);
                }}
              >
                {QUESTION_DRAFT_CREATION_FORMATS.map(({ format, label }) => (
                  <option value={format}>{label}</option>
                ))}
              </select>
            </label>
            <Show when={creationFormat() !== "pleQuestionJson"}>
              <label class="question-draft-creation__field">
                <span>Registered WebWork PG path</span>
                <input
                  type="text"
                  value={webworkPgPath()}
                  disabled={creating()}
                  aria-describedby="question-draft-webwork-path-help"
                  aria-invalid={!isAllowedWebworkPgPath(webworkPgPath())}
                  onInput={(event) => setWebworkPgPath(event.currentTarget.value)}
                />
              </label>
              <p id="question-draft-webwork-path-help">
                Use a registered relative PG path. It stays fixed for this Draft.
              </p>
            </Show>
            <button
              class="primary-action"
              type="submit"
              disabled={
                creating() ||
                (creationFormat() !== "pleQuestionJson" && !isAllowedWebworkPgPath(webworkPgPath()))
              }
            >
              {creating() ? "Creating private draft..." : "Create Draft Question"}
            </button>
            <button
              class="quiet-action"
              type="button"
              disabled={creating() || importing()}
              onClick={() => setImporting(true)}
            >
              Import questions
            </button>
          </form>
        </section>
      </Show>
      <Show when={importing()}>
        <QuestionImportScreen
          onClose={() => {
            setImporting(false);
            void refetch();
          }}
        />
      </Show>
      <Show when={message()}>
        {(value) => (
          <p class={value().kind === "error" ? "inline-error" : "calm-status"} role="status">
            {value().text}
          </p>
        )}
      </Show>
      <Show when={!importing()}>
        <SearchBlock
          layout="above"
          controls={<SearchControls state={draftSearch.state} textLabel="Search My Drafts" />}
          state={draftSearch.state}
          collection={{
            ariaLabel: "Private Draft Questions",
            ...draftSearch.collection,
            get emptyState() {
              return {
                title:
                  draftSearch.state.query().text.length > 0 ||
                  draftSearch.state.query().filters.size > 0
                    ? "No Draft Questions match these filters"
                    : "No Draft Questions yet",
              };
            },
          }}
        />
      </Show>
      <Show when={pendingDelete()}>
        {(draft) => (
          <dialog
            class="confirmation-dialog"
            aria-labelledby="delete-draft-heading"
            aria-describedby="delete-draft-copy"
            ref={(element) => queueMicrotask(() => element.showModal())}
            onCancel={(event) => {
              event.preventDefault();
              cancelDelete();
            }}
          >
            <h2 id="delete-draft-heading">Delete this Draft Question?</h2>
            <p id="delete-draft-copy">
              Delete <strong>{draft().questionTitle}</strong>? This permanently removes the private
              draft.
            </p>
            <Show when={deleteMessage()}>
              {(value) => (
                <section class="inline-error" role="alert">
                  <p>{value()}</p>
                  <button
                    class="quiet-action"
                    type="button"
                    disabled={deleting()}
                    onClick={() => void refreshDrafts()}
                  >
                    Refresh drafts
                  </button>
                </section>
              )}
            </Show>
            <div class="action-row">
              <button
                ref={(element) => queueMicrotask(() => element.focus())}
                class="quiet-action"
                type="button"
                disabled={deleting()}
                onClick={cancelDelete}
              >
                Keep draft
              </button>
              <button
                class="primary-action"
                type="button"
                disabled={deleting()}
                onClick={() => void deleteDraft()}
              >
                {deleting() ? "Deleting private draft..." : "Delete draft"}
              </button>
            </div>
          </dialog>
        )}
      </Show>
    </PageFrame>
  );
}
