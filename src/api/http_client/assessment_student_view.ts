// Strict same-origin transport for the no-write Instructor Student View.

import type { AssessmentId } from "../../../generated/api/AssessmentId";
import type { CourseInstanceId } from "../../../generated/api/CourseInstanceId";
import type { PublishedQuestionRevisionTuple } from "../../../generated/api/PublishedQuestionRevisionTuple";
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

function assessmentStudentViewPath(
  courseInstanceId: CourseInstanceId,
  assessmentId: AssessmentId,
): string {
  if (
    parseCourseInstanceId(courseInstanceId) === null ||
    parseAssessmentId(assessmentId) === null
  ) {
    throw new ApiProtocolError("Student View route IDs must be canonical");
  }
  // ASVS 1.2.2 and 2.2.1: validate, then encode every dynamic path segment.
  return `/api/course-instances/${encodeURIComponent(courseInstanceId)}/assessments/${encodeURIComponent(assessmentId)}/student-view`;
}

function questionPath(
  courseInstanceId: CourseInstanceId,
  assessmentId: AssessmentId,
  authoredPosition: number,
  publishedQuestionRevisionTuple: PublishedQuestionRevisionTuple,
): string {
  const base = assessmentStudentViewPath(courseInstanceId, assessmentId);
  const questionId = validateCanonicalQuestionIdSyntax(
    publishedQuestionRevisionTuple.publishedQuestionId,
  );
  if (
    !Number.isSafeInteger(authoredPosition) ||
    authoredPosition < 0 ||
    authoredPosition > 2_147_483_647 ||
    questionId === null ||
    questionId !== publishedQuestionRevisionTuple.publishedQuestionId ||
    !Number.isSafeInteger(publishedQuestionRevisionTuple.revisionNumber) ||
    publishedQuestionRevisionTuple.revisionNumber < 1 ||
    publishedQuestionRevisionTuple.revisionNumber > 2_147_483_647
  ) {
    throw new ApiProtocolError("Student View Question locator must be canonical and bounded");
  }
  return `${base}/entries/${authoredPosition}/questions/${encodeURIComponent(questionId)}/revisions/${publishedQuestionRevisionTuple.revisionNumber}`;
}

function requireMatchingAssessmentEditNumber(
  response: Response,
  assessmentEditNumber: string,
  path: string,
): void {
  assertResponseMatchesPositiveNumber(
    response,
    assessmentEditNumber,
    path,
    "Assessment Edit Number",
  );
}

function sameQuestionRevision(
  received: PublishedQuestionRevisionTuple,
  expected: PublishedQuestionRevisionTuple,
): boolean {
  return (
    received.publishedQuestionId === expected.publishedQuestionId &&
    received.revisionNumber === expected.revisionNumber
  );
}

async function manifestRequest(
  fetchImplementation: ApiFetch,
  basePath: string,
  courseInstanceId: CourseInstanceId,
  assessmentId: AssessmentId,
): ReturnType<AssessmentStudentViewClient["getInstructorStudentView"]> {
  const path = assessmentStudentViewPath(courseInstanceId, assessmentId);
  const response = await requestSameOrigin(fetchImplementation, basePath, path);
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  // ASVS 1.5.2, 2.2.1, and 4.1.1: bounded JSON is decoded into one closed DTO.
  const manifest = decodeInstructorStudentView(await boundedResponseJson(response, path));
  requireMatchingAssessmentEditNumber(response, manifest.assessmentEditNumber, path);
  return manifest;
}

async function presentationRequest(
  fetchImplementation: ApiFetch,
  basePath: string,
  courseInstanceId: CourseInstanceId,
  assessmentId: AssessmentId,
  authoredPosition: number,
  publishedQuestionRevisionTuple: PublishedQuestionRevisionTuple,
  expectedAssessmentEditNumber: string,
): ReturnType<AssessmentStudentViewClient["getInstructorStudentViewQuestion"]> {
  const path = `${questionPath(courseInstanceId, assessmentId, authoredPosition, publishedQuestionRevisionTuple)}/presentation`;
  const response = await requestSameOrigin(fetchImplementation, basePath, path, {
    headers: {
      "if-match": ifMatchHeaderForPositiveNumber(
        expectedAssessmentEditNumber,
        path,
        "Assessment Edit Number",
      ),
    },
  });
  requireNoStore(response, path);
  if (response.status === 409 || response.status === 412 || response.status === 428) {
    throw new AssessmentConflictError(response.status, path);
  }
  if (!response.ok) throw new ApiRequestError(response.status, path);
  const presentation = decodeStudentQuestionPresentation(await boundedResponseJson(response, path));
  if (
    !sameQuestionRevision(
      presentation.publishedQuestionRevisionTuple,
      publishedQuestionRevisionTuple,
    )
  ) {
    throw new ApiProtocolError(
      `API response ${path} does not match the requested Question Revision`,
    );
  }
  requireMatchingAssessmentEditNumber(response, expectedAssessmentEditNumber, path);
  return presentation;
}

/** Creates the read-only Student View transport; it exposes no mutation method. */
export function createAssessmentStudentViewClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): AssessmentStudentViewClient {
  return {
    getInstructorStudentView: (courseInstanceId, assessmentId) =>
      manifestRequest(fetchImplementation, basePath, courseInstanceId, assessmentId),
    getInstructorStudentViewQuestion: (
      courseInstanceId,
      assessmentId,
      authoredPosition,
      publishedQuestionRevisionTuple,
      expectedAssessmentEditNumber,
    ) =>
      presentationRequest(
        fetchImplementation,
        basePath,
        courseInstanceId,
        assessmentId,
        authoredPosition,
        publishedQuestionRevisionTuple,
        expectedAssessmentEditNumber,
      ),
    instructorStudentViewQuestionDocumentUrl: (
      courseInstanceId,
      assessmentId,
      authoredPosition,
      publishedQuestionRevisionTuple,
      expectedAssessmentEditNumber,
    ): string => {
      const path = `${questionPath(courseInstanceId, assessmentId, authoredPosition, publishedQuestionRevisionTuple)}/document`;
      ifMatchHeaderForPositiveNumber(expectedAssessmentEditNumber, path, "Assessment Edit Number");
      const query = new URLSearchParams({ assessmentEditNumber: expectedAssessmentEditNumber });
      return requestPath(basePath, `${path}?${query.toString()}`);
    },
  };
}
