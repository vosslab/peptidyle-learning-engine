import type { AssessmentId } from "../../../generated/api/AssessmentId";
import type { QuestionDetails } from "../../../generated/api/QuestionDetails";
import type { QuestionSummary } from "../../../generated/api/QuestionSummary";
import type { QuestionSearchPage } from "../../../generated/api/QuestionSearchPage";
import type { QuestionSearchRequest } from "../../../generated/api/QuestionSearchRequest";
import type { CourseAppearanceView } from "../../../generated/api/CourseAppearanceView";
import type { CourseThemeUpdate } from "../../../generated/api/CourseThemeUpdate";
import type { CourseBannerUpdate } from "../../../generated/api/CourseBannerUpdate";
import type { CourseBannerUploadReceipt } from "../../../generated/api/CourseBannerUploadReceipt";
import type { CourseInstanceId } from "../../../generated/api/CourseInstanceId";
import type { CourseBannerId } from "../../../generated/api/CourseBannerId";
import type { StudentRecordId } from "../../../generated/api/StudentRecordId";
import type { PublishedQuestionId } from "../../../generated/api/PublishedQuestionId";
import type { QuestionAttemptId } from "../../../generated/api/QuestionAttemptId";
import type { ApiClient } from "../client";
import type {
  ProfileAvatarView,
  ProfileImageCropInput,
  SelectProvidedProfileAvatarInput,
} from "../profile_avatar";
import type { ProfileSettings, UpdateAccountSettingsInput } from "../profile_settings";
import type { StudentQuestionAttempt } from "../contracts";
import { questionIdPath, questionSearchPath } from "../question_search_query";
import {
  decodeAssessmentAttempt,
  decodeStudentAssessmentDetail,
  decodeAttemptPage,
  decodeQuestionPage,
  decodeQuestionDetails,
  decodeQuestionLineageView,
  decodeQuestionSearchPage,
  decodeCourseAppearanceView,
  decodeCourseThemeUpdate,
  decodeCourseBannerUpdate,
  decodeCourseBannerUploadReceipt,
  decodeImathasQuestionBackendLaunch,
  decodeStudentQuestionAttempt,
  decodeIssuedQuestionPresentation,
  decodeAssessmentAttemptPage,
  decodeStudentAssessmentProgress,
  decodeNavigationResolution,
} from "../decoders";
import {
  decodeProfileSettings,
  decodeUpdateAccountSettingsInput,
} from "../decoders/profile_settings";
import {
  decodeProfileAvatarView,
  decodeProfileImageCropInput,
  decodeSelectProvidedProfileAvatarInput,
} from "../decoders/profile_avatar";
import { ApiProtocolError, ApiRequestError } from "./error";
import { parseCourseInstanceId } from "../../navigation/public_route";
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
  bannerId: CourseBannerId,
): Promise<Blob> {
  const path = `/api/course-banners/${encodedId(bannerId)}/delivery`;
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
  courseInstanceId: CourseInstanceId,
  assessmentId: AssessmentId,
  attempt: StudentQuestionAttempt,
): Promise<import("../../../generated/api/QuestionPresentation").QuestionPresentation> {
  const path = `${studentAttemptPath(courseInstanceId, assessmentId, attempt.id)}/question`;
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
export async function boundedResponseJson(
  response: Response,
  path: string,
  maximumCharacters = MAX_RESPONSE_CHARACTERS,
): Promise<unknown> {
  responseContentType(response, path);
  const text = await response.text();
  if (text.length === 0 || text.length > maximumCharacters)
    throw new ApiProtocolError(`API response ${path} must contain bounded JSON`);
  return decodeJson(text, path);
}

function courseAppearanceViewPath(courseInstanceId: CourseInstanceId): string {
  if (parseCourseInstanceId(courseInstanceId) === null) {
    throw new ApiProtocolError("Course Instance ID must be canonical");
  }
  // ASVS 1.2.2 and 2.2.1: positively validate, then path-encode route input.
  return `/api/course-instances/${encodeURIComponent(courseInstanceId)}/appearance`;
}

async function profileSettings(
  fetchImplementation: ApiFetch,
  basePath: string,
): Promise<ProfileSettings> {
  const path = "/api/profile";
  const response = await fetchImplementation(requestPath(basePath, path), {
    headers: { accept: "application/json" },
    credentials: "same-origin",
    cache: "no-store",
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  return decodeProfileSettings(await boundedResponseJson(response, path));
}

async function accountSettings(
  fetchImplementation: ApiFetch,
  basePath: string,
): Promise<ProfileSettings> {
  const path = "/api/account/settings";
  const response = await fetchImplementation(requestPath(basePath, path), {
    headers: { accept: "application/json" },
    credentials: "same-origin",
    cache: "no-store",
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  return decodeProfileSettings(await boundedResponseJson(response, path));
}

async function saveAccountSettings(
  fetchImplementation: ApiFetch,
  basePath: string,
  input: UpdateAccountSettingsInput,
): Promise<ProfileSettings> {
  const path = "/api/account/settings";
  const request = decodeUpdateAccountSettingsInput(input, "request");
  const response = await fetchImplementation(requestPath(basePath, path), {
    method: "PUT",
    headers: { accept: "application/json", "content-type": "application/json" },
    body: JSON.stringify(request),
    credentials: "same-origin",
    cache: "no-store",
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  return decodeProfileSettings(await boundedResponseJson(response, path));
}

async function profileAvatar(
  fetchImplementation: ApiFetch,
  basePath: string,
): Promise<ProfileAvatarView> {
  const path = "/api/profile/avatar";
  const response = await fetchImplementation(requestPath(basePath, path), {
    headers: { accept: "application/json" },
    credentials: "same-origin",
    cache: "no-store",
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  return decodeProfileAvatarView(await boundedResponseJson(response, path));
}

async function selectProvidedProfileAvatar(
  fetchImplementation: ApiFetch,
  basePath: string,
  input: SelectProvidedProfileAvatarInput,
): Promise<void> {
  const path = "/api/profile/avatar";
  const request = decodeSelectProvidedProfileAvatarInput(input, "request");
  const response = await fetchImplementation(requestPath(basePath, path), {
    method: "PUT",
    headers: { accept: "application/json", "content-type": "application/json" },
    body: JSON.stringify(request),
    credentials: "same-origin",
    cache: "no-store",
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  if (response.status !== 204)
    throw new ApiProtocolError(`API response ${path} must be 204 No Content`);
}

async function replaceProfileAvatarImage(
  fetchImplementation: ApiFetch,
  basePath: string,
  image: Blob,
  crop: ProfileImageCropInput,
): Promise<ProfileAvatarView> {
  const path = "/api/profile/avatar/profile-image";
  const geometry = decodeProfileImageCropInput(crop);
  if (image.size === 0 || image.size > 8 * 1_024 * 1_024) {
    throw new ApiProtocolError("Profile image must contain at most 8 MiB of original image bytes");
  }
  const response = await fetchImplementation(requestPath(basePath, path), {
    method: "POST",
    headers: {
      accept: "application/json",
      "content-type": "application/octet-stream",
      "x-ple-profile-crop": JSON.stringify(geometry),
    },
    body: image,
    credentials: "same-origin",
    cache: "no-store",
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  return decodeProfileAvatarView(await boundedResponseJson(response, path));
}

const MAX_PROFILE_AVATAR_IMAGE_DELIVERY_BYTES = 2 * 1_024 * 1_024;

async function fetchProfileAvatarImage(
  fetchImplementation: ApiFetch,
  basePath: string,
  profileImageId: string,
): Promise<Blob> {
  const path = `/api/profile/avatar/profile-images/${encodedId(profileImageId)}/delivery`;
  const response = await fetchImplementation(requestPath(basePath, path), {
    method: "POST",
    headers: { accept: "image/webp" },
    credentials: "same-origin",
    cache: "no-store",
  });
  requireNoStore(response, path);
  if (!response.ok) throw new ApiRequestError(response.status, path);
  if (response.headers.get("content-type") !== "image/webp")
    throw new ApiProtocolError(`API response ${path} must be normalized image/webp`);
  if (
    response.headers.get("content-disposition") !== 'attachment; filename="ple-profile-image.webp"'
  )
    throw new ApiProtocolError(
      `API response ${path} must use the protected Profile Image Content-Disposition header`,
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
  if (
    !Number.isSafeInteger(expectedBytes) ||
    expectedBytes > MAX_PROFILE_AVATAR_IMAGE_DELIVERY_BYTES
  )
    throw new ApiProtocolError(`API response ${path} exceeds the profile image byte limit`);
  const blob = await response.blob();
  if (blob.type !== "image/webp" || blob.size !== expectedBytes)
    throw new ApiProtocolError(
      `API response ${path} body does not match its profile image metadata`,
    );
  return blob;
}
async function courseAppearanceView(
  fetchImplementation: ApiFetch,
  basePath: string,
  courseInstanceId: CourseInstanceId,
): Promise<CourseAppearanceView> {
  const path = courseAppearanceViewPath(courseInstanceId);
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
  courseInstanceId: CourseInstanceId,
  update: CourseThemeUpdate,
): Promise<CourseAppearanceView> {
  const path = courseAppearanceViewPath(courseInstanceId);
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
  courseInstanceId: CourseInstanceId,
  image: Blob,
): Promise<CourseBannerUploadReceipt> {
  const path = `${courseAppearanceViewPath(courseInstanceId)}/banner-uploads`;
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
  courseInstanceId: CourseInstanceId,
  update: CourseBannerUpdate,
): Promise<CourseAppearanceView> {
  const path = `${courseAppearanceViewPath(courseInstanceId)}/banner`;
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
  courseInstanceId: CourseInstanceId,
): Promise<CourseAppearanceView> {
  const path = `${courseAppearanceViewPath(courseInstanceId)}/banner`;
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
  questionId: PublishedQuestionId,
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
  | "getProfile"
  | "getAccountSettings"
  | "updateAccountSettings"
  | "getProfileAvatar"
  | "selectProvidedProfileAvatar"
  | "replaceProfileAvatarImage"
  | "fetchProfileAvatarImage"
  | "listQuestions"
  | "searchQuestionLibrary"
  | "resolveQuestion"
  | "getQuestionDetails"
  | "getCourseAppearanceView"
  | "updateCourseTheme"
  | "uploadCourseBanner"
  | "setCourseBanner"
  | "removeCourseBanner"
  | "getAssessment"
  | "getAssessmentSummary"
  | "listAssessmentAttempts"
  | "getAssessmentAttempt"
  | "listQuestionAttempts"
  | "getAttempt"
  | "getIssuedQuestion"
  | "beginImathasQuestionBackendLaunch"
  | "getAssessmentActivitySummary"
  | "fetchCourseBanner"
  | "questionImageUrl"
> {
  return {
    getProfile: () => profileSettings(fetchImplementation, basePath),
    getAccountSettings: () => accountSettings(fetchImplementation, basePath),
    updateAccountSettings: (input) => saveAccountSettings(fetchImplementation, basePath, input),
    getProfileAvatar: () => profileAvatar(fetchImplementation, basePath),
    selectProvidedProfileAvatar: (input) =>
      selectProvidedProfileAvatar(fetchImplementation, basePath, input),
    replaceProfileAvatarImage: (image, crop) =>
      replaceProfileAvatarImage(fetchImplementation, basePath, image, crop),
    fetchProfileAvatarImage: (profileImageId) =>
      fetchProfileAvatarImage(fetchImplementation, basePath, profileImageId),
    resolveNavigation: (id) =>
      requestJson(
        fetchImplementation,
        basePath,
        `/api/navigation/${encodedId(id)}`,
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
    resolveQuestion: (questionId: string): Promise<QuestionSummary> => {
      const path = questionIdPath(questionId);
      return requestJson(fetchImplementation, basePath, path, decodeQuestionLineageView).then(
        (lineage) => lineage.summary,
      );
    },
    getQuestionDetails: (questionId) => questionDetails(fetchImplementation, basePath, questionId),
    getCourseAppearanceView: (courseInstanceId) =>
      courseAppearanceView(fetchImplementation, basePath, courseInstanceId),
    updateCourseTheme: (courseInstanceId, update) =>
      updateCourseTheme(fetchImplementation, basePath, courseInstanceId, update),
    uploadCourseBanner: (courseInstanceId, image) =>
      uploadCourseBanner(fetchImplementation, basePath, courseInstanceId, image),
    setCourseBanner: (courseInstanceId, update) =>
      setCourseBanner(fetchImplementation, basePath, courseInstanceId, update),
    removeCourseBanner: (courseInstanceId) =>
      removeCourseBanner(fetchImplementation, basePath, courseInstanceId),
    getAssessment: (assessmentId: AssessmentId) =>
      requestJson(
        fetchImplementation,
        basePath,
        `/api/assessments/${encodedId(assessmentId)}/student`,
        decodeStudentAssessmentDetail,
      ),
    getAssessmentSummary: (assessmentId: AssessmentId) =>
      requestJson(
        fetchImplementation,
        basePath,
        `/api/assessments/${encodedId(assessmentId)}/summary`,
        decodeStudentAssessmentProgress,
      ),
    listAssessmentAttempts: (studentRecordId: StudentRecordId, cursor) =>
      requestJson(
        fetchImplementation,
        basePath,
        cursorPath(
          `/api/student-records/${encodedId(studentRecordId)}/assessment-attempts`,
          cursor,
        ),
        decodeAssessmentAttemptPage,
      ),
    getAssessmentAttempt: (assessmentAttemptId) =>
      requestJson(
        fetchImplementation,
        basePath,
        `/api/assessment-attempts/${encodedId(assessmentAttemptId)}`,
        decodeAssessmentAttempt,
      ),
    listQuestionAttempts: (assessmentAttemptId, cursor) =>
      requestJson(
        fetchImplementation,
        basePath,
        cursorPath(
          `/api/assessment-attempts/${encodedId(assessmentAttemptId)}/question-attempts`,
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
      courseInstanceId,
      assessmentId,
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
        courseInstanceId,
        assessmentId,
        attempt,
      );
    },
    beginImathasQuestionBackendLaunch: (courseInstanceId, assessmentId, attemptId) =>
      requestJson(
        fetchImplementation,
        basePath,
        `${studentAttemptPath(courseInstanceId, assessmentId, attemptId)}/imathas-question-backend/launch`,
        (value, path = "response") =>
          decodeImathasQuestionBackendLaunch(
            value,
            path,
            courseInstanceId,
            assessmentId,
            attemptId,
          ),
        { method: "POST" },
      ),
    getAssessmentActivitySummary: (studentRecordId) =>
      requestJson(
        fetchImplementation,
        basePath,
        `/api/student-records/${encodedId(studentRecordId)}/assessment-activity-summary`,
        decodeStudentAssessmentProgress,
      ),
    fetchCourseBanner: (bannerId) => fetchCourseBanner(fetchImplementation, basePath, bannerId),
    questionImageUrl: (publishedQuestionRevisionTuple, questionImageAssetId) =>
      requestPath(
        basePath,
        `/api/questions/${encodedId(publishedQuestionRevisionTuple.publishedQuestionId)}/revisions/${publishedQuestionRevisionTuple.revisionNumber}/images/${encodedId(questionImageAssetId)}`,
      ),
  };
}
