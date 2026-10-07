// question_drafts_page.tsx - private Authoring Workspace entry and Draft Question list.

import { useNavigate } from "@solidjs/router";
import { Show, createMemo, createResource, createSignal, type JSX } from "solid-js";

import { PageFrame } from "../components/page_frame";
import {
  RecordList,
  type RecordContent,
  type RecordListState,
} from "../components/record_list/record_list";
import { parseDraftQuestionId, type DraftQuestionRouteId } from "../navigation/public_route";
import type { PublishedQuestionRevisionTuple } from "../../generated/api/PublishedQuestionRevisionTuple";
import { decodePublishedQuestionRevisionTuple } from "../api/decoders/shared";
import {
  createQuestionDraftCreationClient,
  isAllowedWebworkPgPath,
  isQuestionDraftCreationFormat,
  QUESTION_DRAFT_CREATION_FORMATS,
  type QuestionDraftCreationFormat,
} from "../api/question_draft_creation";
import "./question_drafts_page.css";

type DraftSummary = {
  readonly draftQuestionId: DraftQuestionRouteId;
  readonly draftQuestionEditNumber: string;
  readonly questionTitle: string;
  readonly questionDescription: string;
  readonly parentPublishedQuestionRevisionTuple: PublishedQuestionRevisionTuple | null;
};

function isParentRevisionTuple(value: unknown): value is PublishedQuestionRevisionTuple | null {
  if (value === null) return true;
  try {
    decodePublishedQuestionRevisionTuple(value, "draft.parentPublishedQuestionRevisionTuple", true);
    return true;
  } catch {
    return false;
  }
}

function isDraftQuestionEditNumber(value: unknown): value is string {
  return (
    typeof value === "string" &&
    /^[1-9][0-9]*$/u.test(value) &&
    BigInt(value) <= 9_223_372_036_854_775_807n
  );
}

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
  draft: DraftSummary,
  deleting: () => boolean,
  requestDelete: (draft: DraftSummary) => void,
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

async function listDrafts(): Promise<ReadonlyArray<DraftSummary>> {
  const response = await fetch("/api/authoring/drafts", {
    headers: { accept: "application/json" },
    credentials: "same-origin",
    cache: "no-store",
  });
  if (!response.ok) throw new Error("My Question Drafts is unavailable.");
  const value: unknown = await response.json();
  if (!isDraftList(value)) throw new Error("My Question Drafts returned an invalid response.");
  return value.items;
}

function isDraftList(value: unknown): value is { readonly items: ReadonlyArray<DraftSummary> } {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return false;
  const record = value as Record<string, unknown>;
  if (Object.keys(record).length !== 1 || !Array.isArray(record.items)) return false;
  return record.items.every((item) => {
    if (typeof item !== "object" || item === null || Array.isArray(item)) return false;
    const summary = item as Record<string, unknown>;
    return (
      Object.keys(summary).length === 5 &&
      typeof summary.draftQuestionId === "string" &&
      parseDraftQuestionId(summary.draftQuestionId) !== null &&
      isDraftQuestionEditNumber(summary.draftQuestionEditNumber) &&
      typeof summary.questionTitle === "string" &&
      typeof summary.questionDescription === "string" &&
      isParentRevisionTuple(summary.parentPublishedQuestionRevisionTuple)
    );
  });
}

/** Lists only the signed-in Instructor's private Draft Questions. */
export function QuestionDraftsPage(): JSX.Element {
  const navigate = useNavigate();
  const [drafts, { refetch }] = createResource(listDrafts);
  const draftCreationClient = createQuestionDraftCreationClient();
  const draftsLoadFailed = createMemo(() => drafts.error !== undefined);
  const [creating, setCreating] = createSignal(false);
  const [creationFormat, setCreationFormat] =
    createSignal<QuestionDraftCreationFormat>("pleQuestionJson");
  const [webworkPgPath, setWebworkPgPath] = createSignal("");
  const [deleting, setDeleting] = createSignal(false);
  const [pendingDelete, setPendingDelete] = createSignal<DraftSummary>();
  const [deleteMessage, setDeleteMessage] = createSignal<string>();
  const [message, setMessage] = createSignal<PageMessage>();
  const draftListState = createMemo<RecordListState>(() => {
    if (drafts.loading) {
      return { kind: "loading", label: "Loading your private Draft Questions..." };
    }
    if (draftsLoadFailed()) {
      return {
        kind: "error",
        title: "My Question Drafts is unavailable.",
        message: "Try loading your Draft Questions again.",
        retry: (): void => void refetch(),
      };
    }
    return { kind: "ready" };
  });

  async function createDraft(): Promise<void> {
    if (creating()) return;
    setCreating(true);
    setMessage(undefined);
    try {
      const created = await draftCreationClient.createDraft({
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

  function requestDelete(draft: DraftSummary): void {
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
      // ASVS 2.2.2: the server, not this UI, validates identity, ownership, and this precondition.
      const response = await fetch(
        `/api/authoring/drafts/${encodeURIComponent(draft.draftQuestionId)}`,
        {
          method: "DELETE",
          headers: {
            accept: "application/json",
            "if-match": `"${draft.draftQuestionEditNumber}"`,
          },
          credentials: "same-origin",
          cache: "no-store",
        },
      );
      if (response.status === 412) {
        setDeleteMessage("This Draft Question changed. Refresh the list before deleting it.");
        return;
      }
      if (!response.ok) throw new Error("This private Draft Question could not be deleted.");
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
    <PageFrame
      routeSurface="questionDrafts"
      eyebrow="Private instructor authoring"
      title="My Draft Questions"
      lede="Draft Questions stay in your Authoring Workspace until you publish a validated question."
    >
      <section class="question-draft-creation" aria-labelledby="question-draft-creation-heading">
        <h2 id="question-draft-creation-heading">Create a private Draft Question</h2>
        <p>Choose the source format for this Draft. Its Backend, format, and PG path stay fixed.</p>
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
              Enter the allowed relative PG path for the configured WebWork Library. This path
              becomes part of the Draft binding.
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
        </form>
      </section>
      <Show when={message()}>
        {(value) => (
          <p class={value().kind === "error" ? "inline-error" : "calm-status"} role="status">
            {value().text}
          </p>
        )}
      </Show>
      <RecordList
        rows={drafts() ?? []}
        content={(draft) => draftContent(draft, deleting, requestDelete)}
        recordId={(draft) => draft.draftQuestionId}
        state={draftListState()}
        ariaLabel="Private Draft Questions"
        emptyState={{
          title: "No Draft Questions yet",
          message: "Create a Draft Question to begin authoring privately.",
        }}
      />
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
