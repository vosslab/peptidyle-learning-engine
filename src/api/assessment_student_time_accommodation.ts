// Narrow live Instructor time-only configuration for an active roster Student.
import type { AssessmentId } from "../../generated/api/AssessmentId";
import type { CourseInstanceId } from "../../generated/api/CourseInstanceId";
import type { AssessmentStudentTimeAccommodation } from "../../generated/api/AssessmentStudentTimeAccommodation";
import type { SaveAssessmentStudentTimeAccommodationInput } from "../../generated/api/SaveAssessmentStudentTimeAccommodationInput";

export interface AssessmentStudentTimeAccommodationClient {
  readonly getAssessmentStudentTimeAccommodation: (
    course: CourseInstanceId,
    assessment: AssessmentId,
    rosterId: string,
  ) => Promise<AssessmentStudentTimeAccommodation>;
  readonly saveAssessmentStudentTimeAccommodation: (
    course: CourseInstanceId,
    assessment: AssessmentId,
    rosterId: string,
    input: SaveAssessmentStudentTimeAccommodationInput,
  ) => Promise<AssessmentStudentTimeAccommodation>;
}
