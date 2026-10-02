// Shared Topic, Subtopic, and Tag editor for many selected Question Pools.

import { Show, createSignal, type JSX } from "solid-js";

import type { QuestionPoolMetadataEditNumber } from "../../generated/api/QuestionPoolMetadataEditNumber";
import type { ContentClassificationClient } from "../api/content_classification";
import { ApiRequestError } from "../api/http_client/error";
import type {
  QuestionPoolSearchMetadataClient,
  QuestionPoolSearchMetadataUpdateResult,
} from "../api/question_pool_search_metadata";
import { ContentClassificationSelect } from "./content_classification_select";
import "./question_bulk_metadata_editor.css";

type EditMode = "keep" | "replace" | "clear";

export interface QuestionPoolSearchMetadataTarget {
  readonly questionPoolId: string;
  readonly questionPoolMetadataEditNumber: QuestionPoolMetadataEditNumber;
  readonly title: string;
  readonly disciplineName: string;
  readonly subjectUuid: string;
  readonly topicUuid: string | null;
  readonly subtopicUuid: string | null;
  readonly tags: ReadonlyArray<string>;
}

const MAX_TAG_CODE_POINTS = 120;

function hasControlCharacter(value: string): boolean {
  return [...value].some((character) => {
    const codePoint = character.codePointAt(0) ?? 0;
    return codePoint <= 0x1f || (codePoint >= 0x7f && codePoint <= 0x9f);
  });
}

function sharedText(values: ReadonlyArray<string>): string {
  return new Set(values).size === 1 ? (values[0] ?? "None") : `Mixed across ${values.length} Pools`;
}

function validatedTag(value: string): string {
  if (
    value.length === 0 ||
    value.trim() !== value ||
    [...value].length > MAX_TAG_CODE_POINTS ||
    hasControlCharacter(value)
  ) {
    throw new Error(
      `Each tag must be trimmed, nonempty, control-free text of at most ${MAX_TAG_CODE_POINTS} characters.`,
    );
  }
  return value;
}

function validatedTags(value: string): ReadonlyArray<string> {
  const tags = value
    .split("\n")
    .map((tag) => tag.trim())
    .filter((tag) => tag.length > 0)
    .map(validatedTag);
  if (tags.length === 0) throw new Error("Enter at least one replacement tag, or choose Clear.");
  if (new Set(tags).size !== tags.length) throw new Error("Enter each tag only once.");
  return tags;
}

function ModeChoice(props: {
  readonly field: string;
  readonly mode: EditMode;
  readonly value: EditMode;
  readonly disabled?: boolean;
  readonly onChange: (mode: EditMode) => void;
}): JSX.Element {
  return (
    <label>
      <input
        type="radio"
        name={`pool-search-metadata-${props.field}`}
        value={props.value}
        checked={props.mode === props.value}
        disabled={props.disabled}
        onChange={() => props.onChange(props.value)}
      />
      {props.value[0]?.toUpperCase()}
      {props.value.slice(1)}
    </label>
  );
}

/** Updates Pool-owned search fields and leaves established Discipline and Subject in place. */
export function QuestionPoolSearchMetadataEditor(props: {
  readonly client: QuestionPoolSearchMetadataClient;
  readonly classificationClient: ContentClassificationClient;
  readonly targets: ReadonlyArray<QuestionPoolSearchMetadataTarget>;
  readonly disabled?: boolean;
  readonly onSuccess: (results: ReadonlyArray<QuestionPoolSearchMetadataUpdateResult>) => void;
}): JSX.Element {
  const [topicMode, setTopicMode] = createSignal<EditMode>("keep");
  const [subtopicMode, setSubtopicMode] = createSignal<EditMode>("keep");
  const [tagsMode, setTagsMode] = createSignal<EditMode>("keep");
  const [topicUuid, setTopicUuid] = createSignal<string | null>(null);
  const [subtopicUuid, setSubtopicUuid] = createSignal<string | null>(null);
  const [tagsText, setTagsText] = createSignal("");
  const [saving, setSaving] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const sharedSubject = (): string | null => {
    const subjects = new Set(props.targets.map((target) => target.subjectUuid));
    return subjects.size === 1 ? (props.targets[0]?.subjectUuid ?? null) : null;
  };
  const keptTopic = (): string | null => {
    const topics = new Set(props.targets.map((target) => target.topicUuid));
    if (topics.size !== 1) return null;
    return props.targets[0]?.topicUuid ?? null;
  };
  const subtopicParent = (): string | null => {
    if (topicMode() === "clear") return null;
    if (topicMode() === "replace") return topicUuid();
    return keptTopic();
  };

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    setError(null);
    setSaving(true);
    try {
      if (topicMode() === "keep" && subtopicMode() === "keep" && tagsMode() === "keep") {
        throw new Error("Choose Replace or Clear for Topic, Subtopic, or Tags.");
      }
      if (topicMode() !== "keep" && sharedSubject() === null) {
        throw new Error("Topic replacement needs every selected Pool to share one Subject.");
      }
      const patch: {
        tags?: ReadonlyArray<string>;
        topicUuid?: string | null;
        subtopicUuid?: string | null;
      } = {};
      if (tagsMode() === "replace") patch.tags = validatedTags(tagsText());
      if (tagsMode() === "clear") patch.tags = [];
      if (topicMode() === "replace") {
        const replacement = topicUuid();
        if (replacement === null) throw new Error("Select a replacement Topic.");
        patch.topicUuid = replacement;
      }
      if (topicMode() === "clear") patch.topicUuid = null;
      if (topicMode() === "clear" || (topicMode() === "replace" && subtopicMode() === "keep")) {
        patch.subtopicUuid = null;
      } else if (subtopicMode() === "clear") {
        patch.subtopicUuid = null;
      } else if (subtopicMode() === "replace") {
        const replacement = subtopicUuid();
        if (replacement === null) throw new Error("Select a replacement Subtopic.");
        patch.subtopicUuid = replacement;
      }
      const selection = [...props.targets]
        .sort((left, right) =>
          left.questionPoolId < right.questionPoolId
            ? -1
            : left.questionPoolId > right.questionPoolId
              ? 1
              : 0,
        )
        .map((target) => ({
          questionPoolId: target.questionPoolId,
          questionPoolMetadataEditNumber: target.questionPoolMetadataEditNumber,
        }));
      const results = await props.client.updateQuestionPoolSearchMetadata({
        selection,
        patch,
      });
      props.onSuccess(results);
    } catch (cause) {
      if (cause instanceof ApiRequestError) {
        setError("Could not update the selected Pools. Check the fields and try again.");
      } else {
        setError(cause instanceof Error ? cause.message : "Could not update the selected Pools.");
      }
    } finally {
      setSaving(false);
    }
  }

  return (
    <form
      class="bulk-metadata-editor"
      aria-label="Pool shared search fields"
      onSubmit={(event) => void submit(event)}
    >
      <div class="bulk-metadata-heading">
        <h3>Update shared search fields</h3>
        <p>
          Discipline and Subject stay the values established by each Pool's first member Question.
          This command updates Topic, Subtopic, and Tags for {props.targets.length} selected Pools
          and does not change Edit Number.
        </p>
      </div>
      <dl class="bulk-metadata-summary">
        <div>
          <dt>Discipline</dt>
          <dd>{sharedText(props.targets.map((target) => target.disciplineName))}</dd>
        </div>
        <div>
          <dt>Subject</dt>
          <dd>
            {sharedSubject() === null ? `Mixed across ${props.targets.length} Pools` : "Shared"}
          </dd>
        </div>
      </dl>
      <fieldset class="bulk-metadata-field" disabled={props.disabled || saving()}>
        <legend>Topic</legend>
        <div class="bulk-metadata-modes">
          <ModeChoice field="topic" mode={topicMode()} value="keep" onChange={setTopicMode} />
          <ModeChoice
            field="topic"
            mode={topicMode()}
            value="replace"
            disabled={sharedSubject() === null}
            onChange={setTopicMode}
          />
          <ModeChoice field="topic" mode={topicMode()} value="clear" onChange={setTopicMode} />
        </div>
        <Show when={topicMode() === "replace" ? sharedSubject() : null}>
          {(subjectUuid) => (
            <ContentClassificationSelect
              label="Topic"
              value={topicUuid()}
              parentUuid={subjectUuid()}
              required
              load={(parentUuid) => props.classificationClient.listTopics(parentUuid)}
              onChange={setTopicUuid}
            />
          )}
        </Show>
        <Show when={sharedSubject() === null}>
          <p>Topic replacement needs every selected Pool to share one Subject.</p>
        </Show>
        <Show when={topicMode() === "clear" || topicMode() === "replace"}>
          <p>Changing Topic clears Subtopic unless you choose a new Subtopic.</p>
        </Show>
      </fieldset>
      <fieldset class="bulk-metadata-field" disabled={props.disabled || saving()}>
        <legend>Subtopic</legend>
        <div class="bulk-metadata-modes">
          <ModeChoice
            field="subtopic"
            mode={subtopicMode()}
            value="keep"
            onChange={setSubtopicMode}
          />
          <ModeChoice
            field="subtopic"
            mode={subtopicMode()}
            value="replace"
            disabled={subtopicParent() === null}
            onChange={setSubtopicMode}
          />
          <ModeChoice
            field="subtopic"
            mode={subtopicMode()}
            value="clear"
            onChange={setSubtopicMode}
          />
        </div>
        <Show when={subtopicMode() === "replace" ? subtopicParent() : null}>
          {(topic) => (
            <ContentClassificationSelect
              label="Subtopic"
              value={subtopicUuid()}
              parentUuid={topic()}
              required
              load={(parentUuid) => props.classificationClient.listSubtopics(parentUuid)}
              onChange={setSubtopicUuid}
            />
          )}
        </Show>
      </fieldset>
      <fieldset class="bulk-metadata-field" disabled={props.disabled || saving()}>
        <legend>Tags</legend>
        <div class="bulk-metadata-modes">
          <ModeChoice field="tags" mode={tagsMode()} value="keep" onChange={setTagsMode} />
          <ModeChoice field="tags" mode={tagsMode()} value="replace" onChange={setTagsMode} />
          <ModeChoice field="tags" mode={tagsMode()} value="clear" onChange={setTagsMode} />
        </div>
        <Show when={tagsMode() === "replace"}>
          <label>
            Replacement tags, one per line
            <textarea
              value={tagsText()}
              onInput={(event) => setTagsText(event.currentTarget.value)}
            />
          </label>
        </Show>
      </fieldset>
      <Show when={error()}>
        <p role="alert">{error()}</p>
      </Show>
      <div class="bulk-metadata-actions">
        <button type="submit" class="primary-action" disabled={props.disabled || saving()}>
          {saving() ? "Updating shared search fields..." : "Update shared search fields"}
        </button>
      </div>
    </form>
  );
}
