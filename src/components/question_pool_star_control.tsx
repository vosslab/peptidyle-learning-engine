// Closed Star control and display-name list for one Published Question Pool.

import { createResource, createSignal, Show, type JSX } from "solid-js";

import type { QuestionPoolId } from "../../generated/api/QuestionPoolId";
import { useApplicationApi } from "../api/application_api";
import type { QuestionPoolStarredInstructor } from "../api/question_pool_stewardship";
import { InstructorProfileLink } from "./instructor_profile_link";
import "./question_pool_star_control.css";

export interface QuestionPoolStarControlProps {
  readonly poolId: QuestionPoolId;
}

export interface QuestionPoolStarredInstructorListProps {
  readonly instructors: ReadonlyArray<QuestionPoolStarredInstructor>;
}

/** Instructor-only Star list with links to active Instructors' public Profiles. */
export function QuestionPoolStarredInstructorList(
  props: QuestionPoolStarredInstructorListProps,
): JSX.Element {
  return (
    <section aria-label="Instructors who starred this Question Pool">
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
 * Renders active Instructor identities supplied by the Instructor-only Star
 * endpoint, linking each displayed name to the public Profile surface.
 */
export function QuestionPoolStarControl(props: QuestionPoolStarControlProps): JSX.Element {
  const applicationApi = useApplicationApi();
  const [star, { mutate }] = createResource(
    () => props.poolId,
    (poolId) => applicationApi.client.getQuestionPoolStar(poolId),
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
        await applicationApi.client.setQuestionPoolStar(props.poolId, !current.viewerHasStarred),
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
        <section class="library-pool-star-control" aria-label="Question Pool Star">
          <button
            type="button"
            classList={{ selected: projection().viewerHasStarred }}
            aria-pressed={projection().viewerHasStarred}
            disabled={saving()}
            onClick={() => void toggleStar()}
          >
            {projection().viewerHasStarred ? "Unstar Pool" : "Star Pool"}
          </button>
          <Show when={error()}>
            <p role="alert">Your Star preference could not be saved. Try again.</p>
          </Show>
          <p aria-live="polite">
            {projection().starCount}{" "}
            {projection().starCount === 1 ? "Instructor has" : "Instructors have"} starred this
            Question Pool.
          </p>
          <Show when={projection().starredInstructors.length > 0}>
            <QuestionPoolStarredInstructorList instructors={projection().starredInstructors} />
          </Show>
        </section>
      )}
    </Show>
  );
}
