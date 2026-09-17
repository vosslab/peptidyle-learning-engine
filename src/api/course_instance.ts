// Browser capability contract for live Course Instance creation and the initial Teaching Team.

import type { AccountReference } from "../../generated/api/AccountReference";
import type { BlueprintCourseReference } from "../../generated/api/BlueprintCourseReference";
import type { BlueprintRevision } from "../../generated/api/BlueprintRevision";
import type { CourseInstanceReference } from "../../generated/api/CourseInstanceReference";
import type { CourseInstanceRouteSummary } from "../../generated/api/CourseInstanceRouteSummary";
import type { CourseTerm } from "../../generated/api/CourseTerm";
import type { CourseTheme } from "../../generated/api/CourseTheme";
import type { CourseClassification } from "../../generated/api/CourseClassification";
import type { BlueprintCourseView } from "../../generated/api/BlueprintCourseView";
import type { CreateBlueprintFromCourseInstanceInput } from "../../generated/api/CreateBlueprintFromCourseInstanceInput";

/** Closed browser source: empty creation never names or reads a Blueprint. */
export type CourseInstanceCreationSource =
  | { readonly kind: "empty" }
  | {
      readonly kind: "adopted";
      readonly blueprintCourse: BlueprintCourseReference;
      readonly blueprintRevision: BlueprintRevision;
    };

/** Exact source and initial Course Term required to create one Course Instance. */
export interface CreateCourseInstanceInput {
  readonly classification: CourseClassification;
  readonly source: CourseInstanceCreationSource;
  readonly shortName: string;
  readonly longName: string;
  readonly term: CourseTerm;
  /** Omitted for Instructor self-creation; required for a Sysadmin creation. */
  readonly assignedInstructor?: AccountReference;
}

/** Browser-safe Course Instance landing-page identity. */
export type CourseInstanceLifecycleState = "active" | "inactive";

/** Browser-safe Course Instance landing-page identity. */
export interface CourseInstanceSummary {
  readonly classification: CourseClassification;
  /** Stored activity state; it is not inferred from dates or retention state. */
  readonly lifecycleState: CourseInstanceLifecycleState;
  readonly metadataEtag: string;
  readonly reference: CourseInstanceReference;
  readonly shortName: string;
  readonly longName: string;
  readonly term: CourseTerm;
  /** Row identity only; it does not apply a Course theme to the product route. */
  readonly theme: CourseTheme;
}

/** Initial Teaching Team workspace projection. */
export interface CourseInstanceView {
  readonly course: CourseInstanceSummary;
  readonly activeInstructorCount: number;
  /** Original adoption provenance; null for Empty Courses or unreadable sources. */
  readonly blueprintOrigin: {
    readonly reference: BlueprintCourseReference;
    readonly adoptedRevision: BlueprintRevision;
    readonly currentRevision: BlueprintRevision;
  } | null;
}

/** Explicit Sysadmin selection target; it carries no email or course authority. */
export interface CourseCreationInstructor {
  readonly reference: AccountReference;
}

/** Creation receipt that does not imply creator Course access. */
export interface CreatedCourseInstance {
  readonly course: CourseInstanceSummary;
}

/** Receipt for a new private Blueprint derived from one Course Instance. */
export interface CreatedBlueprintFromCourseInstance {
  readonly blueprintCourse: BlueprintCourseView;
  /** Strong validator for the new Blueprint's initial Revision. */
  readonly revisionEtag: string;
}

/** Same-origin client boundary for Course Instance creation and initial teaching team. */
export interface CourseInstanceClient {
  /** Creates a new private Blueprint from one Course Instance's reusable structure. */
  readonly createBlueprintFromCourseInstance: (
    reference: CourseInstanceReference,
    input: CreateBlueprintFromCourseInstanceInput,
    idempotencyKey: string,
  ) => Promise<CreatedBlueprintFromCourseInstance>;
  readonly updateCourseInstanceClassification: (
    reference: CourseInstanceReference,
    classification: CourseClassification,
    metadataEtag: string,
  ) => Promise<{
    readonly classification: CourseClassification;
    readonly metadataEtag: string;
    readonly changed: boolean;
  }>;
  readonly listCourseInstances: () => Promise<ReadonlyArray<CourseInstanceSummary>>;
  readonly createCourseInstance: (
    input: CreateCourseInstanceInput,
  ) => Promise<CreatedCourseInstance>;
  readonly getCourseInstance: (reference: CourseInstanceReference) => Promise<CourseInstanceView>;
  /** Reads the closed member-safe identity used by Course Instance routes. */
  readonly getCourseInstanceRouteSummary: (
    reference: CourseInstanceReference,
  ) => Promise<CourseInstanceRouteSummary>;
  readonly listCourseCreationInstructors: () => Promise<ReadonlyArray<CourseCreationInstructor>>;
}
