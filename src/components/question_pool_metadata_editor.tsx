// Owner and Sysadmin editor for one Pool's ordinary current metadata.

import { createSignal, For, Show, type JSX } from "solid-js";

import type { QuestionPoolId } from "../../generated/api/QuestionPoolId";
import type { QuestionPoolView } from "../../generated/api/QuestionPoolView";
import { useApplicationApi } from "../api/application_api";
import { ApiRequestError } from "../api/http_client/error";
import {
  BLOOM_COGNITIVE_PROCESSES,
  BLOOM_KNOWLEDGE_DIMENSIONS,
  isBloomCognitiveProcess,
  isBloomKnowledgeDimension,
} from "../api/decoders/bloom_classification";
import { ContentClassificationSelect } from "./content_classification_select";

function failureMessage(error: unknown): string {
  if (!(error instanceof ApiRequestError)) return "Question Pool metadata could not be saved.";
  if (error.status === 412) return "Question Pool metadata changed elsewhere. Reload the Pool.";
  if ([401, 403, 404].includes(error.status))
    return "This Question Pool is unavailable for editing.";
  if (error.status === 422) return "Check the metadata fields and try again.";
  return "Question Pool metadata could not be saved. Try again.";
}

/** Replaces editable metadata under the Pool's ordinary metadata Edit Number. */
export function QuestionPoolMetadataEditor(props: {
  readonly questionPoolId: QuestionPoolId;
  readonly pool: QuestionPoolView;
  readonly canEdit: boolean;
  readonly onSaved: () => void;
}): JSX.Element {
  const api = useApplicationApi();
  const [open, setOpen] = createSignal(false);
  const [loading, setLoading] = createSignal(false);
  const [saving, setSaving] = createSignal(false);
  const [error, setError] = createSignal("");
  const [status, setStatus] = createSignal("");
  const [metadataEditNumber, setMetadataEditNumber] = createSignal<number>();
  const [title, setTitle] = createSignal("");
  const [description, setDescription] = createSignal("");
  const [tagsText, setTagsText] = createSignal("");
  const [topicUuid, setTopicUuid] = createSignal<string | null>(null);
  const [subtopicUuid, setSubtopicUuid] = createSignal<string | null>(null);
  const [bloomCognitiveProcess, setBloomCognitiveProcess] = createSignal<
    (typeof BLOOM_COGNITIVE_PROCESSES)[number] | null
  >(null);
  const [bloomKnowledgeDimension, setBloomKnowledgeDimension] = createSignal<
    (typeof BLOOM_KNOWLEDGE_DIMENSIONS)[number] | null
  >(null);

  async function openEditor(): Promise<void> {
    if (loading() || saving()) return;
    setLoading(true);
    setError("");
    setStatus("");
    try {
      const current = await api.client.getCurrentQuestionPoolMetadata(props.questionPoolId);
      setMetadataEditNumber(current.questionPoolMetadataEditNumber);
      setTitle(current.title);
      setDescription(current.description);
      setTagsText(current.tags.join("\n"));
      setTopicUuid(current.topicUuid);
      setSubtopicUuid(current.subtopicUuid);
      setBloomCognitiveProcess(current.bloomCognitiveProcess);
      setBloomKnowledgeDimension(current.bloomKnowledgeDimension);
      setOpen(true);
    } catch (caught) {
      setError(failureMessage(caught));
    } finally {
      setLoading(false);
    }
  }

  async function save(): Promise<void> {
    const expectedMetadataEditNumber = metadataEditNumber();
    if (expectedMetadataEditNumber === undefined || saving()) return;
    setSaving(true);
    setError("");
    setStatus("");
    try {
      const saved = await api.client.saveQuestionPoolMetadata({
        questionPoolId: props.questionPoolId,
        expectedMetadataEditNumber,
        metadata: {
          title: title(),
          description: description(),
          topicUuid: topicUuid(),
          subtopicUuid: subtopicUuid(),
          tags: tagsText() === "" ? [] : tagsText().split("\n"),
          bloomCognitiveProcess: bloomCognitiveProcess(),
          bloomKnowledgeDimension: bloomKnowledgeDimension(),
        },
      });
      if (saved.questionPoolId !== props.questionPoolId) {
        throw new Error("Metadata receipt did not match this Question Pool.");
      }
      setMetadataEditNumber(saved.questionPoolMetadataEditNumber);
      setStatus("Question Pool metadata saved.");
      setOpen(false);
      props.onSaved();
    } catch (caught) {
      setError(failureMessage(caught));
    } finally {
      setSaving(false);
    }
  }

  return (
    <Show when={props.canEdit}>
      <section class="question-metadata-editor" aria-label="Question Pool metadata">
        <Show
          when={open()}
          fallback={
            <button type="button" disabled={loading()} onClick={() => void openEditor()}>
              {loading() ? "Loading metadata..." : "Edit Question Pool metadata"}
            </button>
          }
        >
          <h2>Edit Question Pool metadata</h2>
          <label>
            Title
            <input
              value={title()}
              maxLength={512}
              onInput={(event) => setTitle(event.currentTarget.value)}
            />
          </label>
          <label>
            Description
            <textarea
              rows={4}
              value={description()}
              maxLength={4000}
              onInput={(event) => setDescription(event.currentTarget.value)}
            />
          </label>
          <label>
            Tags, one per line
            <textarea
              rows={4}
              value={tagsText()}
              onInput={(event) => setTagsText(event.currentTarget.value)}
            />
          </label>
          <ContentClassificationSelect
            label="Topic"
            value={topicUuid()}
            parentUuid={props.pool.metadata.subjectUuid}
            load={(parent) => api.client.listTopics(parent)}
            onChange={(value) => {
              setTopicUuid(value);
              setSubtopicUuid(null);
            }}
          />
          <ContentClassificationSelect
            label="Subtopic"
            value={subtopicUuid()}
            parentUuid={topicUuid()}
            load={(parent) => api.client.listSubtopics(parent)}
            onChange={setSubtopicUuid}
          />
          <label>
            Bloom Cognitive Process
            <select
              value={bloomCognitiveProcess() ?? ""}
              onChange={(event) => {
                const value = event.currentTarget.value;
                setBloomCognitiveProcess(
                  value === "" ? null : isBloomCognitiveProcess(value) ? value : null,
                );
              }}
            >
              <option value="">Not assigned</option>
              <For each={BLOOM_COGNITIVE_PROCESSES}>{(value) => <option>{value}</option>}</For>
            </select>
          </label>
          <label>
            Bloom Knowledge Dimension
            <select
              value={bloomKnowledgeDimension() ?? ""}
              onChange={(event) => {
                const value = event.currentTarget.value;
                setBloomKnowledgeDimension(
                  value === "" ? null : isBloomKnowledgeDimension(value) ? value : null,
                );
              }}
            >
              <option value="">Not assigned</option>
              <For each={BLOOM_KNOWLEDGE_DIMENSIONS}>{(value) => <option>{value}</option>}</For>
            </select>
          </label>
          <div class="question-metadata-editor__actions">
            <button type="button" disabled={saving()} onClick={() => void save()}>
              {saving() ? "Saving..." : "Save metadata"}
            </button>
            <button type="button" disabled={saving()} onClick={() => setOpen(false)}>
              Cancel
            </button>
          </div>
        </Show>
        <Show when={status()}>{(message) => <p role="status">{message()}</p>}</Show>
        <Show when={error()}>{(message) => <p role="alert">{message()}</p>}</Show>
      </section>
    </Show>
  );
}
