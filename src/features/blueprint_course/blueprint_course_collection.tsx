// Shared Instructor discovery of Blueprint Courses.
import { Show, type JSX } from "solid-js";

import type { BlueprintCourseClient } from "../../api/blueprint_course";
import type { ContentClassificationClient } from "../../api/content_classification";
import { useSessionBootstrap } from "../../auth/session_context";
import { SearchBlock } from "../../components/search_block/search_block";
import { SearchControls } from "../../components/search_block/search_controls";
import { createSearchState } from "../../components/search_block/search_state";
import { useSelectedDisplayDateTimeFormatter } from "../../selected_display_zone";
import { BlueprintSearchClassification } from "./blueprint_course_search_classification";
import {
  blueprintCourseSearchFields,
  blueprintCourseSearchRecordQuery,
} from "./blueprint_course_search_fields";
import {
  blueprintCourseSearchDefinition,
  type BlueprintCourseSearchQuery,
} from "./blueprint_course_search_definition";
import "./blueprint_course.css";

export interface BlueprintCourseCollectionProps {
  readonly client: Pick<
    BlueprintCourseClient,
    "listBlueprintCourses" | "listBlueprintCourseFacetChoices"
  > &
    ContentClassificationClient;
  readonly scope: "active" | "all";
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
    sort: searchState.sort,
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
          ariaLabel: props.scope === "all" ? "Blueprint Courses" : "Active Blueprint Courses",
          fields,
          query: recordQuery,
          initialMode: props.scope === "active" ? "cards" : undefined,
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
