// Browser capability contract for reusable blueprints and shared Alpha curricula.

import type { AlphaCourseDefinitionInput } from "../../generated/api/AlphaCourseDefinitionInput";
import type { AlphaCourseReference } from "../../generated/api/AlphaCourseReference";
import type { AlphaCourseSummaryView } from "../../generated/api/AlphaCourseSummaryView";
import type { AlphaCourseView } from "../../generated/api/AlphaCourseView";
import type { BlueprintDefinitionInput } from "../../generated/api/BlueprintDefinitionInput";
import type { BlueprintReference } from "../../generated/api/BlueprintReference";
import type { BlueprintSummaryView } from "../../generated/api/BlueprintSummaryView";
import type { BlueprintView } from "../../generated/api/BlueprintView";
import type { CursorPage } from "./contracts";

/** Strong server ETag retained unchanged for a subsequent curriculum mutation. */
export type ReusableCurriculumEtag = string;

export interface RevisionedBlueprint {
  readonly blueprint: BlueprintView;
  readonly etag: ReusableCurriculumEtag;
}

export interface RevisionedAlphaCourse {
  readonly alpha: AlphaCourseView;
  readonly etag: ReusableCurriculumEtag;
}

/** Browser capability for instructor-owned blueprints and approved shared Alpha curricula. */
export interface ReusableCurriculumClient {
  readonly listBlueprints: (
    cursor?: string,
    pageSize?: number,
  ) => Promise<CursorPage<BlueprintSummaryView>>;
  readonly getBlueprint: (reference: BlueprintReference) => Promise<RevisionedBlueprint>;
  readonly createBlueprint: (definition: BlueprintDefinitionInput) => Promise<RevisionedBlueprint>;
  readonly replaceBlueprint: (
    reference: BlueprintReference,
    definition: BlueprintDefinitionInput,
    etag: ReusableCurriculumEtag,
  ) => Promise<RevisionedBlueprint>;
  readonly deleteBlueprint: (
    reference: BlueprintReference,
    etag: ReusableCurriculumEtag,
  ) => Promise<void>;
  readonly listAlphaCourses: (
    cursor?: string,
    pageSize?: number,
  ) => Promise<CursorPage<AlphaCourseSummaryView>>;
  readonly getAlphaCourse: (reference: AlphaCourseReference) => Promise<RevisionedAlphaCourse>;
  readonly createAlphaCourse: (
    definition: AlphaCourseDefinitionInput,
  ) => Promise<RevisionedAlphaCourse>;
  readonly replaceAlphaCourse: (
    reference: AlphaCourseReference,
    definition: AlphaCourseDefinitionInput,
    etag: ReusableCurriculumEtag,
  ) => Promise<RevisionedAlphaCourse>;
}
