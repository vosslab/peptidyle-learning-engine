// Focused shared-metadata editor for one bounded Published Question selection.

import { For, Show, batch, createResource, createSignal, type JSX } from "solid-js";

import type { PublishedQuestionSharedMetadata } from "../../generated/api/PublishedQuestionSharedMetadata";
import { ApiRequestError } from "../api/http_client/error";
import type { ContentClassificationClient } from "../api/content_classification";
import { decodeUuid } from "../api/decoder";
import {
  AuthoringClassificationLevel,
  type VocabularyCreation,
} from "./authoring_classification_level";
import { ContentClassificationSelect } from "./content_classification_select";
import type { RecordContent } from "./data_sheet/record_content";
import type { RecordField } from "./data_sheet/data_sheet_fields";
import { SearchBlock } from "./search_block/search_block";
import { SearchControls } from "./search_block/search_controls";
import { createLocalCompleteSearch } from "./search_block/local_complete_search_state";
import { UnsavedChangesGuard, trackEditorSave } from "./unsaved_changes_guard";
import type {
  QuestionBulkMetadataClient,
  QuestionBulkMetadataPatch,
  QuestionBulkMetadataUpdateResult,
} from "../api/question_bulk_metadata";
import "./question_bulk_metadata_editor.css";

type EditMode = "keep" | "replace" | "clear";
type EditorState = "ready" | "saving" | "refreshing" | "refresh-error";
const CLASSIFICATION_FIELDS = [
  "disciplineUuid",
  "subjectUuid",
  "topicUuid",
  "subtopicUuid",
] as const;
type ClassificationField = (typeof CLASSIFICATION_FIELDS)[number];
type MetadataSortKey = "title" | "questionId" | "tags" | "metadataEditNumber" | ClassificationField;
type MetadataFilterKey = "tags" | ClassificationField;
const CLASSIFICATION_LABELS: Readonly<Record<ClassificationField, string>> = {
  disciplineUuid: "Discipline",
  subjectUuid: "Subject",
  topicUuid: "Topic",
  subtopicUuid: "Subtopic",
};

const MAX_TEXT_CODE_POINTS = 120;
function hasControlCharacter(value: string): boolean {
  return [...value].some((character) => {
    const codePoint = character.codePointAt(0) ?? 0;
    return codePoint <= 0x1f || (codePoint >= 0x7f && codePoint <= 0x9f);
  });
}

export interface QuestionBulkMetadataEditorProps {
  readonly client: QuestionBulkMetadataClient;
  readonly classificationClient: ContentClassificationClient;
  readonly initialMetadata: ReadonlyArray<PublishedQuestionSharedMetadata>;
  readonly questionTitles: ReadonlyMap<string, string>;
  readonly guardScope: symbol;
  readonly onBusyChange: (busy: boolean) => void;
  readonly onCancel: () => void;
  readonly onSuccess: (results: ReadonlyArray<QuestionBulkMetadataUpdateResult>) => void;
}

function fieldSummary(
  metadata: ReadonlyArray<PublishedQuestionSharedMetadata>,
  field: "tags" | ClassificationField,
  names: ReadonlyMap<string, string>,
): string {
  const values = metadata.map((item) =>
    field === "tags" ? JSON.stringify(item.tags) : JSON.stringify(item[field]),
  );
  if (new Set(values).size > 1) return `Mixed across ${metadata.length} Questions`;
  const first = metadata[0];
  if (first === undefined) return "None";
  if (field === "tags") return first.tags.length === 0 ? "No tags" : first.tags.join(", ");
  const uuid = first[field];
  return uuid === null ? "Not set" : (names.get(uuid) ?? "Name unavailable");
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
  if (new Set(tags).size !== tags.length) throw new Error("Enter each tag only once.");
  return tags;
}

function replacementPatch(
  tagsMode: EditMode,
  tagsText: string,
  modes: Readonly<Record<ClassificationField, EditMode>>,
  selections: Readonly<Record<ClassificationField, string | null>>,
): QuestionBulkMetadataPatch {
  if (tagsMode === "keep" && CLASSIFICATION_FIELDS.every((field) => modes[field] === "keep")) {
    throw new Error("Choose Replace or Clear for at least one field.");
  }
  const patch: {
    tags?: ReadonlyArray<string>;
    disciplineUuid?: string;
    subjectUuid?: string;
    topicUuid?: string | null;
    subtopicUuid?: string | null;
  } = {};
  if (tagsMode === "replace") patch.tags = validatedTags(tagsText);
  if (tagsMode === "clear") patch.tags = [];
  for (const field of CLASSIFICATION_FIELDS) {
    if (modes[field] === "replace") {
      if (selections[field] === null)
        throw new Error(`Select a replacement ${CLASSIFICATION_LABELS[field]}.`);
      patch[field] = decodeUuid(selections[field], `patch.${field}`);
    }
  }
  for (const field of ["topicUuid", "subtopicUuid"] as const) {
    if (modes[field] === "clear") patch[field] = null;
  }
  return patch;
}

function MetadataMode(props: {
  readonly field: string;
  readonly label: string;
  readonly mode: EditMode;
  readonly disabled: boolean;
  readonly required?: boolean;
  readonly onChange: (mode: EditMode) => void;
  readonly children: JSX.Element;
}): JSX.Element {
  return (
    <fieldset class="bulk-metadata-field" disabled={props.disabled}>
      <legend>{props.label}</legend>
      <div class="bulk-metadata-modes">
        <For
          each={
            props.required
              ? (["keep", "replace"] as const)
              : (["keep", "replace", "clear"] as const)
          }
        >
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

/** Edits tags and classification identities with explicit per-field intent. */
export function QuestionBulkMetadataEditor(props: QuestionBulkMetadataEditorProps): JSX.Element {
  const [metadata, setMetadata] = createSignal(props.initialMetadata);
  const [state, setState] = createSignal<EditorState>("ready");
  const [tagsMode, setTagsMode] = createSignal<EditMode>("keep");
  const [modes, setModes] = createSignal<Record<ClassificationField, EditMode>>({
    disciplineUuid: "keep",
    subjectUuid: "keep",
    topicUuid: "keep",
    subtopicUuid: "keep",
  });
  const [selections, setSelections] = createSignal<Record<ClassificationField, string | null>>({
    disciplineUuid: null,
    subjectUuid: null,
    topicUuid: null,
    subtopicUuid: null,
  });
  const [tagsText, setTagsText] = createSignal("");
  const [message, setMessage] = createSignal<string | null>(null);
  const [closeRequested, setCloseRequested] = createSignal(false);
  const [saved, setSaved] = createSignal(false);
  const busy = (): boolean => state() === "saving" || state() === "refreshing";
  const dirty = (): boolean => {
    if (saved()) return false;
    if (tagsMode() === "replace") {
      const tags = tagsText()
        .split("\n")
        .map((tag) => tag.trim())
        .filter(Boolean);
      if (
        tags.length === 0 ||
        metadata().some((item) => JSON.stringify(item.tags) !== JSON.stringify(tags))
      )
        return true;
    } else if (tagsMode() === "clear" && metadata().some((item) => item.tags.length > 0)) {
      return true;
    }
    return CLASSIFICATION_FIELDS.some((field) => {
      const mode = modes()[field];
      if (mode === "keep") return false;
      if (mode === "replace") {
        const selected = selections()[field];
        return selected === null || metadata().some((item) => item[field] !== selected);
      }
      return metadata().some((item) => item[field] !== null);
    });
  };
  const questionIds = (): ReadonlyArray<PublishedQuestionSharedMetadata["questionId"]> =>
    metadata().map((item) => item.questionId);

  const [names] = createResource(metadata, async (items) => {
    const client = props.classificationClient;
    const vocabularies = await Promise.all([
      client.listDisciplinesIncludingRetired(),
      ...[...new Set(items.map((item) => item.disciplineUuid))].map((uuid) =>
        client.listSubjects(uuid),
      ),
      ...[...new Set(items.map((item) => item.subjectUuid))].map((uuid) => client.listTopics(uuid)),
      ...[
        ...new Set(items.flatMap((item) => (item.topicUuid === null ? [] : [item.topicUuid]))),
      ].map((uuid) => client.listSubtopics(uuid)),
    ]);
    return new Map(vocabularies.flat().map((item) => [item.uuid, item.name]));
  });
  const currentNames = (): ReadonlyMap<string, string> =>
    names.error ? new Map() : (names() ?? new Map());

  const questionTitle = (item: PublishedQuestionSharedMetadata): string =>
    props.questionTitles.get(item.questionId) ?? "Question";
  const classificationName = (
    item: PublishedQuestionSharedMetadata,
    field: ClassificationField,
  ): string => {
    const uuid = item[field];
    return uuid === null ? "Not set" : (currentNames().get(uuid) ?? "Name unavailable");
  };
  const metadataFields: ReadonlyArray<
    RecordField<PublishedQuestionSharedMetadata, MetadataSortKey, MetadataFilterKey>
  > = [
    {
      id: "title",
      label: "Question",
      role: "identity",
      sortKey: "title",
      value: questionTitle,
      defaultColumn: { priority: 0 },
    },
    {
      id: "questionId",
      label: "Question ID",
      sortKey: "questionId",
      value: (item) => item.questionId,
      defaultColumn: { priority: 1 },
    },
    {
      id: "tags",
      label: "Tags",
      sortKey: "tags",
      filterKey: "tags",
      value: (item) => item.tags,
      defaultColumn: { priority: 2 },
    },
    ...CLASSIFICATION_FIELDS.map((field, index) => ({
      id: field,
      label: CLASSIFICATION_LABELS[field],
      sortKey: field,
      filterKey: field,
      value: (item: PublishedQuestionSharedMetadata): string => classificationName(item, field),
      defaultColumn: { priority: index + 3 },
    })),
    {
      id: "metadataEditNumber",
      label: "Metadata Edit Number",
      sortKey: "metadataEditNumber",
      value: (item) => item.metadataEditNumber,
      defaultColumn: { priority: 7 },
    },
  ];
  const metadataContent = (item: PublishedQuestionSharedMetadata): RecordContent => ({
    title: questionTitle(item),
    // ASVS 1.2.1: shared record facts render metadata as escaped text.
    details: [
      { kind: "questionId", questionTitle: questionTitle(item), displayId: item.questionId },
      {
        kind: "text",
        label: "Tags",
        value: item.tags.length === 0 ? "No tags" : item.tags.join(", "),
      },
      ...CLASSIFICATION_FIELDS.map((field) => ({
        kind: "text" as const,
        label: CLASSIFICATION_LABELS[field],
        value: classificationName(item, field),
      })),
      { kind: "text", label: "Metadata Edit Number", value: String(item.metadataEditNumber) },
    ],
    actions: [],
  });
  // Search projects the complete metadata collection; saves keep the full target set above.
  const reviewSource = (): ReadonlyArray<PublishedQuestionSharedMetadata> => {
    currentNames();
    void props.questionTitles;
    return metadata();
  };
  const metadataSearch = createLocalCompleteSearch(
    { rows: reviewSource },
    {
      fields: metadataFields,
      rowId: (item) => item.questionId,
      keyword: (item) =>
        [
          questionTitle(item),
          item.questionId,
          ...item.tags,
          ...CLASSIFICATION_FIELDS.map((field) => classificationName(item, field)),
        ].join(" "),
      content: metadataContent,
    },
  );

  function effectiveParent(field: ClassificationField): string | null {
    if (modes()[field] === "replace") return selections()[field];
    if (modes()[field] === "clear") return null;
    const values = new Set(metadata().map((item) => item[field]));
    return values.size === 1 ? (metadata()[0]?.[field] ?? null) : null;
  }

  async function createLevel(
    field: ClassificationField,
    name: string,
    parentUuid: string,
  ): Promise<VocabularyCreation> {
    if (field === "topicUuid") {
      const item = await props.classificationClient.createTopic(name, parentUuid);
      return { uuid: item.uuid, name: item.name, needsAcceptance: false };
    }
    if (field === "subtopicUuid") {
      const item = await props.classificationClient.createSubtopic(name, parentUuid);
      return { uuid: item.uuid, name: item.name, needsAcceptance: false };
    }
    return props.classificationClient.createSubject(name, parentUuid);
  }

  function changeSelection(field: ClassificationField, uuid: string | null): void {
    const index = CLASSIFICATION_FIELDS.indexOf(field);
    const next = { ...selections(), [field]: uuid };
    // Clear only replacement selection state, never change Keep/Clear intent or omitted patches.
    for (const child of CLASSIFICATION_FIELDS.slice(index + 1)) next[child] = null;
    setSelections(next);
  }

  function changeMode(field: ClassificationField, mode: EditMode): void {
    batch(() => {
      setModes({ ...modes(), [field]: mode });
      changeSelection(field, null);
    });
  }

  async function refresh(reason: string): Promise<void> {
    setState("refreshing");
    props.onBusyChange(true);
    setMessage(`${reason} Refreshing current metadata before another submission.`);
    try {
      const current = await props.client.getCurrentQuestionSharedMetadata(questionIds());
      const previousParents = CLASSIFICATION_FIELDS.map(effectiveParent);
      batch(() => {
        setMetadata(current);
        // ASVS 2.3.1: refresh invalidates dependent choices before another submission.
        for (let index = 1; index < CLASSIFICATION_FIELDS.length; index += 1) {
          const field = CLASSIFICATION_FIELDS[index];
          const parent = CLASSIFICATION_FIELDS[index - 1];
          if (
            field !== undefined &&
            parent !== undefined &&
            modes()[field] === "replace" &&
            effectiveParent(parent) !== previousParents[index - 1]
          ) {
            setSelections({ ...selections(), [field]: null });
          }
        }
      });
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

  async function submit(): Promise<boolean> {
    let patch: QuestionBulkMetadataPatch;
    try {
      patch = replacementPatch(tagsMode(), tagsText(), modes(), selections());
    } catch (error: unknown) {
      setMessage(error instanceof Error ? error.message : "Review the replacement values.");
      return false;
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
      setSaved(true);
      props.onSuccess(results);
      return true;
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
      return false;
    } finally {
      props.onBusyChange(false);
    }
  }
  const trackedSave = trackEditorSave(submit);

  return (
    <section class="bulk-metadata-editor" aria-labelledby="bulk-metadata-title">
      <UnsavedChangesGuard
        scope={props.guardScope}
        dirty={dirty}
        save={trackedSave.save}
        inFlightSave={trackedSave.inFlightSave}
        manualLeave={{
          requested: closeRequested,
          clearRequest: () => setCloseRequested(false),
          continue: props.onCancel,
        }}
        copy={{
          heading: "Save Question metadata changes?",
          description: "Your selected Question metadata changes have not been saved.",
          saveFailureMessage: "Question metadata was not saved. Review the fields and try again.",
        }}
      />
      <div class="bulk-metadata-heading">
        <div>
          <h2 id="bulk-metadata-title">Edit {metadata().length} selected Questions</h2>
        </div>
        <button
          type="button"
          class="quiet-action"
          disabled={busy()}
          onClick={() => setCloseRequested(true)}
        >
          Close editor
        </button>
      </div>
      <p>
        Keep preserves every Question's current value, including mixed values. Discipline and
        Subject cannot be cleared. Clearing Topic or Subtopic requires an explicit Clear choice.
      </p>
      <p>
        When replacing a parent, explicitly replace or clear any incompatible narrower values. The
        server checks the complete hierarchy for every selected Question.
      </p>
      <dl class="bulk-metadata-summary">
        <div>
          <dt>Tags</dt>
          <dd>{fieldSummary(metadata(), "tags", currentNames())}</dd>
        </div>
        <For each={CLASSIFICATION_FIELDS}>
          {(field) => (
            <div>
              <dt>{CLASSIFICATION_LABELS[field]}</dt>
              <dd>{fieldSummary(metadata(), field, currentNames())}</dd>
            </div>
          )}
        </For>
      </dl>
      <details>
        <summary>Review current values for every selected Question</summary>
        <div class="bulk-metadata-current-values">
          <SearchBlock
            layout="above"
            controls={
              <SearchControls
                state={metadataSearch.state}
                textLabel="Search selected Question metadata"
                disabled={busy}
              />
            }
            state={metadataSearch.state}
            collection={{
              ariaLabel: "Current metadata by selected Question",
              ...metadataSearch.collection,
              emptyState: { title: "No selected Questions match this metadata search." },
              disabled: busy,
            }}
          />
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
        <For each={CLASSIFICATION_FIELDS}>
          {(field, index) => {
            const parentField = (): ClassificationField | undefined =>
              CLASSIFICATION_FIELDS[index() - 1];
            const parent = (): string | null | undefined => {
              const previous = parentField();
              return previous === undefined ? undefined : effectiveParent(previous);
            };
            function load(
              uuid: string,
            ): ReturnType<ContentClassificationClient["listDisciplines"]> {
              if (field === "disciplineUuid") {
                return props.classificationClient.listDisciplinesIncludingRetired();
              }
              if (field === "subjectUuid") return props.classificationClient.listSubjects(uuid);
              if (field === "topicUuid") return props.classificationClient.listTopics(uuid);
              return props.classificationClient.listSubtopics(uuid);
            }
            return (
              <MetadataMode
                field={field}
                label={CLASSIFICATION_LABELS[field]}
                mode={modes()[field]}
                required={field === "disciplineUuid" || field === "subjectUuid"}
                disabled={busy() || state() === "refresh-error"}
                onChange={(mode) => changeMode(field, mode)}
              >
                <Show
                  when={field !== "disciplineUuid"}
                  fallback={
                    <ContentClassificationSelect
                      label={CLASSIFICATION_LABELS[field]}
                      required
                      value={selections()[field]}
                      parentUuid={parent()}
                      load={load}
                      onChange={(uuid) => changeSelection(field, uuid)}
                    />
                  }
                >
                  <AuthoringClassificationLevel
                    label={CLASSIFICATION_LABELS[field]}
                    required={field === "subjectUuid"}
                    value={selections()[field]}
                    parentUuid={parent() ?? null}
                    disabled={busy() || state() === "refresh-error"}
                    load={load}
                    onChange={(uuid) => changeSelection(field, uuid)}
                    createName={(vocabularyName, parentUuid) =>
                      createLevel(field, vocabularyName, parentUuid)
                    }
                    acceptExisting={
                      field === "subjectUuid"
                        ? (
                            uuid,
                            parentUuid,
                          ): ReturnType<ContentClassificationClient["acceptSubjectDiscipline"]> =>
                            props.classificationClient.acceptSubjectDiscipline(uuid, parentUuid)
                        : undefined
                    }
                  />
                </Show>
                <Show when={parent() === null}>
                  <p>
                    Choose a replacement parent first when current parent values are mixed or unset.
                  </p>
                </Show>
              </MetadataMode>
            );
          }}
        </For>
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
            onClick={() => void trackedSave.save()}
          >
            Update all {metadata().length} Questions
          </button>
        </Show>
      </div>
    </section>
  );
}
