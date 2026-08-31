// Browser capability contract for reusable Blueprint Courses.

import type { BlueprintCourseReference } from "../../generated/api/BlueprintCourseReference";
import type { BlueprintCourseSummaryView } from "../../generated/api/BlueprintCourseSummaryView";
import type { BlueprintCourseView } from "../../generated/api/BlueprintCourseView";
import type { CreateBlueprintCourseDefinitionInput } from "../../generated/api/CreateBlueprintCourseDefinitionInput";
import type { ReplaceBlueprintCourseDefinitionInput } from "../../generated/api/ReplaceBlueprintCourseDefinitionInput";
import type { CursorPage } from "./contracts";

/** Strong server ETag retained unchanged for a subsequent curriculum mutation. */
export type BlueprintCourseEtag = string;

export interface RevisionedBlueprintCourse {
  readonly blueprintCourse: BlueprintCourseView;
  readonly etag: BlueprintCourseEtag;
}

/** Browser capability for Instructor-owned reusable Blueprint Courses. */
export interface BlueprintCourseClient {
  readonly listBlueprintCourses: (
    cursor?: string,
    pageSize?: number,
  ) => Promise<CursorPage<BlueprintCourseSummaryView>>;
  readonly getBlueprintCourse: (
    reference: BlueprintCourseReference,
  ) => Promise<RevisionedBlueprintCourse>;
  readonly createBlueprintCourse: (
    definition: CreateBlueprintCourseDefinitionInput,
  ) => Promise<RevisionedBlueprintCourse>;
  readonly replaceBlueprintCourse: (
    reference: BlueprintCourseReference,
    definition: ReplaceBlueprintCourseDefinitionInput,
    etag: BlueprintCourseEtag,
  ) => Promise<RevisionedBlueprintCourse>;
  readonly deleteBlueprintCourse: (
    reference: BlueprintCourseReference,
    etag: BlueprintCourseEtag,
  ) => Promise<void>;
}
