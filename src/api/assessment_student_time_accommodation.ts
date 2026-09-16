// Narrow live Instructor time-only configuration for an active roster Student.
import type { AssessmentReference } from "../../generated/api/AssessmentReference";
import type { CourseInstanceReference } from "../../generated/api/CourseInstanceReference";
import type { AssessmentStudentTimeAccommodation } from "../../generated/api/AssessmentStudentTimeAccommodation";
import type { SaveAssessmentStudentTimeAccommodationInput } from "../../generated/api/SaveAssessmentStudentTimeAccommodationInput";

export interface AssessmentStudentTimeAccommodationClient {
  readonly getAssessmentStudentTimeAccommodation: (
    course: CourseInstanceReference,
    assessment: AssessmentReference,
    rosterId: string,
  ) => Promise<AssessmentStudentTimeAccommodation>;
  readonly saveAssessmentStudentTimeAccommodation: (
    course: CourseInstanceReference,
    assessment: AssessmentReference,
    rosterId: string,
    input: SaveAssessmentStudentTimeAccommodationInput,
  ) => Promise<AssessmentStudentTimeAccommodation>;
}
