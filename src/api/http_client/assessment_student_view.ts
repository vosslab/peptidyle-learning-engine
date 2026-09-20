// Strict same-origin transport for the no-write Instructor Student View.

import type { AssessmentId } from "../../../generated/api/AssessmentId";
import type { CourseInstanceId } from "../../../generated/api/CourseInstanceId";
import type { QuestionRevisionTuple } from "../../../generated/api/QuestionRevisionTuple";
import { validateCanonicalQuestionIdSyntax } from "../../../generated/api/QuestionIdSyntaxContract";
import type { AssessmentStudentViewClient } from "../assessment_student_view";
import { decodeInstructorStudentView } from "../decoders/assessment_student_view";
import { decodeStudentQuestionPresentation } from "../decoders/presentation_delivery";
import { parseAssessmentId, parseCourseInstanceId } from "../../navigation/public_route";
import { ApiProtocolError, ApiRequestError, AssessmentConflictError } from "./error";
import {
  assertResponseMatchesPositiveNumber,
  ifMatchHeaderForPositiveNumber,
} from "./conditional_request";
import { requestPath, requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

function assessmentStudentViewPath(course: CourseInstanceId, assessment: AssessmentId): string {
  if (parseCourseInstanceId(course) === null || parseAssessmentId(assessment) === null) {
    throw new ApiProtocolError("Student View route IDs must be canonical");
  }
  // ASVS 1.2.2 and 2.2.1: validate, then encode every dynamic path segment.
  return `/api/course-instances/${encodeURIComponent(course)}/assessments/${encodeURIComponent(assessment)}/student-view`;
}

function questionPath(
  course: CourseInstanceId,
  assessment: AssessmentId,
  authoredPosition: number,
  questionRevisionTuple: QuestionRevisionTuple,
): string {
  const base = assessmentStudentViewPath(course, assessment);
  const questionId = validateCanonicalQuestionIdSyntax(questionRevisionTuple.questionId);
  if (
    !Number.isSafeInteger(authoredPosition) ||
    authoredPosition < 0 ||
    authoredPosition > 2_147_483_647 ||
    questionId === null ||
    questionId !== questionRevisionTuple.questionId ||
    !Number.isSafeInteger(questionRevisionTuple.revisionNumber) ||
    questionRevisionTuple.revisionNumber < 1 ||
    questionRevisionTuple.revisionNumber > 2_147_483_647
  ) {
    throw new ApiProtocolError("Student View Question locator must be canonical and bounded");
  }
  return `${base}/entries/${authoredPosition}/questions/${encodeURIComponent(questionId)}/revisions/${questionRevisionTuple.revisionNumber}`;
}

function requireMatchingAssessmentEditNumber(
  response: Response,
  editNumber: string,
  path: string,
): void {
  assertResponseMatchesPositiveNumber(response, editNumber, path, "Assessment Edit Number");
}

function sameQuestionRevision(
  received: QuestionRevisionTuple,
  expected: QuestionRevisionTuple,
): boolean {
  return (
    received.questionId === expected.questionId &&
    received.revisionNumber === expected.revisionNumber
  );
}

async function manifestRequest(
  fetchImplementation: ApiFetch,
  basePath: string,
  course: CourseInstanceId,
  assessment: AssessmentId,
): ReturnType<AssessmentStudentViewClient["getInstructorStudentView"]> {
  const path = assessmentStudentViewPath(course, assessment);
  const response = await requestSameOrigin(fetchImplementation, basePath, path);
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  // ASVS 1.5.2, 2.2.1, and 4.1.1: bounded JSON is decoded into one closed DTO.
  const manifest = decodeInstructorStudentView(await boundedResponseJson(response, path));
  requireMatchingAssessmentEditNumber(response, manifest.editNumber, path);
  return manifest;
}

async function presentationRequest(
  fetchImplementation: ApiFetch,
  basePath: string,
  course: CourseInstanceId,
  assessment: AssessmentId,
  authoredPosition: number,
  questionRevisionTuple: QuestionRevisionTuple,
  editNumber: string,
): ReturnType<AssessmentStudentViewClient["getInstructorStudentViewQuestion"]> {
  const path = `${questionPath(course, assessment, authoredPosition, questionRevisionTuple)}/presentation`;
  const response = await requestSameOrigin(fetchImplementation, basePath, path, {
    headers: {
      "if-match": ifMatchHeaderForPositiveNumber(editNumber, path, "Assessment Edit Number"),
    },
  });
  requireNoStore(response, path);
  if (response.status === 409 || response.status === 412 || response.status === 428) {
    throw new AssessmentConflictError(response.status, path);
  }
  if (!response.ok) throw new ApiRequestError(response.status, path);
  const presentation = decodeStudentQuestionPresentation(await boundedResponseJson(response, path));
  if (!sameQuestionRevision(presentation.questionRevisionTuple, questionRevisionTuple)) {
    throw new ApiProtocolError(
      `API response ${path} does not match the requested Question Revision`,
    );
  }
  requireMatchingAssessmentEditNumber(response, editNumber, path);
  return presentation;
}

/** Creates the read-only Student View transport; it exposes no mutation method. */
export function createAssessmentStudentViewClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): AssessmentStudentViewClient {
  return {
    getInstructorStudentView: (course, assessment) =>
      manifestRequest(fetchImplementation, basePath, course, assessment),
    getInstructorStudentViewQuestion: (
      course,
      assessment,
      authoredPosition,
      questionRevisionTuple,
      editNumber,
    ) =>
      presentationRequest(
        fetchImplementation,
        basePath,
        course,
        assessment,
        authoredPosition,
        questionRevisionTuple,
        editNumber,
      ),
    instructorStudentViewQuestionDocumentUrl: (
      course,
      assessment,
      authoredPosition,
      questionRevisionTuple,
      editNumber,
    ): string => {
      const path = `${questionPath(course, assessment, authoredPosition, questionRevisionTuple)}/document`;
      ifMatchHeaderForPositiveNumber(editNumber, path, "Assessment Edit Number");
      const query = new URLSearchParams({ editNumber });
      return requestPath(basePath, `${path}?${query.toString()}`);
    },
  };
}
