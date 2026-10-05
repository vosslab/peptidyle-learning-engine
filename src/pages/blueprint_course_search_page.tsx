// Instructor-only discovery of available Public Blueprint Courses.

import { createEffect, createSignal, type JSX } from "solid-js";

import type { BlueprintCourseClient } from "../api/blueprint_course";
import type { ContentClassificationClient } from "../api/content_classification";
import { useSessionBootstrap } from "../auth/session_context";
import { SearchControls } from "../features/search/search_controls";
import { SearchResults } from "../features/search/search_results";
import { SearchPage } from "../features/search/search_page";
import { createSearchState } from "../features/search/search_state";
import { BlueprintSearchClassification } from "./blueprint_course_search_classification";
import {
  blueprintCourseSearchDefinition,
  type BlueprintCourseSearchQuery,
} from "./blueprint_course_search_definition";
import "../features/blueprint_course/blueprint_course.css";

export interface PublicBlueprintSearchPageProps {
  readonly client: Pick<BlueprintCourseClient, "listBlueprintCourses"> &
    ContentClassificationClient;
}

/** Public Blueprint Course discovery using the shared submitted-search boundary. */
export function PublicBlueprintSearchPage(props: PublicBlueprintSearchPageProps): JSX.Element {
  const sessionState = useSessionBootstrap().state();
  if (sessionState.kind !== "authenticated") {
    throw new Error("Public Blueprint Search requires an authenticated session scope");
  }
  const searchState = createSearchState(blueprintCourseSearchDefinition(props.client));
  const query = searchState.query;
  const [tagText, setTagText] = createSignal(query().tag);
  let committedTag = query().tag;

  createEffect(() => {
    const nextTag = query().tag;
    if (nextTag === committedTag) return;
    committedTag = nextTag;
    setTagText(nextTag);
  });

  function applyFilters(change: Partial<BlueprintCourseSearchQuery>): void {
    void searchState.apply({ ...query(), ...change });
  }

  function submitSearch(): void {
    void searchState.apply({ ...query(), query: searchState.typedText(), tag: tagText() });
  }

  const controls = (
    <SearchControls
      state={searchState}
      textLabel="Blueprint Course name"
      displayAriaLabel="Blueprint result display"
      onSubmit={submitSearch}
      onClear={() => setTagText("")}
      hasDraft={() => tagText().length > 0}
    >
      <label>
        <input
          type="checkbox"
          checked={query().promotedOnly}
          onChange={(event) => applyFilters({ promotedOnly: event.currentTarget.checked })}
        />{" "}
        Promoted only
      </label>
      <div class="blueprint-public-search__classification">
        <BlueprintSearchClassification
          client={props.client}
          value={query().classification}
          onChange={(classification, classificationDescription) =>
            applyFilters({ classification, classificationDescription })
          }
        />
      </div>
      <label class="blueprint-public-search__query" for="public-blueprint-tag">
        <span>Tag</span>
        <input
          id="public-blueprint-tag"
          type="search"
          value={tagText()}
          onInput={(event) => setTagText(event.currentTarget.value)}
        />
      </label>
    </SearchControls>
  );
  const results = (
    <section class="blueprint-public-search" aria-labelledby="public-blueprint-results-heading">
      <h2 id="public-blueprint-results-heading">Available Public Blueprint Courses</h2>
      <SearchResults
        state={searchState}
        ariaLabel="Available Public Blueprint Courses"
        emptyState={{
          title: "No available Public Blueprint Courses match.",
          message: "Try another name or classification, or clear the search.",
        }}
      />
    </section>
  );

  return (
    <SearchPage
      state={searchState}
      hasDraft={() => tagText().length > 0}
      toolbar={controls}
      results={results}
      routeSurface="publicBlueprintSearch"
      eyebrow="Blueprint Courses"
      title="Search Public Blueprint Courses"
      lede="Find reusable course structure by short or long name. Open a Blueprint Course to inspect it or create a Course Instance."
    />
  );
}
