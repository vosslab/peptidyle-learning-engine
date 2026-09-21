// Browser contract for the no-write Instructor Student View.

import type { AssessmentEditNumber } from "../../generated/api/AssessmentEditNumber";
import type { AssessmentId } from "../../generated/api/AssessmentId";
import type { CourseInstanceId } from "../../generated/api/CourseInstanceId";
import type { InstructorStudentView } from "../../generated/api/InstructorStudentView";
import type { PublishedQuestionRevisionTuple } from "../../generated/api/PublishedQuestionRevisionTuple";
import type { StudentQuestionPresentation } from "../../generated/api/StudentQuestionPresentation";

/** Read-only preview capability with no Student, Attempt, submission, or grading operation. */
export interface AssessmentStudentViewClient {
  readonly getInstructorStudentView: (
    courseInstanceId: CourseInstanceId,
    assessmentId: AssessmentId,
  ) => Promise<InstructorStudentView>;
  readonly getInstructorStudentViewQuestion: (
    courseInstanceId: CourseInstanceId,
    assessmentId: AssessmentId,
    authoredPosition: number,
    publishedQuestionRevisionTuple: PublishedQuestionRevisionTuple,
    expectedAssessmentEditNumber: AssessmentEditNumber,
  ) => Promise<StudentQuestionPresentation>;
  readonly instructorStudentViewQuestionDocumentUrl: (
    courseInstanceId: CourseInstanceId,
    assessmentId: AssessmentId,
    authoredPosition: number,
    publishedQuestionRevisionTuple: PublishedQuestionRevisionTuple,
    expectedAssessmentEditNumber: AssessmentEditNumber,
  ) => string;
}
