// Closed Star control and display-name list for one Published Question.

import { createResource, createSignal, Show, type JSX, type Resource } from "solid-js";

import type { PublishedQuestionId } from "../../generated/api/PublishedQuestionId";
import { useApplicationApi } from "../api/application_api";
import type { QuestionStarProjection, QuestionStarredInstructor } from "../api/question_star";
import { createLocalCompleteSearch } from "./search_block/local_complete_search_state";
import { SearchBlock } from "./search_block/search_block";
import { SearchControls } from "./search_block/search_controls";
import { InstructorProfileLink } from "./instructor_profile_link";
import type { RecordField } from "./data_sheet/record_fields";

const instructorFields: ReadonlyArray<RecordField<QuestionStarredInstructor, "name", "name">> = [
  {
    id: "name",
    label: "Instructor",
    role: "identity",
    sortKey: "name",
    filterKey: "name",
    value: (row) => row.displayName,
    defaultColumn: { priority: 0, width: "18rem" },
  },
  {
    id: "profile",
    label: "Profile",
    value: () => "Profile",
    render: (row) => <InstructorProfileLink accountId={row.accountId} displayName="Profile" />,
    defaultColumn: { priority: 0, width: "10rem" },
  },
];

export interface QuestionStarControlProps {
  readonly questionId: PublishedQuestionId;
}

export interface QuestionStarredInstructorListProps {
  readonly resource: Resource<QuestionStarProjection>;
  readonly refetchSource: () => unknown;
}

/** Instructor-only Star list with links to active Instructors' public Profiles. */
export function QuestionStarredInstructorList(
  props: QuestionStarredInstructorListProps,
): JSX.Element {
  const search = createLocalCompleteSearch(
    {
      rows: () => {
        const projection = props.resource();
        return projection === undefined ? undefined : projection.starredInstructors;
      },
      loading: () => props.resource.loading,
      error: (): unknown => props.resource.error,
      refetchSource: () => props.refetchSource(),
    },
    {
      fields: instructorFields,
      rowId: (row) => row.accountId,
      keyword: (row) => row.displayName,
      // ASVS 1.2.1: shared content renders the projected name as text.
      content: (row) => ({
        title: row.displayName,
        details: [{ kind: "instructorProfile", accountId: row.accountId, displayName: "Profile" }],
        actions: [],
      }),
    },
  );
  return (
    <section aria-label="Instructors who starred this question">
      <h2>Starred by</h2>
      <SearchBlock
        layout="above"
        controls={<SearchControls state={search.state} textLabel="Instructor name" />}
        state={search.state}
        collection={{
          ariaLabel: "Instructors who starred this question",
          fields: instructorFields,
          query: search.query,
          emptyState: { title: "No matching Instructor Stars." },
        }}
      />
    </section>
  );
}

/**
 * Renders the active Instructor identities supplied by the Instructor-only
 * Star endpoint. Each record links to the ordinary signed-in Profile surface.
 */
export function QuestionStarControl(props: QuestionStarControlProps): JSX.Element {
  const applicationApi = useApplicationApi();
  const [star, { mutate, refetch }] = createResource(
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
    <section class="question-star-control" aria-label="Question Star">
      <Show when={star.error === undefined && star()}>
        {(projection) => (
          <>
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
          </>
        )}
      </Show>
      <QuestionStarredInstructorList resource={star} refetchSource={refetch} />
    </section>
  );
}
