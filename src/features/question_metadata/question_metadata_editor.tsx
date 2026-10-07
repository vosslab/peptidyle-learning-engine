// Owner/Sysadmin single Published Question metadata editor.

import { createResource, createSignal, For, Show, type JSX } from "solid-js";

import type { QuestionDetails } from "../../../generated/api/QuestionDetails";
import type { QuestionType } from "../../../generated/api/QuestionType";
import { useApplicationApi } from "../../api/application_api";
import { ApiRequestError } from "../../api/http_client/error";
import {
  BLOOM_COGNITIVE_PROCESSES,
  BLOOM_KNOWLEDGE_DIMENSIONS,
} from "../../api/decoders/bloom_classification";
import { ContentClassificationSelect } from "../../components/content_classification_select";

function sameRevision(
  left: QuestionDetails["summary"]["publishedQuestionRevisionTuple"],
  right: QuestionDetails["summary"]["publishedQuestionRevisionTuple"],
): boolean {
  return (
    left.publishedQuestionId === right.publishedQuestionId &&
    left.revisionNumber === right.revisionNumber
  );
}

function failureMessage(error: unknown): string {
  if (!(error instanceof ApiRequestError)) return "Metadata could not be saved. Try again.";
  if (error.status === 412)
    return "Question metadata changed elsewhere. Reload the current Question.";
  if (error.status === 409) return "This Question is no longer available to edit.";
  if ([401, 403, 404].includes(error.status)) return "This Question is unavailable for editing.";
  if (error.status === 422) return "Check the metadata fields and try again.";
  return "Metadata could not be saved. Try again.";
}

const QUESTION_TYPES = [
  "multipleChoice",
  "multipleAnswer",
  "fillInBlank",
  "multipleFillInBlank",
  "numeric",
  "matching",
  "ordering",
  "hotspot",
] as const satisfies ReadonlyArray<QuestionType>;

/** Edits bounded metadata on the current exact Question Revision. */
export function QuestionMetadataEditor(props: {
  readonly detail: QuestionDetails;
  readonly onSaved: () => void;
}): JSX.Element {
  const api = useApplicationApi();
  const questionId = (): string => props.detail.summary.questionId;
  const tuple = (): QuestionDetails["summary"]["publishedQuestionRevisionTuple"] =>
    props.detail.summary.publishedQuestionRevisionTuple;
  const [lineage] = createResource(questionId, (id) => api.client.getQuestionLineage(id));
  const [open, setOpen] = createSignal(false);
  const [loading, setLoading] = createSignal(false);
  const [saving, setSaving] = createSignal(false);
  const [error, setError] = createSignal("");
  const [status, setStatus] = createSignal("");
  const [metadataEditNumber, setMetadataEditNumber] = createSignal<number>();
  const [title, setTitle] = createSignal("");
  const [description, setDescription] = createSignal("");
  const [questionType, setQuestionType] = createSignal<QuestionType>(
    props.detail.summary.questionType,
  );
  const [tagsText, setTagsText] = createSignal("");
  const [disciplineUuid, setDisciplineUuid] = createSignal<string | null>(null);
  const [subjectUuid, setSubjectUuid] = createSignal<string | null>(null);
  const [topicUuid, setTopicUuid] = createSignal<string | null>(null);
  const [subtopicUuid, setSubtopicUuid] = createSignal<string | null>(null);
  const [bloomCognitiveProcess, setBloomCognitiveProcess] = createSignal<
    (typeof BLOOM_COGNITIVE_PROCESSES)[number] | null
  >(null);
  const [bloomKnowledgeDimension, setBloomKnowledgeDimension] = createSignal<
    (typeof BLOOM_KNOWLEDGE_DIMENSIONS)[number] | null
  >(null);
  const canEdit = (): boolean => {
    const currentLineage = lineage();
    return (
      currentLineage?.viewerMayEditMetadata === true &&
      sameRevision(currentLineage.summary.publishedQuestionRevisionTuple, tuple())
    );
  };

  async function openEditor(): Promise<void> {
    if (loading() || saving()) return;
    setLoading(true);
    setError("");
    setStatus("");
    try {
      const currentLineage = await api.client.getQuestionLineage(questionId());
      if (
        !currentLineage.viewerMayEditMetadata ||
        !sameRevision(currentLineage.summary.publishedQuestionRevisionTuple, tuple())
      ) {
        setError("Only the current Question Revision can be edited by its owner or a Sysadmin.");
        return;
      }
      const [current] = await api.client.getCurrentQuestionSharedMetadata([questionId()]);
      if (current === undefined) throw new Error("Current metadata is unavailable.");
      setMetadataEditNumber(current.metadataEditNumber);
      setTitle(current.questionTitle);
      setDescription(current.questionDescription);
      setQuestionType(current.questionType);
      setTagsText(current.tags.join("\n"));
      setDisciplineUuid(current.disciplineUuid);
      setSubjectUuid(current.subjectUuid);
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
    const tags = tagsText() === "" ? [] : tagsText().split("\n");
    setSaving(true);
    setError("");
    setStatus("");
    try {
      const saved = await api.client.saveQuestionMetadata({
        publishedQuestionRevisionTuple: tuple(),
        expectedMetadataEditNumber,
        metadata: {
          questionTitle: title(),
          questionDescription: description(),
          questionType: questionType(),
          tags,
          disciplineUuid: disciplineUuid() ?? "",
          subjectUuid: subjectUuid() ?? "",
          topicUuid: topicUuid(),
          subtopicUuid: subtopicUuid(),
          bloomCognitiveProcess: bloomCognitiveProcess(),
          bloomKnowledgeDimension: bloomKnowledgeDimension(),
        },
      });
      if (!sameRevision(saved.publishedQuestionRevisionTuple, tuple())) {
        throw new Error("Metadata receipt did not match this Question Revision.");
      }
      setMetadataEditNumber(saved.metadataEditNumber);
      setStatus("Question metadata saved.");
      setOpen(false);
      props.onSaved();
    } catch (caught) {
      setError(failureMessage(caught));
    } finally {
      setSaving(false);
    }
  }

  return (
    <Show when={canEdit()}>
      <section class="question-metadata-editor" aria-label="Question metadata">
        <Show
          when={open()}
          fallback={
            <button type="button" disabled={loading()} onClick={() => void openEditor()}>
              {loading() ? "Loading metadata..." : "Edit Question metadata"}
            </button>
          }
        >
          <h2>Edit Question metadata</h2>
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
          <Show
            when={props.detail.summary.backend === "webwork"}
            fallback={<p>Question Type: {props.detail.summary.questionType}</p>}
          >
            <label>
              Question Type
              <select
                value={questionType()}
                onChange={(event) => setQuestionType(event.currentTarget.value as QuestionType)}
              >
                <For each={QUESTION_TYPES}>{(value) => <option value={value}>{value}</option>}</For>
              </select>
            </label>
          </Show>
          <label>
            Tags, one per line
            <textarea
              rows={4}
              value={tagsText()}
              onInput={(event) => setTagsText(event.currentTarget.value)}
            />
          </label>
          <ContentClassificationSelect
            label="Discipline"
            required
            value={disciplineUuid()}
            load={() => api.client.listDisciplinesIncludingRetired()}
            onChange={(value) => {
              setDisciplineUuid(value);
              setSubjectUuid(null);
              setTopicUuid(null);
              setSubtopicUuid(null);
            }}
          />
          <ContentClassificationSelect
            label="Subject"
            required
            value={subjectUuid()}
            parentUuid={disciplineUuid()}
            load={(parent) => api.client.listSubjects(parent)}
            onChange={(value) => {
              setSubjectUuid(value);
              setTopicUuid(null);
              setSubtopicUuid(null);
            }}
          />
          <ContentClassificationSelect
            label="Topic"
            value={topicUuid()}
            parentUuid={subjectUuid()}
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
              onChange={(event) =>
                setBloomCognitiveProcess(
                  (BLOOM_COGNITIVE_PROCESSES as ReadonlyArray<string>).includes(
                    event.currentTarget.value,
                  )
                    ? (event.currentTarget.value as (typeof BLOOM_COGNITIVE_PROCESSES)[number])
                    : null,
                )
              }
            >
              <option value="">Not assigned</option>
              <For each={BLOOM_COGNITIVE_PROCESSES}>{(value) => <option>{value}</option>}</For>
            </select>
          </label>
          <label>
            Bloom Knowledge Dimension
            <select
              value={bloomKnowledgeDimension() ?? ""}
              onChange={(event) =>
                setBloomKnowledgeDimension(
                  (BLOOM_KNOWLEDGE_DIMENSIONS as ReadonlyArray<string>).includes(
                    event.currentTarget.value,
                  )
                    ? (event.currentTarget.value as (typeof BLOOM_KNOWLEDGE_DIMENSIONS)[number])
                    : null,
                )
              }
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
