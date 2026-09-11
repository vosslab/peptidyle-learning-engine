// Browser capability contract for live Course Instance creation and the initial Teaching Team.

import type { AccountReference } from "../../generated/api/AccountReference";
import type { BlueprintCourseReference } from "../../generated/api/BlueprintCourseReference";
import type { CourseInstanceReference } from "../../generated/api/CourseInstanceReference";
import type { CourseTerm } from "../../generated/api/CourseTerm";
import type { CourseTheme } from "../../generated/api/CourseTheme";

/** Exact source and initial Course Term required to create one Course Instance. */
export interface CreateCourseInstanceInput {
  readonly blueprintCourse: BlueprintCourseReference;
  readonly blueprintRevision: string;
  readonly title: string;
  readonly term: CourseTerm;
  /** Omitted for Instructor self-assignment; required for a Sysadmin creation. */
  readonly assignedInstructor?: AccountReference;
}

/** Browser-safe Course Instance landing-page identity. */
export interface CourseInstanceSummary {
  readonly reference: CourseInstanceReference;
  readonly title: string;
  readonly term: CourseTerm;
  /** Row identity only; it does not apply a Course theme to the product route. */
  readonly theme: CourseTheme;
}

/** Initial Teaching Team workspace projection. */
export interface CourseInstanceView {
  readonly course: CourseInstanceSummary;
  readonly isAssignedInstructor: boolean;
  readonly activeInstructorCount: number;
}

/** Explicit Sysadmin selection target; it carries no email or course authority. */
export interface CourseCreationInstructor {
  readonly reference: AccountReference;
}

/** Creation receipt that does not imply creator Course access. */
export interface CreatedCourseInstance {
  readonly course: CourseInstanceSummary;
  readonly creatorIsAssignedInstructor: boolean;
}

/** Same-origin client boundary for Course Instance creation and initial teaching team. */
export interface CourseInstanceClient {
  readonly listCourseInstances: () => Promise<ReadonlyArray<CourseInstanceSummary>>;
  readonly createCourseInstance: (
    input: CreateCourseInstanceInput,
  ) => Promise<CreatedCourseInstance>;
  readonly getCourseInstance: (reference: CourseInstanceReference) => Promise<CourseInstanceView>;
  readonly listCourseCreationInstructors: () => Promise<ReadonlyArray<CourseCreationInstructor>>;
}
