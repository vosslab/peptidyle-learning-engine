// Shared Instructor discovery of Public and Archived Blueprint Courses.
import { Show, type JSX } from "solid-js";

import type { BlueprintCourseClient } from "../api/blueprint_course";
import type { ContentClassificationClient } from "../api/content_classification";
import { useSessionBootstrap } from "../auth/session_context";
import { PageFrame } from "../components/page_frame";
import { SearchBlock } from "../features/search/search_block";
import { SearchControls } from "../features/search/search_controls";
import { createSearchState } from "../features/search/search_state";
import { useSelectedDisplayDateTimeFormatter } from "../selected_display_zone";
import { BlueprintSearchClassification } from "./blueprint_course_search_classification";
import {
  blueprintCourseSearchFields,
  blueprintCourseSearchRecordQuery,
} from "./blueprint_course_search_fields";
import {
  blueprintCourseSearchDefinition,
  type BlueprintCourseSearchQuery,
} from "./blueprint_course_search_definition";
import "../features/blueprint_course/blueprint_course.css";

export interface PublicBlueprintSearchPageProps {
  readonly client: Pick<
    BlueprintCourseClient,
    "listBlueprintCourses" | "listBlueprintCourseFacetChoices"
  > &
    ContentClassificationClient;
}

export interface BlueprintCourseCollectionProps extends PublicBlueprintSearchPageProps {
  readonly scope: "public" | "all";
  readonly actions?: JSX.Element;
}

/** Both routes submit the same whole-collection predicates to one paged query owner. */
export function BlueprintCourseCollection(props: BlueprintCourseCollectionProps): JSX.Element {
  const sessionState = useSessionBootstrap().state();
  if (sessionState.kind !== "authenticated") {
    throw new Error("Blueprint Course discovery requires an authenticated session scope");
  }
  const formatTimestamp = useSelectedDisplayDateTimeFormatter();
  const searchState = createSearchState(
    blueprintCourseSearchDefinition(props.client, formatTimestamp, props.scope),
    { runOnMount: props.scope === "all" },
  );
  const query = searchState.query;
  const fields = blueprintCourseSearchFields(props.client, props.scope, formatTimestamp);
  const recordQuery = blueprintCourseSearchRecordQuery({
    query,
    apply: searchState.apply,
    client: props.client,
    scope: props.scope,
  });

  function applyFilters(change: Partial<BlueprintCourseSearchQuery>): void {
    void searchState.apply((current) => ({ ...current, ...change }));
  }

  const controls = (
    <SearchControls state={searchState} textLabel="Blueprint Course name">
      <label>
        <input
          type="checkbox"
          checked={query().promotedOnly}
          onChange={(event) => applyFilters({ promotedOnly: event.currentTarget.checked })}
        />{" "}
        Promoted only
      </label>
      <Show when={props.scope === "all"}>
        <label>
          <input
            type="checkbox"
            checked={query().includeArchived}
            onChange={(event) => applyFilters({ includeArchived: event.currentTarget.checked })}
          />{" "}
          Include Archived
        </label>
      </Show>
      <div class="blueprint-public-search__classification">
        <BlueprintSearchClassification
          client={props.client}
          value={query().classification}
          onChange={(classification, classificationDescription) =>
            applyFilters({ classification, classificationDescription })
          }
        />
      </div>
    </SearchControls>
  );
  return (
    <>
      {props.actions}
      <SearchBlock
        state={searchState}
        layout="above"
        controls={controls}
        collection={{
          ariaLabel: props.scope === "all" ? "Blueprint Courses" : "Public Blueprint Courses",
          fields,
          query: recordQuery,
          initialEmptyState: {
            title: "Enter a name or choose a classification to search.",
            message: "",
          },
          emptyState: {
            title: "No Blueprint Courses match.",
            message: "Try another name or classification, or clear the search.",
          },
        }}
      />
    </>
  );
}

/** Public discovery keeps its own route title and visibility scope. */
export function PublicBlueprintSearchPage(props: PublicBlueprintSearchPageProps): JSX.Element {
  return (
    <PageFrame routeSurface="publicBlueprintSearch">
      <BlueprintCourseCollection client={props.client} scope="public" />
    </PageFrame>
  );
}
