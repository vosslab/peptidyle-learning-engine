import type { AssignmentId } from "../../../generated/api/AssignmentId";
import type { QuestionDetails } from "../../../generated/api/QuestionDetails";
import type { QuestionSummary } from "../../../generated/api/QuestionSummary";
import type { QuestionSearchPage } from "../../../generated/api/QuestionSearchPage";
import type { QuestionSearchRequest } from "../../../generated/api/QuestionSearchRequest";
import type { CourseAppearanceView } from "../../../generated/api/CourseAppearanceView";
import type { CourseThemeUpdate } from "../../../generated/api/CourseThemeUpdate";
import type { CourseBannerUpdate } from "../../../generated/api/CourseBannerUpdate";
import type { CourseBannerUploadReceipt } from "../../../generated/api/CourseBannerUploadReceipt";
import type { CourseGradeSchemeView } from "../../../generated/api/CourseGradeSchemeView";
import type { CourseGradebookTotalsView } from "../../../generated/api/CourseGradebookTotalsView";
import type { CourseId } from "../../../generated/api/CourseId";
import type { CourseBannerReference } from "../../../generated/api/CourseBannerReference";
import type { StudentRecordId } from "../../../generated/api/StudentRecordId";
import type { QuestionId } from "../../../generated/api/QuestionId";
import type { QuestionAttemptId } from "../../../generated/api/QuestionAttemptId";
import type { ApiClient } from "../client";
import type {
  InstructorProfile,
  InstructorProfileThumbnail,
  UpdateInstructorProfileInput,
} from "../instructor_profile";
import type { StudentQuestionAttempt } from "../contracts";
import { questionReferencePath, questionSearchPath } from "../question_search_query";
import {
  decodeStudentAssignmentPage,
  decodeAssignmentAttempt,
  decodeStudentAssignmentDetail,
  decodeAttemptPage,
  decodeQuestionPage,
  decodeQuestionDetails,
  decodeQuestionSummary,
  decodeQuestionSearchPage,
  decodeCourseAppearanceView,
  decodeCourseThemeUpdate,
  decodeCourseBannerUpdate,
  decodeCourseBannerUploadReceipt,
  decodeCourseGradeSchemeView,
  decodeCourseGradebookTotalsView,
  decodeCoursePage,
  decodeCourseSummary,
  decodeImathasQuestionBackendLaunch,
  decodeStudentQuestionAttempt,
  decodeIssuedQuestionPresentation,
  decodeAssignmentAttemptPage,
  decodeStudentAssignmentProgress,
  decodeNavigationResolution,
} from "../decoders";
import {
  decodeInstructorProfile,
  decodeInstructorProfileThumbnail,
  decodeUpdateInstructorProfileInput,
} from "../decoders/instructor_profile";
import { ApiProtocolError, ApiRequestError } from "./error";
import {
  encodedId,
  cursorPath,
  studentAttemptPath,
  requestJson,
  requestPath,
  type ApiFetch,
} from "./request";

export const MAX_RESPONSE_CHARACTERS = 4 * 1_024 * 1_024;

const MAX_COURSE_BANNER_DELIVERY_BYTES = 2 * 1_024 * 1_024;

async function fetchCourseBanner(
  fetchImplementation: ApiFetch,
  basePath: string,
  bannerReference: CourseBannerReference,
): Promise<Blob> {
  return fetchCourseBannerRendition(fetchImplementation, basePath, bannerReference, "hero");
}

async function fetchCourseBannerCard(
  fetchImplementation: ApiFetch,
  basePath: string,
  bannerReference: CourseBannerReference,
): Promise<Blob> {
  return fetchCourseBannerRendition(fetchImplementation, basePath, bannerReference, "card");
}

async function fetchCourseBannerRendition(
  fetchImplementation: ApiFetch,
  basePath: string,
  bannerReference: CourseBannerReference,
  rendition: "hero" | "card",
): Promise<Blob> {
  const path =
    rendition === "hero"
      ? `/api/course-banners/${encodedId(bannerReference)}/delivery`
      : `/api/course-banners/${encodedId(bannerReference)}/delivery/card`;
  const response = await fetchImplementation(requestPath(basePath, path), {
    method: "POST",
    headers: { accept: "image/webp" },
    credentials: "same-origin",
    cache: "no-store",
  });
  if (!response.ok) throw new ApiRequestError(response.status, path);
  requireNoStore(response, path);
  // ASVS 3.2.1, 3.4.4, 4.1.1, and 14.3.2: accept only the closed normalized
  // banner response and reject cache, sniffing, or cross-origin policy drift.
  if (response.headers.get("content-type") !== "image/webp")
    throw new ApiProtocolError(`API response ${path} must be normalized image/webp`);
  if (
    response.headers.get("content-disposition") !== 'attachment; filename="ple-course-banner.webp"'
  )
    throw new ApiProtocolError(
      `API response ${path} must use the protected Course Banner Content-Disposition header`,
    );
  if (response.headers.get("x-content-type-options") !== "nosniff")
    throw new ApiProtocolError(`API response ${path} must prevent content sniffing`);
  if (response.headers.get("cross-origin-resource-policy") !== "same-origin")
    throw new ApiProtocolError(`API response ${path} must remain same-origin`);
  if (response.headers.get("referrer-policy") !== "no-referrer")
    throw new ApiProtocolError(`API response ${path} must suppress referrers`);
  const contentLength = response.headers.get("content-length");
  if (contentLength === null || !/^[1-9][0-9]*$/u.test(contentLength))
    throw new ApiProtocolError(`API response ${path} must include a positive Content-Length`);
  const expectedBytes = Number(contentLength);
  if (!Number.isSafeInteger(expectedBytes) || expectedBytes > MAX_COURSE_BANNER_DELIVERY_BYTES)
    throw new ApiProtocolError(`API response ${path} exceeds the course banner byte limit`);
  const blob = await response.blob();
  if (blob.type !== "image/webp" || blob.size !== expectedBytes)
    throw new ApiProtocolError(`API response ${path} body does not match its banner metadata`);
  return blob;
}

async function issuedQuestionForAttempt(
  fetchImplementation: ApiFetch,
  basePath: string,
  courseId: CourseId,
  assignmentId: AssignmentId,
  attempt: StudentQuestionAttempt,
): Promise<import("../../../generated/api/QuestionPresentation").QuestionPresentation> {
  const path = `${studentAttemptPath(courseId, assignmentId, attempt.id)}/question`;
  return requestJson(fetchImplementation, basePath, path, decodeIssuedQuestionPresentation);
}

export function requireNoStore(response: Response, path: string): void {
  const directives =
    response.headers
      .get("cache-control")
      ?.split(",")
      .map((directive) => directive.trim().toLowerCase()) ?? [];
  if (!directives.includes("no-store"))
    throw new ApiProtocolError(`API response ${path} must be no-store`);
}
export function decodeJson(text: string, path: string): unknown {
  try {
    return JSON.parse(text);
  } catch (error: unknown) {
    const detail = error instanceof Error ? error.message : "invalid JSON";
    throw new ApiProtocolError(`API response ${path} is not valid JSON: ${detail}`);
  }
}
export function responseContentType(response: Response, path: string): void {
  const contentType = response.headers.get("content-type");
  if (contentType === null || !contentType.toLowerCase().includes("application/json"))
    throw new ApiProtocolError(`API response ${path} must use application/json`);
}
export async function boundedResponseJson(response: Response, path: string): Promise<unknown> {
  responseContentType(response, path);
  const text = await response.text();
  if (text.length === 0 || text.length > MAX_RESPONSE_CHARACTERS)
    throw new ApiProtocolError(`API response ${path} must contain bounded JSON`);
  return decodeJson(text, path);
}

function courseAppearanceViewPath(courseId: CourseId): string {
  return `/api/courses/${encodedId(courseId)}/appearance`;
}

async function instructorProfile(
  fetchImplementation: ApiFetch,
  basePath: string,
): Promise<InstructorProfile> {
  const path = "/api/instructor-profile";
  const response = await fetchImplementation(requestPath(basePath, path), {
    headers: { accept: "application/json" },
    credentials: "same-origin",
    cache: "no-store",
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  return decodeInstructorProfile(await boundedResponseJson(response, path));
}

async function saveInstructorProfile(
  fetchImplementation: ApiFetch,
  basePath: string,
  input: UpdateInstructorProfileInput,
): Promise<InstructorProfile> {
  const path = "/api/instructor-profile";
  const request = decodeUpdateInstructorProfileInput(input, "request");
  const response = await fetchImplementation(requestPath(basePath, path), {
    method: "PATCH",
    headers: { accept: "application/json", "content-type": "application/json" },
    body: JSON.stringify(request),
    credentials: "same-origin",
    cache: "no-store",
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  return decodeInstructorProfile(await boundedResponseJson(response, path));
}

async function instructorProfileThumbnail(
  fetchImplementation: ApiFetch,
  basePath: string,
): Promise<InstructorProfileThumbnail> {
  const path = "/api/instructor-profile/thumbnail";
  const response = await fetchImplementation(requestPath(basePath, path), {
    headers: { accept: "application/json" },
    credentials: "same-origin",
    cache: "no-store",
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  return decodeInstructorProfileThumbnail(await boundedResponseJson(response, path));
}

async function replaceInstructorProfileThumbnail(
  fetchImplementation: ApiFetch,
  basePath: string,
  image: Blob,
): Promise<InstructorProfileThumbnail> {
  const path = "/api/instructor-profile/thumbnail";
  const response = await fetchImplementation(requestPath(basePath, path), {
    method: "POST",
    headers: { accept: "application/json", "content-type": "application/octet-stream" },
    body: image,
    credentials: "same-origin",
    cache: "no-store",
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  return decodeInstructorProfileThumbnail(await boundedResponseJson(response, path));
}

async function fetchInstructorProfileThumbnail(
  fetchImplementation: ApiFetch,
  basePath: string,
  reference: string,
): Promise<Blob> {
  const path = `/api/instructor-profile/thumbnails/${encodedId(reference)}/delivery`;
  const response = await fetchImplementation(requestPath(basePath, path), {
    method: "POST",
    headers: { accept: "image/webp" },
    credentials: "same-origin",
    cache: "no-store",
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  if (
    response.headers.get("content-type") !== "image/webp" ||
    response.headers.get("x-content-type-options") !== "nosniff"
  )
    throw new ApiProtocolError(`API response ${path} must be a protected WebP thumbnail`);
  return response.blob();
}
async function courseAppearanceView(
  fetchImplementation: ApiFetch,
  basePath: string,
  courseId: CourseId,
): Promise<CourseAppearanceView> {
  const path = courseAppearanceViewPath(courseId);
  const response = await fetchImplementation(requestPath(basePath, path), {
    headers: { accept: "application/json" },
    credentials: "same-origin",
    cache: "no-store",
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  return decodeCourseAppearanceView(await boundedResponseJson(response, path));
}
async function updateCourseTheme(
  fetchImplementation: ApiFetch,
  basePath: string,
  courseId: CourseId,
  update: CourseThemeUpdate,
): Promise<CourseAppearanceView> {
  const path = courseAppearanceViewPath(courseId);
  // Decode at the browser boundary before dispatch so an unknown theme is
  // refused locally and can never silently become the default palette.
  const request = decodeCourseThemeUpdate(update, "request");
  const response = await fetchImplementation(requestPath(basePath, path), {
    method: "PUT",
    headers: { accept: "application/json", "content-type": "application/json" },
    body: JSON.stringify(request),
    credentials: "same-origin",
    cache: "no-store",
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  return decodeCourseAppearanceView(await boundedResponseJson(response, path));
}

async function uploadCourseBanner(
  fetchImplementation: ApiFetch,
  basePath: string,
  courseId: CourseId,
  image: Blob,
): Promise<CourseBannerUploadReceipt> {
  const path = `${courseAppearanceViewPath(courseId)}/banner-uploads`;
  const response = await fetchImplementation(requestPath(basePath, path), {
    method: "POST",
    headers: { accept: "application/json", "content-type": "application/octet-stream" },
    body: image,
    credentials: "same-origin",
    cache: "no-store",
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  return decodeCourseBannerUploadReceipt(await boundedResponseJson(response, path));
}

async function setCourseBanner(
  fetchImplementation: ApiFetch,
  basePath: string,
  courseId: CourseId,
  update: CourseBannerUpdate,
): Promise<CourseAppearanceView> {
  const path = `${courseAppearanceViewPath(courseId)}/banner`;
  const request = decodeCourseBannerUpdate(update, "request");
  const response = await fetchImplementation(requestPath(basePath, path), {
    method: "PUT",
    headers: { accept: "application/json", "content-type": "application/json" },
    body: JSON.stringify(request),
    credentials: "same-origin",
    cache: "no-store",
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  return decodeCourseAppearanceView(await boundedResponseJson(response, path));
}

async function removeCourseBanner(
  fetchImplementation: ApiFetch,
  basePath: string,
  courseId: CourseId,
): Promise<CourseAppearanceView> {
  const path = `${courseAppearanceViewPath(courseId)}/banner`;
  const response = await fetchImplementation(requestPath(basePath, path), {
    method: "DELETE",
    headers: { accept: "application/json" },
    credentials: "same-origin",
    cache: "no-store",
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  return decodeCourseAppearanceView(await boundedResponseJson(response, path));
}
async function questionDetails(
  fetchImplementation: ApiFetch,
  basePath: string,
  questionId: QuestionId,
): Promise<QuestionDetails> {
  const path = `/api/questions/by-id/${encodedId(questionId)}/detail`;
  const detail = await requestJson(fetchImplementation, basePath, path, decodeQuestionDetails);
  if (detail.summary.questionId !== questionId)
    throw new ApiProtocolError(
      "Question Details identity does not match its requested immutable version",
    );
  return detail;
}
export function createResponseClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<
  ApiClient,
  | "resolveNavigation"
  | "getInstructorProfile"
  | "updateInstructorProfile"
  | "getInstructorProfileThumbnail"
  | "replaceInstructorProfileThumbnail"
  | "fetchInstructorProfileThumbnail"
  | "listQuestions"
  | "searchQuestionLibrary"
  | "resolveQuestion"
  | "getQuestionDetails"
  | "listCourses"
  | "getCourse"
  | "getCourseAppearanceView"
  | "updateCourseTheme"
  | "uploadCourseBanner"
  | "setCourseBanner"
  | "removeCourseBanner"
  | "getCourseGradeScheme"
  | "getCourseGradebookTotals"
  | "listAssignments"
  | "getAssignment"
  | "getAssignmentSummary"
  | "listAssignmentAttempts"
  | "getAssignmentAttempt"
  | "listQuestionAttempts"
  | "getAttempt"
  | "getIssuedQuestion"
  | "beginImathasQuestionBackendLaunch"
  | "getAssignmentActivitySummary"
  | "fetchCourseBanner"
  | "fetchCourseBannerCard"
  | "assetUrl"
> {
  return {
    getInstructorProfile: () => instructorProfile(fetchImplementation, basePath),
    updateInstructorProfile: (input) => saveInstructorProfile(fetchImplementation, basePath, input),
    getInstructorProfileThumbnail: () => instructorProfileThumbnail(fetchImplementation, basePath),
    replaceInstructorProfileThumbnail: (image) =>
      replaceInstructorProfileThumbnail(fetchImplementation, basePath, image),
    fetchInstructorProfileThumbnail: (reference) =>
      fetchInstructorProfileThumbnail(fetchImplementation, basePath, reference),
    resolveNavigation: (reference) =>
      requestJson(
        fetchImplementation,
        basePath,
        `/api/navigation/${encodedId(reference)}`,
        decodeNavigationResolution,
      ),
    listQuestions: (cursor) =>
      requestJson(
        fetchImplementation,
        basePath,
        cursorPath("/api/questions", cursor),
        decodeQuestionPage,
      ),
    searchQuestionLibrary: (query: QuestionSearchRequest): Promise<QuestionSearchPage> =>
      requestJson(
        fetchImplementation,
        basePath,
        questionSearchPath(query),
        decodeQuestionSearchPage,
      ),
    resolveQuestion: (displayReference: string): Promise<QuestionSummary> => {
      const path = questionReferencePath(displayReference);
      return requestJson(fetchImplementation, basePath, path, (value, decoderPath) =>
        decodeQuestionSummary(value, decoderPath, true),
      );
    },
    getQuestionDetails: (questionId) => questionDetails(fetchImplementation, basePath, questionId),
    listCourses: (cursor) =>
      requestJson(
        fetchImplementation,
        basePath,
        cursorPath("/api/courses", cursor),
        decodeCoursePage,
      ),
    getCourse: (courseId) =>
      requestJson(
        fetchImplementation,
        basePath,
        `/api/courses/${encodedId(courseId)}`,
        decodeCourseSummary,
      ),
    getCourseAppearanceView: (courseId) =>
      courseAppearanceView(fetchImplementation, basePath, courseId),
    updateCourseTheme: (courseId, update) =>
      updateCourseTheme(fetchImplementation, basePath, courseId, update),
    uploadCourseBanner: (courseId, image) =>
      uploadCourseBanner(fetchImplementation, basePath, courseId, image),
    setCourseBanner: (courseId, update) =>
      setCourseBanner(fetchImplementation, basePath, courseId, update),
    removeCourseBanner: (courseId) => removeCourseBanner(fetchImplementation, basePath, courseId),
    getCourseGradeScheme: async (
      courseId,
    ): Promise<CourseGradeSchemeView & { readonly revision: string }> => {
      const path = `/api/courses/${encodedId(courseId)}/grade-scheme`;
      const response = await fetchImplementation(requestPath(basePath, path), {
        headers: { accept: "application/json" },
        credentials: "same-origin",
        cache: "no-store",
      });
      requireNoStore(response, path);
      if (!response.ok) throw new ApiRequestError(response.status, path);
      const scheme = decodeCourseGradeSchemeView(await boundedResponseJson(response, path));
      const revision = response.headers.get("etag");
      if (
        revision === null ||
        !/^"[1-9][0-9]*"$/u.test(revision) ||
        BigInt(revision.slice(1, -1)) > 9_223_372_036_854_775_807n
      )
        throw new ApiProtocolError(
          `API response ${path} must include one positive strong numeric ETag`,
        );
      return { ...scheme, revision };
    },
    getCourseGradebookTotals: async (courseId): Promise<CourseGradebookTotalsView> => {
      const path = `/api/courses/${encodedId(courseId)}/gradebook-totals`;
      const response = await fetchImplementation(requestPath(basePath, path), {
        headers: { accept: "application/json" },
        credentials: "same-origin",
        cache: "no-store",
      });
      requireNoStore(response, path);
      if (!response.ok) throw new ApiRequestError(response.status, path);
      return decodeCourseGradebookTotalsView(await boundedResponseJson(response, path));
    },
    listAssignments: (courseId, cursor) =>
      requestJson(
        fetchImplementation,
        basePath,
        cursorPath(`/api/courses/${encodedId(courseId)}/assignments`, cursor),
        decodeStudentAssignmentPage,
      ),
    getAssignment: (assignmentId: AssignmentId) =>
      requestJson(
        fetchImplementation,
        basePath,
        `/api/assignments/${encodedId(assignmentId)}/student`,
        decodeStudentAssignmentDetail,
      ),
    getAssignmentSummary: (assignmentId: AssignmentId) =>
      requestJson(
        fetchImplementation,
        basePath,
        `/api/assignments/${encodedId(assignmentId)}/summary`,
        decodeStudentAssignmentProgress,
      ),
    listAssignmentAttempts: (studentRecordId: StudentRecordId, cursor) =>
      requestJson(
        fetchImplementation,
        basePath,
        cursorPath(
          `/api/student-records/${encodedId(studentRecordId)}/assignment-attempts`,
          cursor,
        ),
        decodeAssignmentAttemptPage,
      ),
    getAssignmentAttempt: (assignmentAttemptId) =>
      requestJson(
        fetchImplementation,
        basePath,
        `/api/assignment-attempts/${encodedId(assignmentAttemptId)}`,
        decodeAssignmentAttempt,
      ),
    listQuestionAttempts: (assignmentAttemptId, cursor) =>
      requestJson(
        fetchImplementation,
        basePath,
        cursorPath(
          `/api/assignment-attempts/${encodedId(assignmentAttemptId)}/question-attempts`,
          cursor,
        ),
        decodeAttemptPage,
      ),
    getAttempt: (attemptId: QuestionAttemptId) =>
      requestJson(
        fetchImplementation,
        basePath,
        `/api/attempts/${encodedId(attemptId)}`,
        decodeStudentQuestionAttempt,
      ),
    getIssuedQuestion: async (
      courseId,
      assignmentId,
      attemptId,
    ): Promise<import("../../../generated/api/QuestionPresentation").QuestionPresentation> => {
      const attempt = await requestJson(
        fetchImplementation,
        basePath,
        `/api/attempts/${encodedId(attemptId)}`,
        decodeStudentQuestionAttempt,
      );
      return issuedQuestionForAttempt(
        fetchImplementation,
        basePath,
        courseId,
        assignmentId,
        attempt,
      );
    },
    beginImathasQuestionBackendLaunch: (courseId, assignmentId, attemptId) =>
      requestJson(
        fetchImplementation,
        basePath,
        `${studentAttemptPath(courseId, assignmentId, attemptId)}/imathas-question-backend/launch`,
        (value, path = "response") =>
          decodeImathasQuestionBackendLaunch(value, path, courseId, assignmentId, attemptId),
        { method: "POST" },
      ),
    getAssignmentActivitySummary: (studentRecordId) =>
      requestJson(
        fetchImplementation,
        basePath,
        `/api/student-records/${encodedId(studentRecordId)}/assignment-activity-summary`,
        decodeStudentAssignmentProgress,
      ),
    fetchCourseBanner: (bannerReference) =>
      fetchCourseBanner(fetchImplementation, basePath, bannerReference),
    fetchCourseBannerCard: (bannerReference) =>
      fetchCourseBannerCard(fetchImplementation, basePath, bannerReference),
    assetUrl: (questionRevision, assetId) =>
      requestPath(
        basePath,
        `/api/questions/${encodedId(questionRevision.questionId)}/revisions/${questionRevision.revisionNumber}/assets/${encodedId(assetId)}`,
      ),
  };
}
