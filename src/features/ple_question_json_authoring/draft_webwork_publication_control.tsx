import { A } from "@solidjs/router";
import { Show, createEffect, createSignal, type Accessor, type JSX } from "solid-js";

import type { QuestionSummary } from "../../../generated/api/QuestionSummary";
import type { QuestionType } from "../../../generated/api/QuestionType";
import type { ContentClassificationClient } from "../../api/content_classification";
import { parseReviewedQuestionAuthorship } from "../../api/question_authorship";
import type { DraftQuestionRouteId } from "../../navigation/public_route";
import { buildRoutePath } from "../../ribbon/ribbon_route_path";
import type { PleQuestionJsonPublicationRequest } from "./question_json_repository";
import { canPublishWebworkDraft } from "./question_publication_review_model";
import { QuestionPublicationReviewFields } from "./question_publication_review_fields";

export type DraftWebworkPublicationControlProps = {
  readonly draftQuestion: DraftQuestionRouteId;
  readonly questionTitle: Accessor<string>;
  readonly authors: Accessor<ReadonlyArray<string>>;
  readonly questionType: Accessor<QuestionType | null>;
  readonly draftQuestionEditNumber: Accessor<string>;
  readonly isSaved: Accessor<boolean>;
  readonly classificationClient: ContentClassificationClient;
  readonly publish: (
    draftQuestion: DraftQuestionRouteId,
    request: PleQuestionJsonPublicationRequest,
    expectedDraftQuestionEditNumber: string,
  ) => Promise<QuestionSummary>;
};

/** Saved-source and saved-Type gate for first publication of WebWork PG/PGML Drafts. */
export function DraftWebworkPublicationControl(
  props: DraftWebworkPublicationControlProps,
): JSX.Element {
  const [reviewing, setReviewing] = createSignal(false);
  const [publishing, setPublishing] = createSignal(false);
  const [error, setError] = createSignal("");
  const [authorshipText, setAuthorshipText] = createSignal(props.authors().join("\n"));
  const [authorshipEdited, setAuthorshipEdited] = createSignal(false);
  const [disciplineUuid, setDisciplineUuid] = createSignal<string | null>(null);
  const [subjectUuid, setSubjectUuid] = createSignal<string | null>(null);
  const [topicUuid, setTopicUuid] = createSignal<string | null>(null);
  const [subtopicUuid, setSubtopicUuid] = createSignal<string | null>(null);
  const [publishedSummary, setPublishedSummary] = createSignal<QuestionSummary>();
  let authorshipInput: HTMLTextAreaElement | undefined;

  createEffect(() => {
    if (!authorshipEdited()) setAuthorshipText(props.authors().join("\n"));
  });

  const isLocked = (): boolean => publishing() || publishedSummary() !== undefined;
  const canReview = (): boolean =>
    canPublishWebworkDraft(props.isSaved(), props.questionType()) && !isLocked();
  createEffect(() => {
    if (!props.isSaved()) setReviewing(false);
  });

  async function publish(): Promise<void> {
    if (!canReview() || !reviewing()) return;
    const authorship = parseReviewedQuestionAuthorship(authorshipText());
    if (authorship === null) {
      setError("Provide one to sixteen distinct reviewed Question Authors before publishing.");
      requestAnimationFrame(() => authorshipInput?.focus());
      return;
    }
    const discipline = disciplineUuid();
    const subject = subjectUuid();
    if (discipline === null || subject === null) {
      setError("Select a Discipline and Subject before publishing.");
      return;
    }

    setPublishing(true);
    setError("");
    try {
      const summary = await props.publish(
        props.draftQuestion,
        {
          authorship,
          disciplineUuid: discipline,
          subjectUuid: subject,
          topicUuid: topicUuid(),
          subtopicUuid: subtopicUuid(),
        },
        props.draftQuestionEditNumber(),
      );
      setPublishedSummary(summary);
    } catch (caught: unknown) {
      setError(
        caught instanceof Error && caught.message.length > 0 && caught.message.length < 240
          ? caught.message
          : "Publication could not finish. Your saved Draft remains available for editing.",
      );
    } finally {
      setPublishing(false);
    }
  }

  return (
    <section class="editor-panel" aria-labelledby="draft-webwork-publish-heading">
      <h2 id="draft-webwork-publish-heading">Publish review</h2>
      <p>Review the saved WebWork Draft before creating a new Question ID.</p>
      <Show when={!props.isSaved()}>
        <p role="status">Save all source, metadata, support, and Type changes before publishing.</p>
      </Show>
      <Show when={props.questionType() === null}>
        <p role="status">Select and save a WebWork Question Type before publication.</p>
      </Show>
      <Show when={!reviewing()}>
        <button
          type="button"
          class="primary-action"
          disabled={!canReview()}
          onClick={() => {
            setError("");
            setReviewing(true);
          }}
        >
          Review publication changes
        </button>
      </Show>
      <Show when={reviewing()}>
        <QuestionPublicationReviewFields
          questionTitle={props.questionTitle()}
          changed={["WebWork PG/PGML source", "Question record metadata", "WebWork Question Type"]}
          isSaved={props.isSaved}
          isLocked={isLocked}
          authorshipText={authorshipText()}
          disciplineUuid={disciplineUuid()}
          subjectUuid={subjectUuid()}
          topicUuid={topicUuid()}
          subtopicUuid={subtopicUuid()}
          classificationClient={props.classificationClient}
          onAuthorshipTextChange={(value) => {
            setAuthorshipEdited(true);
            setAuthorshipText(value);
          }}
          onAuthorshipInput={(element) => {
            authorshipInput = element;
          }}
          onDisciplineChange={setDisciplineUuid}
          onSubjectChange={setSubjectUuid}
          onTopicChange={setTopicUuid}
          onSubtopicChange={setSubtopicUuid}
          onPublish={() => void publish()}
        />
      </Show>
      <Show when={error()}>
        <p role="alert">{error()}</p>
      </Show>
      <Show when={publishedSummary()} keyed>
        {(summary) => (
          <section class="editor-panel" role="status">
            <h3>Published</h3>
            {/* ASVS 1.2.1: render the validated Question title and ID as Solid text. */}
            <p>
              <strong>Question:</strong> {props.questionTitle()}
            </p>
            <p>
              <strong>Question ID:</strong> <code>{summary.questionId}</code>
            </p>
            <p>
              <strong>Published Revision:</strong>{" "}
              {summary.publishedQuestionRevisionTuple.revisionNumber}
            </p>
            <A
              class="primary-action"
              href={
                buildRoutePath("questionDetail", { questionId: summary.questionId }) ??
                "/library/browse"
              }
            >
              Open published Question
            </A>
          </section>
        )}
      </Show>
    </section>
  );
}
