// Focused shared-metadata editor for one bounded Published Question selection.

import { For, Show, createSignal, type JSX } from "solid-js";

import type { PublishedQuestionSharedMetadata } from "../../generated/api/PublishedQuestionSharedMetadata";
import { ApiRequestError } from "../api/http_client/error";
import type {
  QuestionBulkMetadataClient,
  QuestionBulkMetadataPatch,
  QuestionBulkMetadataUpdateResult,
} from "../api/question_bulk_metadata";
import "./question_bulk_metadata_editor.css";

type EditMode = "keep" | "replace" | "clear";
type EditorState = "ready" | "saving" | "refreshing" | "refresh-error";

const MAX_TAGS = 64;
const MAX_TEXT_CODE_POINTS = 120;
function hasControlCharacter(value: string): boolean {
  return [...value].some((character) => {
    const codePoint = character.codePointAt(0) ?? 0;
    return codePoint <= 0x1f || (codePoint >= 0x7f && codePoint <= 0x9f);
  });
}

export interface QuestionBulkMetadataEditorProps {
  readonly client: QuestionBulkMetadataClient;
  readonly initialMetadata: ReadonlyArray<PublishedQuestionSharedMetadata>;
  readonly onBusyChange: (busy: boolean) => void;
  readonly onCancel: () => void;
  readonly onSuccess: (results: ReadonlyArray<QuestionBulkMetadataUpdateResult>) => void;
}

function fieldSummary(
  metadata: ReadonlyArray<PublishedQuestionSharedMetadata>,
  field: "tags" | "subject" | "topic",
): string {
  const values = metadata.map((item) =>
    field === "tags" ? JSON.stringify(item.tags) : JSON.stringify(item[field]),
  );
  if (new Set(values).size > 1) return `Mixed across ${metadata.length} Questions`;
  const first = metadata[0];
  if (first === undefined) return "None";
  if (field === "tags") return first.tags.length === 0 ? "No tags" : first.tags.join(", ");
  return first[field] ?? "Not set";
}

function validatedText(value: string, label: string): string {
  if (
    value.length === 0 ||
    value.trim() !== value ||
    [...value].length > MAX_TEXT_CODE_POINTS ||
    hasControlCharacter(value)
  ) {
    throw new Error(
      `${label} must be trimmed, nonempty, control-free text of at most ${MAX_TEXT_CODE_POINTS} characters.`,
    );
  }
  return value;
}

function validatedTags(value: string): ReadonlyArray<string> {
  const tags = value
    .split("\n")
    .map((tag) => tag.trim())
    .filter((tag) => tag.length > 0)
    .map((tag) => validatedText(tag, "Each tag"));
  if (tags.length === 0) throw new Error("Enter at least one replacement tag, or choose Clear.");
  if (tags.length > MAX_TAGS) throw new Error(`Enter at most ${MAX_TAGS} tags.`);
  if (new Set(tags).size !== tags.length) throw new Error("Enter each tag only once.");
  return tags;
}

function replacementPatch(
  tagsMode: EditMode,
  tagsText: string,
  subjectMode: EditMode,
  subjectText: string,
  topicMode: EditMode,
  topicText: string,
): QuestionBulkMetadataPatch {
  if (tagsMode === "keep" && subjectMode === "keep" && topicMode === "keep") {
    throw new Error("Choose Replace or Clear for at least one field.");
  }
  const patch: {
    tags?: ReadonlyArray<string>;
    subject?: string | null;
    topic?: string | null;
  } = {};
  if (tagsMode === "replace") patch.tags = validatedTags(tagsText);
  if (tagsMode === "clear") patch.tags = [];
  if (subjectMode === "replace") patch.subject = validatedText(subjectText, "Subject");
  if (subjectMode === "clear") patch.subject = null;
  if (topicMode === "replace") patch.topic = validatedText(topicText, "Topic");
  if (topicMode === "clear") patch.topic = null;
  return patch;
}

function MetadataMode(props: {
  readonly field: string;
  readonly label: string;
  readonly mode: EditMode;
  readonly disabled: boolean;
  readonly onChange: (mode: EditMode) => void;
  readonly children: JSX.Element;
}): JSX.Element {
  return (
    <fieldset class="bulk-metadata-field" disabled={props.disabled}>
      <legend>{props.label}</legend>
      <div class="bulk-metadata-modes">
        <For each={["keep", "replace", "clear"] as const}>
          {(mode) => (
            <label>
              <input
                type="radio"
                name={`bulk-metadata-${props.field}`}
                value={mode}
                checked={props.mode === mode}
                onChange={() => props.onChange(mode)}
              />
              {mode[0]?.toUpperCase()}
              {mode.slice(1)}
            </label>
          )}
        </For>
      </div>
      <Show when={props.mode === "replace"}>{props.children}</Show>
    </fieldset>
  );
}

/** Edits only tags, subject, and topic with explicit per-field intent. */
export function QuestionBulkMetadataEditor(props: QuestionBulkMetadataEditorProps): JSX.Element {
  const [metadata, setMetadata] = createSignal(props.initialMetadata);
  const [state, setState] = createSignal<EditorState>("ready");
  const [tagsMode, setTagsMode] = createSignal<EditMode>("keep");
  const [subjectMode, setSubjectMode] = createSignal<EditMode>("keep");
  const [topicMode, setTopicMode] = createSignal<EditMode>("keep");
  const [tagsText, setTagsText] = createSignal("");
  const [subjectText, setSubjectText] = createSignal("");
  const [topicText, setTopicText] = createSignal("");
  const [message, setMessage] = createSignal<string | null>(null);
  const busy = (): boolean => state() === "saving" || state() === "refreshing";
  const questionIds = (): ReadonlyArray<PublishedQuestionSharedMetadata["questionId"]> =>
    metadata().map((item) => item.questionId);

  async function refresh(reason: string): Promise<void> {
    setState("refreshing");
    props.onBusyChange(true);
    setMessage(`${reason} Refreshing current metadata before another submission.`);
    try {
      const current = await props.client.getCurrentQuestionBulkMetadata(questionIds());
      setMetadata(current);
      setState("ready");
      setMessage("Current values were refreshed. Review them before submitting again.");
    } catch {
      setState("refresh-error");
      setMessage(
        "Current metadata could not be refreshed. Retry when the connection is available.",
      );
    } finally {
      props.onBusyChange(false);
    }
  }

  async function submit(): Promise<void> {
    let patch: QuestionBulkMetadataPatch;
    try {
      patch = replacementPatch(
        tagsMode(),
        tagsText(),
        subjectMode(),
        subjectText(),
        topicMode(),
        topicText(),
      );
    } catch (error: unknown) {
      setMessage(error instanceof Error ? error.message : "Review the replacement values.");
      return;
    }
    setState("saving");
    props.onBusyChange(true);
    setMessage("Updating all selected Questions...");
    try {
      const results = await props.client.updateQuestionBulkMetadata({
        selection: metadata().map((item) => ({
          questionId: item.questionId,
          metadataEditNumber: item.metadataEditNumber,
        })),
        patch,
      });
      props.onSuccess(results);
    } catch (error: unknown) {
      if (error instanceof ApiRequestError && error.status === 422) {
        setState("ready");
        setMessage("The server rejected these values. Review them and submit again.");
      } else if (
        error instanceof ApiRequestError &&
        (error.status === 401 || error.status === 403 || error.status === 404)
      ) {
        setState("ready");
        setMessage(
          "This bulk metadata action is not available. Your entered values are preserved.",
        );
      } else {
        const reason =
          error instanceof ApiRequestError && error.status === 412
            ? "The metadata changed before the update was accepted."
            : "The update outcome could not be confirmed.";
        await refresh(reason);
      }
    } finally {
      props.onBusyChange(false);
    }
  }

  return (
    <section class="bulk-metadata-editor" aria-labelledby="bulk-metadata-title">
      <div class="bulk-metadata-heading">
        <div>
          <p class="eyebrow">Shared search metadata</p>
          <h2 id="bulk-metadata-title">Edit {metadata().length} selected Questions</h2>
        </div>
        <button type="button" class="quiet-action" disabled={busy()} onClick={props.onCancel}>
          Close editor
        </button>
      </div>
      <p>
        Choose Keep, Replace, or Clear for each field. Keeping a mixed field preserves every
        Question's current value.
      </p>
      <dl class="bulk-metadata-summary">
        <div>
          <dt>Tags</dt>
          <dd>{fieldSummary(metadata(), "tags")}</dd>
        </div>
        <div>
          <dt>Subject</dt>
          <dd>{fieldSummary(metadata(), "subject")}</dd>
        </div>
        <div>
          <dt>Topic</dt>
          <dd>{fieldSummary(metadata(), "topic")}</dd>
        </div>
      </dl>
      <details>
        <summary>Review current values for every selected Question</summary>
        <div class="bulk-metadata-current-values">
          <For each={metadata()}>
            {(item) => (
              <article>
                <h3>{item.questionId}</h3>
                <p>Tags: {item.tags.length === 0 ? "No tags" : item.tags.join(", ")}</p>
                <p>Subject: {item.subject ?? "Not set"}</p>
                <p>Topic: {item.topic ?? "Not set"}</p>
              </article>
            )}
          </For>
        </div>
      </details>
      <div class="bulk-metadata-fields">
        <MetadataMode
          field="tags"
          label="Tags"
          mode={tagsMode()}
          disabled={busy() || state() === "refresh-error"}
          onChange={setTagsMode}
        >
          <label>
            Replacement tags, one per line
            <textarea
              rows={5}
              value={tagsText()}
              onInput={(event) => setTagsText(event.currentTarget.value)}
              aria-describedby="bulk-metadata-tags-help"
            />
          </label>
          <p id="bulk-metadata-tags-help">Commas remain part of a tag.</p>
        </MetadataMode>
        <MetadataMode
          field="subject"
          label="Subject"
          mode={subjectMode()}
          disabled={busy() || state() === "refresh-error"}
          onChange={setSubjectMode}
        >
          <label>
            Replacement subject
            <input
              value={subjectText()}
              onInput={(event) => setSubjectText(event.currentTarget.value)}
            />
          </label>
        </MetadataMode>
        <MetadataMode
          field="topic"
          label="Topic"
          mode={topicMode()}
          disabled={busy() || state() === "refresh-error"}
          onChange={setTopicMode}
        >
          <label>
            Replacement topic
            <input
              value={topicText()}
              onInput={(event) => setTopicText(event.currentTarget.value)}
            />
          </label>
        </MetadataMode>
      </div>
      <Show when={message()}>{(text) => <p role="status">{text()}</p>}</Show>
      <div class="bulk-metadata-actions">
        <Show when={state() === "refresh-error"}>
          <button type="button" class="primary-action" onClick={() => void refresh("Read retry.")}>
            Retry current metadata
          </button>
        </Show>
        <Show when={state() !== "refresh-error"}>
          <button
            type="button"
            class="primary-action"
            disabled={busy()}
            onClick={() => void submit()}
          >
            Update all {metadata().length} Questions
          </button>
        </Show>
      </div>
    </section>
  );
}
