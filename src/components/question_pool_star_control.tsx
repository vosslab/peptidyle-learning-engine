// Closed Star control and display-name list for one Published Question Pool.

import { createResource, createSignal, Show, type JSX, type Resource } from "solid-js";

import type { QuestionPoolId } from "../../generated/api/QuestionPoolId";
import { useApplicationApi } from "../api/application_api";
import type {
  QuestionPoolStarProjection,
  QuestionPoolStarredInstructor,
} from "../api/question_pool_stewardship";
import { createLocalCompleteSearch } from "./search_block/local_complete_search_state";
import { SearchBlock } from "./search_block/search_block";
import { SearchControls } from "./search_block/search_controls";
import { InstructorProfileLink } from "./instructor_profile_link";
import type { RecordField } from "./data_sheet/data_sheet_fields";
import "./question_pool_star_control.css";

const instructorFields: ReadonlyArray<RecordField<QuestionPoolStarredInstructor, "name", "name">> =
  [
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

export interface QuestionPoolStarControlProps {
  readonly poolId: QuestionPoolId;
}

export interface QuestionPoolStarredInstructorListProps {
  readonly resource: Resource<QuestionPoolStarProjection>;
  readonly refetchSource: () => unknown;
}

/** Instructor-only Star list with links to active Instructors' public Profiles. */
export function QuestionPoolStarredInstructorList(
  props: QuestionPoolStarredInstructorListProps,
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
    <section aria-label="Instructors who starred this Question Pool">
      <h2>Starred by</h2>
      <SearchBlock
        layout="above"
        controls={<SearchControls state={search.state} textLabel="Instructor name" />}
        state={search.state}
        collection={{
          ariaLabel: "Instructors who starred this Question Pool",
          ...search.collection,
          emptyState: { title: "No matching Instructor Stars." },
        }}
      />
    </section>
  );
}

/**
 * Renders active Instructor identities supplied by the Instructor-only Star
 * endpoint, linking each record to the public Profile surface.
 */
export function QuestionPoolStarControl(props: QuestionPoolStarControlProps): JSX.Element {
  const applicationApi = useApplicationApi();
  const [star, { mutate, refetch }] = createResource(
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
    <section class="library-pool-star-control" aria-label="Question Pool Star">
      <Show when={star.error === undefined && star()}>
        {(projection) => (
          <>
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
          </>
        )}
      </Show>
      <QuestionPoolStarredInstructorList resource={star} refetchSource={refetch} />
    </section>
  );
}
