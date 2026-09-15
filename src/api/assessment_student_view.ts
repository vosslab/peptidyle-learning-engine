// Browser contract for the no-write Instructor Student View.

import type { AssessmentEditNumber } from "../../generated/api/AssessmentEditNumber";
import type { AssessmentReference } from "../../generated/api/AssessmentReference";
import type { CourseInstanceReference } from "../../generated/api/CourseInstanceReference";
import type { InstructorStudentView } from "../../generated/api/InstructorStudentView";
import type { QuestionRevisionReference } from "../../generated/api/QuestionRevisionReference";
import type { StudentQuestionPresentation } from "../../generated/api/StudentQuestionPresentation";

/** Read-only preview capability with no Student, Attempt, submission, or grading operation. */
export interface AssessmentStudentViewClient {
  readonly getInstructorStudentView: (
    course: CourseInstanceReference,
    assessment: AssessmentReference,
  ) => Promise<InstructorStudentView>;
  readonly getInstructorStudentViewQuestion: (
    course: CourseInstanceReference,
    assessment: AssessmentReference,
    authoredPosition: number,
    questionRevision: QuestionRevisionReference,
    editNumber: AssessmentEditNumber,
  ) => Promise<StudentQuestionPresentation>;
  readonly instructorStudentViewQuestionDocumentUrl: (
    course: CourseInstanceReference,
    assessment: AssessmentReference,
    authoredPosition: number,
    questionRevision: QuestionRevisionReference,
    editNumber: AssessmentEditNumber,
  ) => string;
}
