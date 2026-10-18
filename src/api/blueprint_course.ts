// Browser capability contract for Blueprint lineage metadata and immutable Revisions.

import type { BlueprintCourseId } from "../../generated/api/BlueprintCourseId";
import type { BlueprintAssessmentId } from "../../generated/api/BlueprintAssessmentId";
import type { BlueprintPoolMembersView } from "../../generated/api/BlueprintPoolMembersView";
import type { PublishedQuestionId } from "../../generated/api/PublishedQuestionId";
import type { BlueprintCourseListPageView } from "../../generated/api/BlueprintCourseListPageView";
import type { BlueprintCourseView } from "../../generated/api/BlueprintCourseView";
import type { BlueprintRevisionView } from "../../generated/api/BlueprintRevisionView";
import type { BlueprintHistoryPageView } from "../../generated/api/BlueprintHistoryPageView";

export type BlueprintHistoryQuery = {
  readonly q: string;
  readonly sort: "sequence" | "recordedAt" | "name";
  readonly direction: "ascending" | "descending";
  readonly availability: ReadonlyArray<"active" | "archived">;
};

export type BlueprintHistoryPage = BlueprintHistoryPageView & {
  readonly availabilityFacets: { readonly activeCount: number; readonly archivedCount: number };
};
import type { BlueprintComparisonView } from "../../generated/api/BlueprintComparisonView";
import type { BlueprintKnownForkView } from "../../generated/api/BlueprintKnownForkView";
import type { BlueprintCourseSaveResponse } from "../../generated/api/BlueprintCourseSaveResponse";
import type { BlueprintEditNumber } from "../../generated/api/BlueprintEditNumber";
import type { BlueprintForkApplyRequest } from "../../generated/api/BlueprintForkApplyRequest";
import type { BlueprintForkApplyResponse } from "../../generated/api/BlueprintForkApplyResponse";
import type { BlueprintMetadataState } from "../../generated/api/BlueprintMetadataState";
import type { CreateBlueprintCourseInput } from "../../generated/api/CreateBlueprintCourseInput";
import type { RenameBlueprintCourseInput } from "../../generated/api/RenameBlueprintCourseInput";
import type { ReplaceBlueprintCourseContentInput } from "../../generated/api/ReplaceBlueprintCourseContentInput";
import type { BlueprintThemeUpdate } from "../../generated/api/BlueprintThemeUpdate";
import type { CourseClassification } from "../../generated/api/CourseClassification";
import type { CanonicalBlueprintCourse } from "../../generated/api/CanonicalBlueprintCourse";
import type { BlueprintStewardshipClient } from "./blueprint_stewardship";
import type { BlueprintRevisionNumber } from "../../generated/api/BlueprintRevisionNumber";
import type { RecognitionTitleMaps } from "./recognition_titles";

export type { BlueprintEditNumber, BlueprintRevisionNumber };
export type BlueprintIdempotencyKey = string;

/** Optional identity-based discovery restrictions; Discipline remains the browsing anchor. */
export interface BlueprintCourseClassificationSearch {
  readonly disciplineUuid: string | null;
  readonly subjectUuid: string | null;
  readonly topicUuid: string | null;
  readonly subtopicUuid: string | null;
  readonly crossDiscipline: boolean;
}

/** Closed server-owned ordering choices for My Blueprint Courses. */
export type BlueprintCourseListSort =
  | "name"
  | "nameDescending"
  | "adoptions"
  | "adoptionsAscending"
  | "students"
  | "studentsAscending"
  | "stars"
  | "starsAscending"
  | "watches"
  | "watchesAscending"
  | "recentEdits"
  | "recentEditsAscending";

export type BlueprintCourseFacetField =
  "disciplineUuids" | "subjectUuids" | "topicUuids" | "subtopicUuids" | "tags";

export type BlueprintCourseFacetFilters = Readonly<
  Record<BlueprintCourseFacetField, ReadonlyArray<string>>
>;

export interface BlueprintCourseFacetRequest {
  readonly includeArchived: boolean;
  readonly activeOnly: boolean;
  readonly promotedOnly: boolean;
  readonly query: string;
  readonly classification: BlueprintCourseClassificationSearch;
  readonly sort: BlueprintCourseListSort;
  readonly tag: string;
  readonly facets: BlueprintCourseFacetFilters;
}

export interface BlueprintCourseFacetChoices {
  readonly choices: ReadonlyArray<{
    readonly key: string;
    readonly label: string;
    readonly count: number;
  }>;
  readonly selected: ReadonlyArray<{
    readonly key: string;
    readonly label: string;
    readonly count: number;
  }>;
  readonly nextCursor: string | null;
}

export interface LoadedBlueprintCourse {
  readonly blueprintCourse: BlueprintCourseView;
}

export interface BlueprintMetadataTransition {
  readonly metadata: BlueprintMetadataState;
}

/** Browser capability for Instructor-owned reusable Blueprint Course lifecycle operations. */
export interface BlueprintCourseClient extends BlueprintStewardshipClient {
  /** Exports the current reusable structure without ownership or delivery state. */
  readonly exportBlueprintCourse: (
    blueprintCourseId: BlueprintCourseId,
  ) => Promise<CanonicalBlueprintCourse>;
  /** Creates an independent Active Blueprint Course from validated reusable structure. */
  readonly importBlueprintCourse: (
    exchange: CanonicalBlueprintCourse,
    idempotencyKey: BlueprintIdempotencyKey,
  ) => Promise<LoadedBlueprintCourse>;
  readonly updateBlueprintCourseClassification: (
    blueprintCourseId: BlueprintCourseId,
    classification: CourseClassification,
    expectedBlueprintEditNumber: BlueprintEditNumber,
  ) => Promise<BlueprintMetadataTransition>;
  readonly updateBlueprintCourseTheme: (
    blueprintCourseId: BlueprintCourseId,
    update: BlueprintThemeUpdate,
    expectedBlueprintEditNumber: BlueprintEditNumber,
  ) => Promise<BlueprintMetadataTransition>;
  readonly listBlueprintHistory: (
    blueprintCourseId: BlueprintCourseId,
    kind: "revisions" | "metadata",
    query: BlueprintHistoryQuery,
    pageNumber: number,
    pageSize: number,
  ) => Promise<BlueprintHistoryPage>;
  readonly getBlueprintPoolMembers: (
    blueprintCourseId: BlueprintCourseId,
    assessmentId: BlueprintAssessmentId,
    poolId: PublishedQuestionId,
  ) => Promise<BlueprintPoolMembersView>;
  /** Current lineage titles for the requested Question and Pool public IDs. */
  readonly loadRecognitionTitles: (
    questionIds: readonly string[],
    poolIds: readonly string[],
  ) => Promise<RecognitionTitleMaps>;
  readonly forkBlueprintCourse: (
    blueprintCourseId: BlueprintCourseId,
    revisionNumber: BlueprintRevisionNumber,
    idempotencyKey: BlueprintIdempotencyKey,
  ) => Promise<LoadedBlueprintCourse>;
  readonly applyBlueprintFork: (
    blueprintCourseId: BlueprintCourseId,
    request: BlueprintForkApplyRequest,
  ) => Promise<BlueprintForkApplyResponse>;
  readonly listKnownBlueprintForks: (
    blueprintCourseId: BlueprintCourseId,
  ) => Promise<readonly BlueprintKnownForkView[]>;
  readonly getBlueprintComparison: (
    leftBlueprintCourseId: BlueprintCourseId,
    rightBlueprintCourseId: BlueprintCourseId,
  ) => Promise<BlueprintComparisonView>;
  readonly listBlueprintCourses: (
    pageNumber: number,
    pageSize: number,
    includeArchived?: boolean,
    query?: string,
    activeOnly?: boolean,
    promotedOnly?: boolean,
    classification?: BlueprintCourseClassificationSearch,
    sort?: BlueprintCourseListSort,
    tag?: string,
    facets?: BlueprintCourseFacetFilters,
  ) => Promise<BlueprintCourseListPageView>;
  readonly listBlueprintCourseFacetChoices: (
    field: BlueprintCourseFacetField,
    request: BlueprintCourseFacetRequest,
    choiceSearch: string,
    choiceCursor: string | null,
  ) => Promise<BlueprintCourseFacetChoices>;
  readonly getBlueprintCourse: (
    blueprintCourseId: BlueprintCourseId,
  ) => Promise<LoadedBlueprintCourse>;
  readonly createBlueprintCourse: (
    content: CreateBlueprintCourseInput,
    idempotencyKey: BlueprintIdempotencyKey,
  ) => Promise<LoadedBlueprintCourse>;
  readonly saveBlueprintCourse: (
    blueprintCourseId: BlueprintCourseId,
    content: ReplaceBlueprintCourseContentInput,
    expectedCurrentBlueprintRevisionNumber: BlueprintRevisionNumber,
    idempotencyKey: BlueprintIdempotencyKey,
  ) => Promise<BlueprintCourseSaveResponse>;
  readonly renameBlueprintCourse: (
    blueprintCourseId: BlueprintCourseId,
    names: RenameBlueprintCourseInput,
    expectedBlueprintEditNumber: BlueprintEditNumber,
  ) => Promise<BlueprintMetadataTransition>;
  readonly getBlueprintRevision: (
    blueprintCourseId: BlueprintCourseId,
    revisionNumber: BlueprintRevisionNumber,
  ) => Promise<BlueprintRevisionView>;
  readonly archiveBlueprintCourse: (
    blueprintCourseId: BlueprintCourseId,
    confirmationLongName: string,
    expectedBlueprintEditNumber: BlueprintEditNumber,
  ) => Promise<BlueprintMetadataTransition>;
  readonly restoreBlueprintCourse: (
    blueprintCourseId: BlueprintCourseId,
    expectedBlueprintEditNumber: BlueprintEditNumber,
  ) => Promise<BlueprintMetadataTransition>;
}
