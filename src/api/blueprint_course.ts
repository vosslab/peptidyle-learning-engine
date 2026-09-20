// Browser capability contract for Blueprint lineage metadata and immutable Revisions.

import type { BlueprintCourseId } from "../../generated/api/BlueprintCourseId";
import type { BlueprintAssessmentId } from "../../generated/api/BlueprintAssessmentId";
import type { BlueprintPoolMembersView } from "../../generated/api/BlueprintPoolMembersView";
import type { QuestionId } from "../../generated/api/QuestionId";
import type { BlueprintCourseSummaryView } from "../../generated/api/BlueprintCourseSummaryView";
import type { BlueprintCourseView } from "../../generated/api/BlueprintCourseView";
import type { BlueprintRevisionView } from "../../generated/api/BlueprintRevisionView";
import type { BlueprintHistoryPageView } from "../../generated/api/BlueprintHistoryPageView";
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
import type { CursorPage } from "./contracts";
import type { CourseClassification } from "../../generated/api/CourseClassification";
import type { CanonicalBlueprintCourse } from "../../generated/api/CanonicalBlueprintCourse";
import type { BlueprintStewardshipClient } from "./blueprint_stewardship";

export type BlueprintRevisionEtag = string;
export type { BlueprintEditNumber };
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
}

/** Browser capability for Instructor-owned reusable Blueprint Course lifecycle operations. */
export interface BlueprintCourseClient extends BlueprintStewardshipClient {
  /** Exports the current reusable structure without ownership or delivery state. */
  readonly exportBlueprintCourse: (
    blueprintCourseId: BlueprintCourseId,
  ) => Promise<CanonicalBlueprintCourse>;
  /** Creates an independent Private Blueprint Course from validated reusable structure. */
  readonly importBlueprintCourse: (
    exchange: CanonicalBlueprintCourse,
    idempotencyKey: BlueprintIdempotencyKey,
  ) => Promise<LoadedBlueprintCourse>;
  readonly updateBlueprintCourseClassification: (
    blueprintCourseId: BlueprintCourseId,
    classification: CourseClassification,
    etag: BlueprintEditNumber,
  ) => Promise<BlueprintMetadataTransition>;
  readonly listBlueprintHistory: (
    blueprintCourseId: BlueprintCourseId,
    kind?: "revisions" | "metadata",
    cursor?: string,
    pageSize?: number,
  ) => Promise<BlueprintHistoryPageView>;
  readonly getBlueprintPoolMembers: (
    blueprintCourseId: BlueprintCourseId,
    assessmentId: BlueprintAssessmentId,
    poolId: QuestionId,
  ) => Promise<BlueprintPoolMembersView>;
  readonly forkBlueprintCourse: (
    blueprintCourseId: BlueprintCourseId,
    revisionNumber: string,
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
    left: BlueprintCourseId,
    right: BlueprintCourseId,
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
    blueprintCourseId: BlueprintCourseId,
  ) => Promise<LoadedBlueprintCourse>;
  readonly createBlueprintCourse: (
    content: CreateBlueprintCourseInput,
    idempotencyKey: BlueprintIdempotencyKey,
  ) => Promise<LoadedBlueprintCourse>;
  readonly saveBlueprintCourse: (
    blueprintCourseId: BlueprintCourseId,
    content: ReplaceBlueprintCourseContentInput,
    etag: BlueprintRevisionEtag,
    idempotencyKey: BlueprintIdempotencyKey,
  ) => Promise<BlueprintCourseSaveResponse & { readonly revisionEtag: BlueprintRevisionEtag }>;
  readonly renameBlueprintCourse: (
    blueprintCourseId: BlueprintCourseId,
    names: RenameBlueprintCourseInput,
    etag: BlueprintEditNumber,
  ) => Promise<BlueprintMetadataTransition>;
  readonly publishBlueprintCourse: (
    blueprintCourseId: BlueprintCourseId,
    etag: BlueprintEditNumber,
  ) => Promise<BlueprintMetadataTransition>;
  readonly getBlueprintRevision: (
    blueprintCourseId: BlueprintCourseId,
    revisionNumber: string,
  ) => Promise<BlueprintRevisionView>;
  readonly archiveBlueprintCourse: (
    blueprintCourseId: BlueprintCourseId,
    confirmationLongName: string,
    etag: BlueprintEditNumber,
  ) => Promise<BlueprintMetadataTransition>;
  readonly restoreBlueprintCourse: (
    blueprintCourseId: BlueprintCourseId,
    etag: BlueprintEditNumber,
  ) => Promise<BlueprintMetadataTransition>;
  readonly returnBlueprintCourseToPrivate: (
    blueprintCourseId: BlueprintCourseId,
    etag: BlueprintEditNumber,
  ) => Promise<BlueprintMetadataTransition>;
}
