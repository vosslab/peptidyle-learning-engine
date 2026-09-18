// Browser contract for the no-write Instructor Student View.

import type { AssessmentEditNumber } from "../../generated/api/AssessmentEditNumber";
import type { AssessmentId } from "../../generated/api/AssessmentId";
import type { CourseInstanceId } from "../../generated/api/CourseInstanceId";
import type { InstructorStudentView } from "../../generated/api/InstructorStudentView";
import type { QuestionRevisionReference } from "../../generated/api/QuestionRevisionReference";
import type { StudentQuestionPresentation } from "../../generated/api/StudentQuestionPresentation";

/** Read-only preview capability with no Student, Attempt, submission, or grading operation. */
export interface AssessmentStudentViewClient {
  readonly getInstructorStudentView: (
    course: CourseInstanceId,
    assessment: AssessmentId,
  ) => Promise<InstructorStudentView>;
  readonly getInstructorStudentViewQuestion: (
    course: CourseInstanceId,
    assessment: AssessmentId,
    authoredPosition: number,
    questionRevision: QuestionRevisionReference,
    editNumber: AssessmentEditNumber,
  ) => Promise<StudentQuestionPresentation>;
  readonly instructorStudentViewQuestionDocumentUrl: (
    course: CourseInstanceId,
    assessment: AssessmentId,
    authoredPosition: number,
    questionRevision: QuestionRevisionReference,
    editNumber: AssessmentEditNumber,
  ) => string;
}
