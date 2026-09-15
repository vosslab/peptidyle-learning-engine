// Strict same-origin transport for the no-write Instructor Student View.

import type { AssessmentReference } from "../../../generated/api/AssessmentReference";
import type { CourseInstanceReference } from "../../../generated/api/CourseInstanceReference";
import type { QuestionRevisionReference } from "../../../generated/api/QuestionRevisionReference";
import { normalizeQuestionIdSyntax } from "../../../generated/api/QuestionIdSyntaxContract";
import type { AssessmentStudentViewClient } from "../assessment_student_view";
import { decodeInstructorStudentView } from "../decoders/assessment_student_view";
import { decodeStudentQuestionPresentation } from "../decoders/presentation_delivery";
import {
  parseAssessmentReference,
  parseCourseInstanceReference,
} from "../../navigation/public_route";
import { ApiProtocolError, ApiRequestError, AssessmentConflictError } from "./error";
import { requestPath, requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

function assessmentStudentViewPath(
  course: CourseInstanceReference,
  assessment: AssessmentReference,
): string {
  if (
    parseCourseInstanceReference(course) === null ||
    parseAssessmentReference(assessment) === null
  ) {
    throw new ApiProtocolError("Student View route references must be canonical");
  }
  // ASVS 1.2.2 and 2.2.1: validate, then encode every dynamic path segment.
  return `/api/course-instances/${encodeURIComponent(course)}/assessments/${encodeURIComponent(assessment)}/student-view`;
}

function questionPath(
  course: CourseInstanceReference,
  assessment: AssessmentReference,
  authoredPosition: number,
  questionRevision: QuestionRevisionReference,
): string {
  const base = assessmentStudentViewPath(course, assessment);
  const questionId = normalizeQuestionIdSyntax(questionRevision.questionId);
  if (
    !Number.isSafeInteger(authoredPosition) ||
    authoredPosition < 0 ||
    authoredPosition > 2_147_483_647 ||
    questionId === null ||
    questionId !== questionRevision.questionId ||
    !Number.isSafeInteger(questionRevision.revisionNumber) ||
    questionRevision.revisionNumber < 1 ||
    questionRevision.revisionNumber > 2_147_483_647
  ) {
    throw new ApiProtocolError("Student View Question locator must be canonical and bounded");
  }
  return `${base}/entries/${authoredPosition}/questions/${encodeURIComponent(questionId)}/revisions/${questionRevision.revisionNumber}`;
}

function quotedEditNumber(editNumber: string, path: string): string {
  if (!/^[1-9][0-9]*$/u.test(editNumber) || BigInt(editNumber) > 9_223_372_036_854_775_807n) {
    throw new ApiProtocolError(`API ${path} requires a positive Assessment Edit Number`);
  }
  return `"${editNumber}"`;
}

function requireMatchingEtag(response: Response, editNumber: string, path: string): void {
  if (response.headers.get("etag") !== quotedEditNumber(editNumber, path)) {
    throw new ApiProtocolError(`API response ${path} ETag must match its Assessment Edit Number`);
  }
}

function sameQuestionRevision(
  received: QuestionRevisionReference,
  expected: QuestionRevisionReference,
): boolean {
  return (
    received.questionId === expected.questionId &&
    received.revisionNumber === expected.revisionNumber
  );
}

async function manifestRequest(
  fetchImplementation: ApiFetch,
  basePath: string,
  course: CourseInstanceReference,
  assessment: AssessmentReference,
): ReturnType<AssessmentStudentViewClient["getInstructorStudentView"]> {
  const path = assessmentStudentViewPath(course, assessment);
  const response = await requestSameOrigin(fetchImplementation, basePath, path);
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  // ASVS 1.5.2, 2.2.1, and 4.1.1: bounded JSON is decoded into one closed DTO.
  const manifest = decodeInstructorStudentView(await boundedResponseJson(response, path));
  requireMatchingEtag(response, manifest.editNumber, path);
  return manifest;
}

async function presentationRequest(
  fetchImplementation: ApiFetch,
  basePath: string,
  course: CourseInstanceReference,
  assessment: AssessmentReference,
  authoredPosition: number,
  questionRevision: QuestionRevisionReference,
  editNumber: string,
): ReturnType<AssessmentStudentViewClient["getInstructorStudentViewQuestion"]> {
  const path = `${questionPath(course, assessment, authoredPosition, questionRevision)}/presentation`;
  const response = await requestSameOrigin(fetchImplementation, basePath, path, {
    headers: { "if-match": quotedEditNumber(editNumber, path) },
  });
  requireNoStore(response, path);
  if (response.status === 409 || response.status === 412 || response.status === 428) {
    throw new AssessmentConflictError(response.status, path);
  }
  if (!response.ok) throw new ApiRequestError(response.status, path);
  const presentation = decodeStudentQuestionPresentation(await boundedResponseJson(response, path));
  if (!sameQuestionRevision(presentation.questionRevision, questionRevision)) {
    throw new ApiProtocolError(
      `API response ${path} does not match the requested Question Revision`,
    );
  }
  requireMatchingEtag(response, editNumber, path);
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
      questionRevision,
      editNumber,
    ) =>
      presentationRequest(
        fetchImplementation,
        basePath,
        course,
        assessment,
        authoredPosition,
        questionRevision,
        editNumber,
      ),
    instructorStudentViewQuestionDocumentUrl: (
      course,
      assessment,
      authoredPosition,
      questionRevision,
      editNumber,
    ): string => {
      const path = `${questionPath(course, assessment, authoredPosition, questionRevision)}/document`;
      const query = new URLSearchParams({ editNumber });
      quotedEditNumber(editNumber, path);
      return requestPath(basePath, `${path}?${query.toString()}`);
    },
  };
}
