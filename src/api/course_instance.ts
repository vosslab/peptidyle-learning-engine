// Browser capability contract for live Course Instance creation and the initial Teaching Team.

import type { AccountId } from "../../generated/api/AccountId";
import type { BlueprintCourseId } from "../../generated/api/BlueprintCourseId";
import type { BlueprintRevisionNumber } from "../../generated/api/BlueprintRevisionNumber";
import type { CourseInstanceId } from "../../generated/api/CourseInstanceId";
import type { CourseInstanceRouteSummary } from "../../generated/api/CourseInstanceRouteSummary";
import type { CourseTerm } from "../../generated/api/CourseTerm";
import type { CourseTheme } from "../../generated/api/CourseTheme";
import type { CourseClassification } from "../../generated/api/CourseClassification";
import type { CourseEditNumber } from "../../generated/api/CourseEditNumber";
import type { BlueprintCourseView } from "../../generated/api/BlueprintCourseView";
import type { CreateBlueprintFromCourseInstanceInput } from "../../generated/api/CreateBlueprintFromCourseInstanceInput";

/** Closed browser source: empty creation never names or reads a Blueprint. */
export type CourseInstanceCreationSource =
  | { readonly kind: "empty" }
  | {
      readonly kind: "adopted";
      readonly blueprintCourse: BlueprintCourseId;
      readonly blueprintRevisionNumber: BlueprintRevisionNumber;
    };

/** Exact source and initial Course Term required to create one Course Instance. */
export interface CreateCourseInstanceInput {
  readonly classification: CourseClassification;
  readonly source: CourseInstanceCreationSource;
  readonly shortName: string;
  readonly longName: string;
  readonly term: CourseTerm;
  /** Omitted for Instructor self-creation; required for a Sysadmin creation. */
  readonly assignedInstructor?: AccountId;
}

/** Browser-safe Course Instance landing-page identity. */
export type CourseInstanceLifecycleState = "active" | "inactive";

/** Browser-safe Course Instance landing-page identity. */
export interface CourseInstanceSummary {
  readonly classification: CourseClassification;
  /** Stored activity state; it is not inferred from dates or retention state. */
  readonly lifecycleState: CourseInstanceLifecycleState;
  readonly courseEditNumber: CourseEditNumber;
  readonly id: CourseInstanceId;
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
    readonly id: BlueprintCourseId;
    readonly adoptedRevisionNumber: BlueprintRevisionNumber;
    readonly currentRevisionNumber: BlueprintRevisionNumber;
  } | null;
}

/** Explicit Sysadmin selection target; it carries no email or course authority. */
export interface CourseCreationInstructor {
  readonly id: AccountId;
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
    courseInstanceId: CourseInstanceId,
    input: CreateBlueprintFromCourseInstanceInput,
    idempotencyKey: string,
  ) => Promise<CreatedBlueprintFromCourseInstance>;
  readonly updateCourseInstanceClassification: (
    courseInstanceId: CourseInstanceId,
    classification: CourseClassification,
    courseEditNumber: CourseEditNumber,
  ) => Promise<{
    readonly classification: CourseClassification;
    readonly courseEditNumber: CourseEditNumber;
    readonly changed: boolean;
  }>;
  readonly listCourseInstances: () => Promise<ReadonlyArray<CourseInstanceSummary>>;
  readonly createCourseInstance: (
    input: CreateCourseInstanceInput,
  ) => Promise<CreatedCourseInstance>;
  readonly getCourseInstance: (courseInstanceId: CourseInstanceId) => Promise<CourseInstanceView>;
  /** Reads the closed member-safe identity used by Course Instance routes. */
  readonly getCourseInstanceRouteSummary: (
    courseInstanceId: CourseInstanceId,
  ) => Promise<CourseInstanceRouteSummary>;
  readonly listCourseCreationInstructors: () => Promise<ReadonlyArray<CourseCreationInstructor>>;
}
