// Shared public Blueprint search filter shape and detail collection destination.
import type {
  BlueprintCourseClassificationSearch,
  BlueprintCourseListSort,
} from "../api/blueprint_course";

export interface BlueprintSearchSnapshot {
  readonly query: string;
  readonly promotedOnly: boolean;
  readonly classification: BlueprintCourseClassificationSearch;
  readonly classificationDescription: string;
  readonly sort: BlueprintCourseListSort;
  readonly tag: string;
}

export function blueprintDetailCollectionLink(readAccess: string | undefined): {
  readonly href: string;
  readonly label: string;
} {
  if (readAccess === "active_instructor") {
    return {
      href: "/blueprint-courses/search/public",
      label: "Return to Public Blueprint Courses",
    };
  }
  return { href: "/blueprint-courses", label: "Return to My Blueprint Courses" };
}
