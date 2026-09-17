// Browser capability contract for Blueprint lineage metadata and immutable Revisions.

import type { BlueprintCourseReference } from "../../generated/api/BlueprintCourseReference";
import type { BlueprintAssessmentReference } from "../../generated/api/BlueprintAssessmentReference";
import type { BlueprintPoolMembersView } from "../../generated/api/BlueprintPoolMembersView";
import type { QuestionId } from "../../generated/api/QuestionId";
import type { BlueprintCourseSummaryView } from "../../generated/api/BlueprintCourseSummaryView";
import type { BlueprintCourseView } from "../../generated/api/BlueprintCourseView";
import type { BlueprintRevisionView } from "../../generated/api/BlueprintRevisionView";
import type { BlueprintHistoryPageView } from "../../generated/api/BlueprintHistoryPageView";
import type { BlueprintComparisonView } from "../../generated/api/BlueprintComparisonView";
import type { BlueprintKnownForkView } from "../../generated/api/BlueprintKnownForkView";
import type { BlueprintCourseSaveResponse } from "../../generated/api/BlueprintCourseSaveResponse";
import type { BlueprintForkApplyRequest } from "../../generated/api/BlueprintForkApplyRequest";
import type { BlueprintForkApplyResponse } from "../../generated/api/BlueprintForkApplyResponse";
import type { BlueprintMetadataState } from "../../generated/api/BlueprintMetadataState";
import type { CreateBlueprintCourseInput } from "../../generated/api/CreateBlueprintCourseInput";
import type { RenameBlueprintCourseInput } from "../../generated/api/RenameBlueprintCourseInput";
import type { ReplaceBlueprintCourseContentInput } from "../../generated/api/ReplaceBlueprintCourseContentInput";
import type { CursorPage } from "./contracts";
import type { CourseClassification } from "../../generated/api/CourseClassification";
import type { CanonicalBlueprintCourse } from "../../generated/api/CanonicalBlueprintCourse";
import type { BlueprintStewardshipClient } from "./blueprint_stewardship";

export type BlueprintRevisionEtag = string;
export type BlueprintMetadataEtag = string;
export type BlueprintIdempotencyKey = string;

/** Optional identity-based discovery restrictions; Discipline remains the browsing anchor. */
export interface BlueprintCourseClassificationSearch {
  readonly disciplineUuid: string | null;
  readonly subjectUuid: string | null;
  readonly topicUuid: string | null;
  readonly subtopicUuid: string | null;
  readonly crossDiscipline: boolean;
}

export interface LoadedBlueprintCourse {
  readonly blueprintCourse: BlueprintCourseView;
  /** Strong validator for the exact current Blueprint Revision. */
  readonly revisionEtag: BlueprintRevisionEtag;
}

export interface BlueprintMetadataTransition {
  readonly metadata: BlueprintMetadataState;
  /** Strong opaque validator for future lineage metadata changes. */
  readonly metadataEtag: BlueprintMetadataEtag;
}

/** Browser capability for Instructor-owned reusable Blueprint Course lifecycle operations. */
export interface BlueprintCourseClient extends BlueprintStewardshipClient {
  /** Exports the current reusable structure without ownership or delivery state. */
  readonly exportBlueprintCourse: (
    reference: BlueprintCourseReference,
  ) => Promise<CanonicalBlueprintCourse>;
  /** Creates an independent Private Blueprint Course from validated reusable structure. */
  readonly importBlueprintCourse: (
    exchange: CanonicalBlueprintCourse,
    idempotencyKey: BlueprintIdempotencyKey,
  ) => Promise<LoadedBlueprintCourse>;
  readonly updateBlueprintCourseClassification: (
    reference: BlueprintCourseReference,
    classification: CourseClassification,
    etag: BlueprintMetadataEtag,
  ) => Promise<BlueprintMetadataTransition>;
  readonly listBlueprintHistory: (
    reference: BlueprintCourseReference,
    kind?: "revisions" | "metadata",
    cursor?: string,
    pageSize?: number,
  ) => Promise<BlueprintHistoryPageView>;
  readonly getBlueprintPoolMembers: (
    reference: BlueprintCourseReference,
    assessmentReference: BlueprintAssessmentReference,
    poolId: QuestionId,
  ) => Promise<BlueprintPoolMembersView>;
  readonly forkBlueprintCourse: (
    reference: BlueprintCourseReference,
    revision: string,
    idempotencyKey: BlueprintIdempotencyKey,
  ) => Promise<LoadedBlueprintCourse>;
  readonly applyBlueprintFork: (
    reference: BlueprintCourseReference,
    request: BlueprintForkApplyRequest,
  ) => Promise<BlueprintForkApplyResponse>;
  readonly listKnownBlueprintForks: (
    reference: BlueprintCourseReference,
  ) => Promise<readonly BlueprintKnownForkView[]>;
  readonly getBlueprintComparison: (
    left: BlueprintCourseReference,
    right: BlueprintCourseReference,
  ) => Promise<BlueprintComparisonView>;
  readonly listBlueprintCourses: (
    cursor?: string,
    pageSize?: number,
    includeArchived?: boolean,
    query?: string,
    publicOnly?: boolean,
    promotedOnly?: boolean,
    classification?: BlueprintCourseClassificationSearch,
  ) => Promise<CursorPage<BlueprintCourseSummaryView>>;
  readonly getBlueprintCourse: (
    reference: BlueprintCourseReference,
  ) => Promise<LoadedBlueprintCourse>;
  readonly createBlueprintCourse: (
    content: CreateBlueprintCourseInput,
    idempotencyKey: BlueprintIdempotencyKey,
  ) => Promise<LoadedBlueprintCourse>;
  readonly saveBlueprintCourse: (
    reference: BlueprintCourseReference,
    content: ReplaceBlueprintCourseContentInput,
    etag: BlueprintRevisionEtag,
    idempotencyKey: BlueprintIdempotencyKey,
  ) => Promise<BlueprintCourseSaveResponse & { readonly revisionEtag: BlueprintRevisionEtag }>;
  readonly renameBlueprintCourse: (
    reference: BlueprintCourseReference,
    names: RenameBlueprintCourseInput,
    etag: BlueprintMetadataEtag,
  ) => Promise<BlueprintMetadataTransition>;
  readonly publishBlueprintCourse: (
    reference: BlueprintCourseReference,
    etag: BlueprintMetadataEtag,
  ) => Promise<BlueprintMetadataTransition>;
  readonly getBlueprintRevision: (
    reference: BlueprintCourseReference,
    revision: string,
  ) => Promise<BlueprintRevisionView>;
  readonly archiveBlueprintCourse: (
    reference: BlueprintCourseReference,
    confirmationLongName: string,
    etag: BlueprintMetadataEtag,
  ) => Promise<BlueprintMetadataTransition>;
  readonly restoreBlueprintCourse: (
    reference: BlueprintCourseReference,
    etag: BlueprintMetadataEtag,
  ) => Promise<BlueprintMetadataTransition>;
  readonly returnBlueprintCourseToPrivate: (
    reference: BlueprintCourseReference,
    etag: BlueprintMetadataEtag,
  ) => Promise<BlueprintMetadataTransition>;
}
