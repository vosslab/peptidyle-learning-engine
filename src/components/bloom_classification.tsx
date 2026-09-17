// Shared exact-Revision Bloom Classification display and correction editor.

import { For, Show, createEffect, createSignal, onCleanup, type JSX } from "solid-js";

import type { BloomClassificationCorrectionRequest } from "../../generated/api/BloomClassificationCorrectionRequest";
import type { BloomClassificationView } from "../../generated/api/BloomClassificationView";
import type { BloomCognitiveProcess } from "../../generated/api/BloomCognitiveProcess";
import type { BloomKnowledgeDimension } from "../../generated/api/BloomKnowledgeDimension";
import {
  BLOOM_COGNITIVE_PROCESSES,
  BLOOM_KNOWLEDGE_DIMENSIONS,
} from "../api/decoders/bloom_classification";
import { BloomClassificationConflictError } from "../api/http_client/error";
import "./bloom_classification.css";

export function BloomClassificationText(props: {
  readonly bloom: BloomClassificationView;
}): JSX.Element {
  return (
    <span>
      Bloom: {props.bloom.cognitiveProcess} / {props.bloom.knowledgeDimension}
    </span>
  );
}

type Notice = {
  readonly kind: "status" | "alert";
  readonly text: string;
};

export interface BloomClassificationEditorProps {
  readonly targetName: "Question" | "Question Pool";
  readonly revisionNumber: number;
  readonly bloom: BloomClassificationView;
  readonly save: (
    request: BloomClassificationCorrectionRequest,
  ) => Promise<BloomClassificationView>;
  /** Reloads the same exact Revision after a 412; it never resolves current/latest instead. */
  readonly loadCurrent: () => Promise<BloomClassificationView>;
  readonly onCurrent?: (bloom: BloomClassificationView) => void;
  readonly onConflictCurrent?: (bloom: BloomClassificationView) => void;
  readonly onAccepted?: (bloom: BloomClassificationView, changed: boolean) => void;
}

/**
 * Edits both independent dimensions as one CAS command.
 *
 * A stale response reloads only the exact target's classification. The draft is
 * retained and another Save always requires a separate Instructor action.
 */
export function BloomClassificationEditor(props: BloomClassificationEditorProps): JSX.Element {
  const [current, setCurrent] = createSignal(props.bloom);
  const [draftCognitive, setDraftCognitive] = createSignal<BloomCognitiveProcess>(
    props.bloom.cognitiveProcess,
  );
  const [draftKnowledge, setDraftKnowledge] = createSignal<BloomKnowledgeDimension>(
    props.bloom.knowledgeDimension,
  );
  const [editing, setEditing] = createSignal(false);
  const [pending, setPending] = createSignal(false);
  const [conflict, setConflict] = createSignal<BloomClassificationView>();
  const [notice, setNotice] = createSignal<Notice>();
  let operation = 0;
  let disposed = false;
  let correctionTrigger: HTMLButtonElement | undefined;
  let cognitiveProcessSelect: HTMLSelectElement | undefined;

  createEffect(() => {
    if (editing() || pending()) return;
    setCurrent(props.bloom);
  });
  onCleanup(() => {
    disposed = true;
    ++operation;
  });

  function focusCognitiveProcess(): void {
    queueMicrotask(() => cognitiveProcessSelect?.focus());
  }

  function focusCorrectionTrigger(): void {
    queueMicrotask(() => correctionTrigger?.focus());
  }

  function beginEditing(): void {
    const value = current();
    setDraftCognitive(value.cognitiveProcess);
    setDraftKnowledge(value.knowledgeDimension);
    setConflict(undefined);
    setNotice(undefined);
    setEditing(true);
    focusCognitiveProcess();
  }

  function cancel(): void {
    if (pending()) return;
    const value = current();
    setDraftCognitive(value.cognitiveProcess);
    setDraftKnowledge(value.knowledgeDimension);
    setConflict(undefined);
    setNotice(undefined);
    setEditing(false);
    focusCorrectionTrigger();
  }

  async function save(): Promise<void> {
    if (pending()) return;
    const base = current();
    const request: BloomClassificationCorrectionRequest = {
      cognitiveProcess: draftCognitive(),
      knowledgeDimension: draftKnowledge(),
      expectedClassificationEditNumber: base.classificationEditNumber,
    };
    const changed =
      request.cognitiveProcess !== base.cognitiveProcess ||
      request.knowledgeDimension !== base.knowledgeDimension;
    const generation = ++operation;
    setPending(true);
    setNotice(undefined);
    try {
      const accepted = await props.save(request);
      if (disposed || generation !== operation) return;
      setCurrent(accepted);
      setDraftCognitive(accepted.cognitiveProcess);
      setDraftKnowledge(accepted.knowledgeDimension);
      setConflict(undefined);
      setEditing(false);
      props.onCurrent?.(accepted);
      props.onAccepted?.(accepted, changed);
      setNotice({
        kind: "status",
        text: changed
          ? "Bloom Classification corrected for this exact Revision."
          : "No classification change was needed; the Edit Number is unchanged.",
      });
      focusCorrectionTrigger();
    } catch (error: unknown) {
      if (disposed || generation !== operation) return;
      if (error instanceof BloomClassificationConflictError) {
        try {
          const refreshed = await props.loadCurrent();
          if (disposed || generation !== operation) return;
          setCurrent(refreshed);
          setConflict(refreshed);
          props.onCurrent?.(refreshed);
          props.onConflictCurrent?.(refreshed);
          setNotice({
            kind: "alert",
            text: "Classification changed elsewhere. Compare the current values with your retained draft, then save again or cancel.",
          });
        } catch {
          if (disposed || generation !== operation) return;
          setNotice({
            kind: "alert",
            text: "Classification changed elsewhere, but the current values could not be loaded. Your draft is retained. Reload this page before saving.",
          });
        }
      } else {
        // ASVS 16.5.1/16.5.3: retain the private draft while exposing no
        // response body, authorization reason, or persistence detail.
        setNotice({
          kind: "alert",
          text: "Bloom Classification could not be saved. Your draft is retained; try again or cancel.",
        });
      }
    } finally {
      if (!disposed && generation === operation) setPending(false);
    }
  }

  return (
    <section
      class="bloom-classification-editor"
      aria-label={props.targetName + " Bloom Classification correction"}
      aria-busy={pending()}
    >
      <h2>Bloom Classification</h2>
      <p class="bloom-classification-editor__target">
        {props.targetName} Revision {props.revisionNumber} | Classification Edit Number:{" "}
        {current().classificationEditNumber}
      </p>
      <p>
        <BloomClassificationText bloom={current()} />
      </p>
      <p class="bloom-classification-editor__help">
        Correcting this pair does not create a new content Revision. Classify the work required for
        full credit using the{" "}
        <a
          href="https://github.com/vosslab/peptidyle-learning-engine/blob/main/docs/BLOOM_TAXONOMY_GUIDE.md"
          target="_blank"
          rel="noreferrer"
        >
          Bloom Taxonomy guide
        </a>
        .
      </p>
      <Show
        when={editing()}
        fallback={
          <button
            ref={(element) => {
              correctionTrigger = element;
            }}
            type="button"
            onClick={beginEditing}
          >
            Correct Bloom Classification
          </button>
        }
      >
        <div class="bloom-classification-editor__fields">
          <label>
            Bloom Cognitive Process
            <select
              ref={(element) => {
                cognitiveProcessSelect = element;
              }}
              value={draftCognitive()}
              disabled={pending()}
              onInput={(event) =>
                setDraftCognitive(event.currentTarget.value as BloomCognitiveProcess)
              }
            >
              <For each={BLOOM_COGNITIVE_PROCESSES}>
                {(value) => <option value={value}>{value}</option>}
              </For>
            </select>
          </label>
          <label>
            Bloom Knowledge Dimension
            <select
              value={draftKnowledge()}
              disabled={pending()}
              onInput={(event) =>
                setDraftKnowledge(event.currentTarget.value as BloomKnowledgeDimension)
              }
            >
              <For each={BLOOM_KNOWLEDGE_DIMENSIONS}>
                {(value) => <option value={value}>{value}</option>}
              </For>
            </select>
          </label>
        </div>
        <Show when={conflict()}>
          {(value) => (
            <div class="bloom-classification-editor__conflict">
              <p>
                <strong>Current saved values:</strong> {value().cognitiveProcess} /{" "}
                {value().knowledgeDimension} (Edit Number {value().classificationEditNumber})
              </p>
              <p>
                <strong>Your retained draft:</strong> {draftCognitive()} / {draftKnowledge()}
              </p>
            </div>
          )}
        </Show>
        <div class="bloom-classification-editor__actions">
          <button type="button" disabled={pending()} onClick={() => void save()}>
            {pending() ? "Saving..." : conflict() ? "Save reviewed correction" : "Save correction"}
          </button>
          <button type="button" disabled={pending()} onClick={cancel}>
            Cancel
          </button>
        </div>
      </Show>
      <Show when={notice()}>
        {(value) => (
          <p role={value().kind} aria-live="polite">
            {value().text}
          </p>
        )}
      </Show>
    </section>
  );
}
