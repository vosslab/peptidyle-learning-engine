import { For, batch, type JSX } from "solid-js";

import type { ContentClassificationClient } from "../../api/content_classification";
import { AuthoringClassificationLevel } from "../../components/authoring_classification_level";
import { ContentClassificationSelect } from "../../components/content_classification_select";
export { canPublishWebworkDraft, isDraftSnapshotSaved } from "./question_publication_review_model";

export type QuestionPublicationReviewFieldsProps = {
  readonly questionTitle: string;
  readonly changed: ReadonlyArray<string>;
  readonly isSaved: () => boolean;
  readonly isLocked: () => boolean;
  readonly authorshipText: string;
  readonly disciplineUuid: string | null;
  readonly subjectUuid: string | null;
  readonly topicUuid: string | null;
  readonly subtopicUuid: string | null;
  readonly classificationClient: ContentClassificationClient;
  readonly onAuthorshipTextChange: (value: string) => void;
  readonly onAuthorshipInput: (element: HTMLTextAreaElement) => void;
  readonly onDisciplineChange: (uuid: string | null) => void;
  readonly onSubjectChange: (uuid: string | null) => void;
  readonly onTopicChange: (uuid: string | null) => void;
  readonly onSubtopicChange: (uuid: string | null) => void;
  readonly onPublish: () => void;
};

/** Shared authorship and content classification review used by Native and WebWork Drafts. */
export function QuestionPublicationReviewFields(
  props: QuestionPublicationReviewFieldsProps,
): JSX.Element {
  return (
    <div class="ple-question-json-authoring__review">
      <p>
        <strong>Question:</strong> {props.questionTitle}
      </p>
      <p>
        This publication creates a new Question ID. Existing assessments keep their assigned
        questions until an instructor deliberately replaces an item.
      </p>
      <h3>Changed sections</h3>
      <ul>
        <For each={props.changed}>{(section) => <li>{section}</li>}</For>
      </ul>
      <label class="ple-question-json-authoring__field">
        <span>Question Authors</span>
        <textarea
          ref={props.onAuthorshipInput}
          value={props.authorshipText}
          onInput={(event) => props.onAuthorshipTextChange(event.currentTarget.value)}
          aria-describedby="ple-question-json-authorship-help"
          disabled={props.isLocked()}
        />
        <span id="ple-question-json-authorship-help" class="ple-question-json-authoring__help">
          Enter one to sixteen distinct names, one per line. This reviewed text, not account
          information, is published with the question.
        </span>
      </label>
      <div class="publication-classification-fields">
        <ContentClassificationSelect
          label="Discipline"
          required
          value={props.disciplineUuid}
          disabled={props.isLocked()}
          load={() => props.classificationClient.listDisciplinesIncludingRetired()}
          onChange={(uuid) =>
            batch(() => {
              props.onDisciplineChange(uuid);
              props.onSubjectChange(null);
              props.onTopicChange(null);
              props.onSubtopicChange(null);
            })
          }
        />
        <AuthoringClassificationLevel
          label="Subject"
          required
          value={props.subjectUuid}
          parentUuid={props.disciplineUuid}
          disabled={props.isLocked()}
          load={(uuid) => props.classificationClient.listSubjects(uuid)}
          onChange={(uuid) =>
            batch(() => {
              props.onSubjectChange(uuid);
              props.onTopicChange(null);
              props.onSubtopicChange(null);
            })
          }
          createName={(name, parent) => props.classificationClient.createSubject(name, parent)}
          acceptExisting={(uuid, parent) =>
            props.classificationClient.acceptSubjectDiscipline(uuid, parent)
          }
        />
        <AuthoringClassificationLevel
          label="Topic"
          value={props.topicUuid}
          parentUuid={props.subjectUuid}
          disabled={props.isLocked()}
          load={(uuid) => props.classificationClient.listTopics(uuid)}
          onChange={(uuid) =>
            batch(() => {
              props.onTopicChange(uuid);
              props.onSubtopicChange(null);
            })
          }
          createName={async (name, parent) => {
            const item = await props.classificationClient.createTopic(name, parent);
            return { uuid: item.uuid, name: item.name, needsAcceptance: false };
          }}
        />
        <AuthoringClassificationLevel
          label="Subtopic"
          value={props.subtopicUuid}
          parentUuid={props.topicUuid}
          disabled={props.isLocked()}
          load={(uuid) => props.classificationClient.listSubtopics(uuid)}
          onChange={props.onSubtopicChange}
          createName={async (name, parent) => {
            const item = await props.classificationClient.createSubtopic(name, parent);
            return { uuid: item.uuid, name: item.name, needsAcceptance: false };
          }}
        />
      </div>
      <p>Confirming publishes this saved private draft with a new Question ID.</p>
      <button
        type="button"
        class="primary-action"
        disabled={
          !props.isSaved() ||
          props.isLocked() ||
          props.disciplineUuid === null ||
          props.subjectUuid === null
        }
        onClick={() => void props.onPublish()}
      >
        Confirm and publish
      </button>
    </div>
  );
}
