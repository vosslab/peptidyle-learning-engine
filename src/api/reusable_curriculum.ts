// Browser capability contract for reusable Blueprint Courses.

import type { BlueprintReference } from "../../generated/api/BlueprintReference";
import type { BlueprintCourseSummaryView } from "../../generated/api/BlueprintCourseSummaryView";
import type { BlueprintCourseView } from "../../generated/api/BlueprintCourseView";
import type { CreateBlueprintCourseDefinitionInput } from "../../generated/api/CreateBlueprintCourseDefinitionInput";
import type { ReplaceBlueprintCourseDefinitionInput } from "../../generated/api/ReplaceBlueprintCourseDefinitionInput";
import type { CursorPage } from "./contracts";

/** Strong server ETag retained unchanged for a subsequent curriculum mutation. */
export type ReusableCurriculumEtag = string;

export interface RevisionedBlueprintCourse {
  readonly blueprintCourse: BlueprintCourseView;
  readonly etag: ReusableCurriculumEtag;
}

/** Browser capability for Instructor-owned reusable Blueprint Courses. */
export interface ReusableCurriculumClient {
  readonly listBlueprintCourses: (
    cursor?: string,
    pageSize?: number,
  ) => Promise<CursorPage<BlueprintCourseSummaryView>>;
  readonly getBlueprintCourse: (
    reference: BlueprintReference,
  ) => Promise<RevisionedBlueprintCourse>;
  readonly createBlueprintCourse: (
    definition: CreateBlueprintCourseDefinitionInput,
  ) => Promise<RevisionedBlueprintCourse>;
  readonly replaceBlueprintCourse: (
    reference: BlueprintReference,
    definition: ReplaceBlueprintCourseDefinitionInput,
    etag: ReusableCurriculumEtag,
  ) => Promise<RevisionedBlueprintCourse>;
  readonly deleteBlueprintCourse: (
    reference: BlueprintReference,
    etag: ReusableCurriculumEtag,
  ) => Promise<void>;
}
