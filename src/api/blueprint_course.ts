// Browser capability contract for the Blueprint lineage, Draft, and publication lifecycle.

import type { BlueprintAvailability } from "../../generated/api/BlueprintAvailability";
import type { BlueprintCourseReference } from "../../generated/api/BlueprintCourseReference";
import type { BlueprintCourseSummaryView } from "../../generated/api/BlueprintCourseSummaryView";
import type { BlueprintCourseView } from "../../generated/api/BlueprintCourseView";
import type { BlueprintRevisionReference } from "../../generated/api/BlueprintRevisionReference";
import type { BlueprintRevisionView } from "../../generated/api/BlueprintRevisionView";
import type { CreateBlueprintCourseContentInput } from "../../generated/api/CreateBlueprintCourseContentInput";
import type { ReplaceBlueprintCourseContentInput } from "../../generated/api/ReplaceBlueprintCourseContentInput";
import type { CursorPage } from "./contracts";

export type BlueprintDraftEtag = string;
export type BlueprintAvailabilityEtag = string;
export type BlueprintIdempotencyKey = string;

export interface LoadedBlueprintCourse {
  readonly blueprintCourse: BlueprintCourseView;
  /** Present exactly when the private Draft is present for its owner. */
  readonly draftEtag: BlueprintDraftEtag | undefined;
}

export interface BlueprintAvailabilityTransition {
  readonly availability: BlueprintAvailability;
  readonly editNumber: string;
  readonly etag: BlueprintAvailabilityEtag;
}

/** Browser capability for Instructor-owned reusable Blueprint Course lifecycle operations. */
export interface BlueprintCourseClient {
  readonly listBlueprintCourses: (
    cursor?: string,
    pageSize?: number,
  ) => Promise<CursorPage<BlueprintCourseSummaryView>>;
  readonly getBlueprintCourse: (
    reference: BlueprintCourseReference,
  ) => Promise<LoadedBlueprintCourse>;
  readonly createBlueprintCourse: (
    content: CreateBlueprintCourseContentInput,
    idempotencyKey: BlueprintIdempotencyKey,
  ) => Promise<LoadedBlueprintCourse>;
  readonly saveBlueprintDraft: (
    reference: BlueprintCourseReference,
    content: ReplaceBlueprintCourseContentInput,
    etag: BlueprintDraftEtag,
    idempotencyKey: BlueprintIdempotencyKey,
  ) => Promise<LoadedBlueprintCourse>;
  readonly publishBlueprintDraft: (
    reference: BlueprintCourseReference,
    etag: BlueprintDraftEtag,
    idempotencyKey: BlueprintIdempotencyKey,
  ) => Promise<BlueprintRevisionReference>;
  readonly getBlueprintRevision: (
    reference: BlueprintCourseReference,
    revision: string,
  ) => Promise<BlueprintRevisionView>;
  readonly archiveBlueprintCourse: (
    reference: BlueprintCourseReference,
    confirmationTitle: string,
    etag: BlueprintAvailabilityEtag,
  ) => Promise<BlueprintAvailabilityTransition>;
  readonly restoreBlueprintCourse: (
    reference: BlueprintCourseReference,
    etag: BlueprintAvailabilityEtag,
  ) => Promise<BlueprintAvailabilityTransition>;
}
