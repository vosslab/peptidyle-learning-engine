// Closed Star control and approved-name list for one Published Question.

import { createResource, createSignal, For, Show, type JSX } from "solid-js";

import type { PublishedQuestionId } from "../../generated/api/PublishedQuestionId";
import { useApplicationApi } from "../api/application_api";
import type { QuestionStarredInstructor } from "../api/question_star";

export interface QuestionStarControlProps {
  readonly questionId: PublishedQuestionId;
}

export interface QuestionStarredInstructorListProps {
  readonly instructors: ReadonlyArray<QuestionStarredInstructor>;
}

/** Plain-text presentation for the exact identity projection already authorized by the server. */
export function QuestionStarredInstructorList(
  props: QuestionStarredInstructorListProps,
): JSX.Element {
  return (
    <section aria-label="Instructors who starred this question">
      <h2>Starred by</h2>
      <ul>
        <For each={props.instructors}>{(instructor) => <li>{instructor.displayName}</li>}</For>
      </ul>
    </section>
  );
}

/**
 * Renders only the exact verified Instructor display names supplied by the
 * Star endpoint. Names intentionally remain plain text: there is no client
 * lookup, Profile link, avatar, or substitute identity.
 */
export function QuestionStarControl(props: QuestionStarControlProps): JSX.Element {
  const applicationApi = useApplicationApi();
  const [star, { mutate }] = createResource(
    () => props.questionId,
    (questionId) => applicationApi.client.getQuestionStar(questionId),
  );
  const [saving, setSaving] = createSignal(false);

  async function toggleStar(): Promise<void> {
    const current = star();
    if (current === undefined || saving()) return;
    setSaving(true);
    try {
      mutate(
        await applicationApi.client.setQuestionStar(props.questionId, !current.viewerHasStarred),
      );
    } finally {
      setSaving(false);
    }
  }

  return (
    <Show when={star()}>
      {(projection) => (
        <section class="question-star-control" aria-label="Question Star">
          <button
            type="button"
            class="question-preference-control"
            classList={{ selected: projection().viewerHasStarred }}
            aria-pressed={projection().viewerHasStarred}
            disabled={saving()}
            onClick={() => void toggleStar()}
          >
            {projection().viewerHasStarred ? "Unstar question" : "Star question"}
          </button>
          <p aria-live="polite">
            {projection().starCount}{" "}
            {projection().starCount === 1 ? "Instructor has" : "Instructors have"} starred this
            question.
          </p>
          <Show when={projection().starredInstructors.length > 0}>
            <QuestionStarredInstructorList instructors={projection().starredInstructors} />
          </Show>
        </section>
      )}
    </Show>
  );
}
