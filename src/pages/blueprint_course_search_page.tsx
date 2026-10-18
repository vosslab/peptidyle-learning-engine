// Public Blueprint Course discovery route composition.
import type { JSX } from "solid-js";
import type { BlueprintCourseClient } from "../api/blueprint_course";
import type { ContentClassificationClient } from "../api/content_classification";
import { PageFrame } from "../components/page_frame/page_frame";
import { BlueprintCourseCollection } from "../features/blueprint_course/blueprint_course_collection";

export interface PublicBlueprintSearchPageProps {
  readonly client: Pick<
    BlueprintCourseClient,
    "listBlueprintCourses" | "listBlueprintCourseFacetChoices"
  > &
    ContentClassificationClient;
}

/** Public discovery keeps its route identity while using the active lifecycle scope. */
export function PublicBlueprintSearchPage(props: PublicBlueprintSearchPageProps): JSX.Element {
  return (
    <PageFrame routeSurface="publicBlueprintSearch">
      <BlueprintCourseCollection client={props.client} scope="active" />
    </PageFrame>
  );
}
