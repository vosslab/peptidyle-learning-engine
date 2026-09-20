// question_drafts_page.tsx - private Authoring Workspace entry and Draft Question list.

import { A, useNavigate } from "@solidjs/router";
import { For, Show, createMemo, createResource, createSignal, type JSX } from "solid-js";

import { createDefaultPleQuestionJsonSource } from "../features/ple_question_json_authoring/question_json_defaults";
import { PLE_QUESTION_JSON_MEDIA_TYPE } from "../features/ple_question_json_authoring/question_json_source";
import { serializePleQuestionJsonSource } from "../features/ple_question_json_authoring/question_json_codec";
import { parseDraftQuestionId, type DraftQuestionRouteId } from "../navigation/public_route";
import "./question_drafts_page.css";

type DraftSummary = {
  readonly draftQuestionId: DraftQuestionRouteId;
  readonly draftQuestionEditNumber: string;
  readonly questionTitle: string;
  readonly questionDescription: string;
};

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
      Object.keys(summary).length === 4 &&
      typeof summary.draftQuestionId === "string" &&
      parseDraftQuestionId(summary.draftQuestionId) !== null &&
      isDraftQuestionEditNumber(summary.draftQuestionEditNumber) &&
      typeof summary.questionTitle === "string" &&
      typeof summary.questionDescription === "string"
    );
  });
}

function createdDraftId(value: unknown): DraftQuestionRouteId | null {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return null;
  const record = value as Record<string, unknown>;
  if (
    Object.keys(record).length !== 2 ||
    typeof record.draftQuestionId !== "string" ||
    !isDraftQuestionEditNumber(record.draftQuestionEditNumber)
  ) {
    return null;
  }
  return parseDraftQuestionId(record.draftQuestionId);
}

/** Lists only the signed-in Instructor's private Draft Questions. */
export function QuestionDraftsPage(): JSX.Element {
  const navigate = useNavigate();
  const [drafts, { refetch }] = createResource(listDrafts);
  const draftsLoadFailed = createMemo(() => drafts.error !== undefined);
  const [creating, setCreating] = createSignal(false);
  const [deleting, setDeleting] = createSignal(false);
  const [pendingDelete, setPendingDelete] = createSignal<DraftSummary>();
  const [deleteMessage, setDeleteMessage] = createSignal<string>();
  const [message, setMessage] = createSignal<PageMessage>();

  async function createDraft(): Promise<void> {
    if (creating()) return;
    setCreating(true);
    setMessage(undefined);
    try {
      const response = await fetch("/api/authoring/drafts", {
        method: "POST",
        headers: {
          accept: "application/json",
          "content-type": PLE_QUESTION_JSON_MEDIA_TYPE,
        },
        body: serializePleQuestionJsonSource(createDefaultPleQuestionJsonSource()),
        credentials: "same-origin",
        cache: "no-store",
      });
      if (!response.ok) throw new Error("A new private draft could not be created.");
      const draftQuestionId = createdDraftId(await response.json());
      if (draftQuestionId === null) {
        throw new Error("A new private draft returned an invalid response.");
      }
      navigate(`/authoring/drafts/${encodeURIComponent(draftQuestionId)}`);
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
    <main class="page" data-route-surface="questionDrafts">
      <header>
        <p class="eyebrow">Private instructor authoring</p>
        <h1>My Draft Questions</h1>
        <p>
          Draft Questions stay in your Authoring Workspace until you publish a validated question.
        </p>
      </header>
      <p>
        <button
          class="primary-action"
          type="button"
          disabled={creating()}
          onClick={() => void createDraft()}
        >
          {creating() ? "Creating private draft..." : "New Draft Question"}
        </button>
      </p>
      <Show when={message()}>
        {(value) => (
          <p class={value().kind === "error" ? "inline-error" : "calm-status"} role="status">
            {value().text}
          </p>
        )}
      </Show>
      <Show when={drafts.loading}>
        <p class="calm-status" role="status">
          Loading your private Draft Questions...
        </p>
      </Show>
      <Show when={draftsLoadFailed()}>
        <section class="inline-error" role="alert">
          <p>My Question Drafts is unavailable.</p>
          <button class="quiet-action" type="button" onClick={() => void refetch()}>
            Retry
          </button>
        </section>
      </Show>
      <Show when={drafts()}>
        {(items) => (
          <Show
            when={items().length > 0}
            fallback={
              <p class="calm-status">Create a Draft Question to begin authoring privately.</p>
            }
          >
            <ul class="draft-question-list" aria-label="Private Draft Questions">
              <For each={items()}>
                {(draft) => (
                  <li class="draft-question-row">
                    <div class="draft-question-content">
                      <A
                        class="draft-question-title"
                        href={`/authoring/drafts/${encodeURIComponent(draft.draftQuestionId)}`}
                      >
                        {draft.questionTitle}
                      </A>
                      <Show when={draft.questionDescription.length > 0}>
                        <p class="draft-question-description">{draft.questionDescription}</p>
                      </Show>
                    </div>
                    <p class="draft-question-metadata">
                      <span>Private draft</span>
                      <span>Draft Question Edit Number {draft.draftQuestionEditNumber}</span>
                    </p>
                    <div class="draft-question-actions">
                      <A
                        class="quiet-link draft-question-edit"
                        href={`/authoring/drafts/${encodeURIComponent(draft.draftQuestionId)}`}
                        aria-label={`Edit draft: ${draft.questionTitle}`}
                      >
                        Edit draft
                      </A>
                      <button
                        class="quiet-action draft-question-delete"
                        type="button"
                        aria-label={`Delete draft: ${draft.questionTitle}`}
                        disabled={deleting()}
                        onClick={() => requestDelete(draft)}
                      >
                        Delete draft
                      </button>
                    </div>
                  </li>
                )}
              </For>
            </ul>
          </Show>
        )}
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
    </main>
  );
}
