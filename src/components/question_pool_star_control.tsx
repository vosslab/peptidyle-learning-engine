// Closed Star control and vetted-name list for one Published Question Pool.

import { createResource, createSignal, Show, type JSX } from "solid-js";

import type { QuestionPoolId } from "../../generated/api/QuestionPoolId";
import { useApplicationApi } from "../api/application_api";
import type { QuestionPoolStarredInstructor } from "../api/question_pool_stewardship";
import { RecordList } from "./record_list/record_list";
import "./question_pool_star_control.css";

export interface QuestionPoolStarControlProps {
  readonly poolId: QuestionPoolId;
}

export interface QuestionPoolStarredInstructorListProps {
  readonly instructors: ReadonlyArray<QuestionPoolStarredInstructor>;
}

/** Plain-text presentation for the exact vetted names already authorized by the server. */
export function QuestionPoolStarredInstructorList(
  props: QuestionPoolStarredInstructorListProps,
): JSX.Element {
  return (
    <section aria-label="Instructors who starred this Question Pool">
      <h2>Starred by</h2>
      <RecordList
        ariaLabel="Instructors who starred this Question Pool"
        emptyState={{ title: "No Instructors have starred this Question Pool." }}
        recordId={(instructor) => instructor.displayName}
        content={(instructor) => ({
          title: instructor.displayName,
          details: [],
          actions: [],
        })}
        rows={props.instructors}
        state={{ kind: "ready" }}
      />
    </section>
  );
}

/**
 * Renders only the exact verified Instructor display names supplied by the
 * Pool Star endpoint. Names intentionally remain plain text: there is no
 * client lookup, Profile link, avatar, or substitute identity.
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
