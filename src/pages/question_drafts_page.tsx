// question_drafts_page.tsx - private Authoring Workspace entry and Draft Question list.

import { A, useNavigate } from "@solidjs/router";
import { For, Show, createResource, createSignal, type JSX } from "solid-js";

import type { DraftQuestionReference } from "../../generated/api/DraftQuestionReference";
import { createDefaultPleQuestionJsonSource } from "../features/ple_question_json_authoring/question_json_defaults";
import {
  PLE_QUESTION_JSON_MEDIA_TYPE,
} from "../features/ple_question_json_authoring/question_json_source";
import { serializePleQuestionJsonSource } from "../features/ple_question_json_authoring/question_json_codec";
import { parseDraftQuestionReference } from "../navigation/public_route";

type DraftSummary = {
  readonly draftQuestion: DraftQuestionReference;
  readonly editNumber: number;
  readonly questionTitle: string;
  readonly questionDescription: string;
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
      typeof summary.draftQuestion === "string" &&
      parseDraftQuestionReference(summary.draftQuestion) !== null &&
      Number.isSafeInteger(summary.editNumber) &&
      typeof summary.questionTitle === "string" &&
      typeof summary.questionDescription === "string"
    );
  });
}

function createdDraftReference(value: unknown): DraftQuestionReference | null {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return null;
  const record = value as Record<string, unknown>;
  if (
    Object.keys(record).length !== 2 ||
    typeof record.draftQuestion !== "string" ||
    !Number.isSafeInteger(record.editNumber)
  ) {
    return null;
  }
  return parseDraftQuestionReference(record.draftQuestion);
}

/** Lists only the signed-in Instructor's private Draft Questions. */
export function QuestionDraftsPage(): JSX.Element {
  const navigate = useNavigate();
  const [drafts, { refetch }] = createResource(listDrafts);
  const [creating, setCreating] = createSignal(false);
  const [message, setMessage] = createSignal<string>();

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
      const reference = createdDraftReference(await response.json());
      if (reference === null) throw new Error("A new private draft returned an invalid response.");
      navigate(`/authoring/drafts/${encodeURIComponent(reference)}`);
    } catch (error: unknown) {
      setMessage(error instanceof Error ? error.message : "A new private draft could not be created.");
    } finally {
      setCreating(false);
    }
  }

  return (
    <main class="page" data-route-surface="questionDrafts">
      <header>
        <p class="eyebrow">Private instructor authoring</p>
        <h1>My Question Drafts</h1>
        <p>Draft Questions stay in your Authoring Workspace until you publish a validated question.</p>
      </header>
      <p>
        <button class="primary-action" type="button" disabled={creating()} onClick={() => void createDraft()}>
          {creating() ? "Creating private draft..." : "New Draft Question"}
        </button>
      </p>
      <Show when={message()}>{(value) => <p class="inline-error" role="alert">{value()}</p>}</Show>
      <Show when={drafts.loading}>
        <p class="calm-status" role="status">Loading your private Draft Questions...</p>
      </Show>
      <Show when={drafts.error}>
        <section class="inline-error" role="alert">
          <p>My Question Drafts is unavailable.</p>
          <button class="quiet-action" type="button" onClick={() => void refetch()}>Retry</button>
        </section>
      </Show>
      <Show when={drafts()}>
        {(items) => (
          <Show
            when={items().length > 0}
            fallback={<p class="calm-status">Create a Draft Question to begin authoring privately.</p>}
          >
            <ul class="question-library-list">
              <For each={items()}>
                {(draft) => (
                  <li>
                    <A href={`/authoring/drafts/${encodeURIComponent(draft.draftQuestion)}`}>
                      <strong>{draft.questionTitle}</strong>
                      <span>{draft.questionDescription}</span>
                      <small>{draft.draftQuestion} · Edit {draft.editNumber}</small>
                    </A>
                  </li>
                )}
              </For>
            </ul>
          </Show>
        )}
      </Show>
    </main>
  );
}
