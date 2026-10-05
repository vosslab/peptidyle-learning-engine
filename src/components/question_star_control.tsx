// Closed Star control and display-name list for one Published Question.

import { createResource, createSignal, Show, type JSX } from "solid-js";

import type { PublishedQuestionId } from "../../generated/api/PublishedQuestionId";
import { useApplicationApi } from "../api/application_api";
import type { QuestionStarredInstructor } from "../api/question_star";
import { InstructorProfileLink } from "./instructor_profile_link";

export interface QuestionStarControlProps {
  readonly questionId: PublishedQuestionId;
}

export interface QuestionStarredInstructorListProps {
  readonly instructors: ReadonlyArray<QuestionStarredInstructor>;
}

/** Instructor-only Star list with links to active Instructors' public Profiles. */
export function QuestionStarredInstructorList(
  props: QuestionStarredInstructorListProps,
): JSX.Element {
  return (
    <section aria-label="Instructors who starred this question">
      <h2>Starred by</h2>
      <ul>
        {props.instructors.map((instructor) => (
          <li>
            <InstructorProfileLink
              accountId={instructor.accountId}
              displayName={instructor.displayName}
            />
          </li>
        ))}
      </ul>
    </section>
  );
}

/**
 * Renders the active Instructor identities supplied by the Instructor-only
 * Star endpoint. Each name leads to the ordinary signed-in Profile surface.
 */
export function QuestionStarControl(props: QuestionStarControlProps): JSX.Element {
  const applicationApi = useApplicationApi();
  const [star, { mutate }] = createResource(
    () => props.questionId,
    (questionId) => applicationApi.client.getQuestionStar(questionId),
  );
  const [saving, setSaving] = createSignal(false);
  const [error, setError] = createSignal(false);

  async function toggleStar(): Promise<void> {
    const current = star();
    if (current === undefined || saving()) return;
    setSaving(true);
    setError(false);
    try {
      mutate(
        await applicationApi.client.setQuestionStar(props.questionId, !current.viewerHasStarred),
      );
    } catch {
      // ASVS 16.5.1: show a generic failure without exposing request details.
      setError(true);
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
          <Show when={error()}>
            <p role="alert">Your Star preference could not be saved. Try again.</p>
          </Show>
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
