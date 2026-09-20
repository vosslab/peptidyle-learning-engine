import type {
  CourseStudentWorkRecoveryClient,
  RecoverySelection,
  RecoveredAttempt,
} from "../course_student_work_recovery";
import {
  decodeRecoveredAttempt,
  decodeRecoverySelection,
} from "../decoders/course_student_work_recovery";
import { parseCourseInstanceId, parseAssessmentAttemptId } from "../../navigation/public_route";
import { ApiProtocolError, ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

export function createCourseStudentWorkRecoveryClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): CourseStudentWorkRecoveryClient {
  async function request(courseInstanceId: string, body: unknown): Promise<unknown> {
    if (parseCourseInstanceId(courseInstanceId) === null)
      throw new ApiProtocolError("Course Instance ID must be canonical");
    // ASVS 14.2.1, 14.3.2: selection cursor and Attempt are POST body only; no cache.
    const path = `/api/course-instances/${encodeURIComponent(courseInstanceId)}/student-work/recovery`;
    const response = await requestSameOrigin(fetchImplementation, basePath, path, {
      method: "POST",
      body,
    });
    requireNoStore(response, path);
    if (!response.ok) throw new ApiRequestError(response.status, path);
    if (response.status !== 200) throw new ApiProtocolError("Recovery must return status 200");
    return boundedResponseJson(response, path);
  }
  return {
    async selectArchivedStudentWork(courseInstanceId, cursor): Promise<RecoverySelection> {
      const selection = decodeRecoverySelection(
        await request(courseInstanceId, { action: "select", cursor }),
      );
      if (
        selection.courseInstanceId !== courseInstanceId ||
        selection.attempts.some((attempt) => attempt.courseInstanceId !== courseInstanceId)
      )
        throw new ApiProtocolError("Recovery selection must match the requested Course");
      return selection;
    },
    async recoverArchivedStudentWork(
      courseInstanceId,
      assessmentAttemptId,
    ): Promise<RecoveredAttempt> {
      if (parseAssessmentAttemptId(assessmentAttemptId) === null)
        throw new ApiProtocolError("Assessment Attempt ID must be canonical");
      const attempt = decodeRecoveredAttempt(
        await request(courseInstanceId, {
          action: "recover",
          assessmentAttemptId,
        }),
      );
      if (
        attempt.courseInstanceId !== courseInstanceId ||
        attempt.assessmentAttemptId !== assessmentAttemptId
      ) {
        throw new ApiProtocolError("Recovery evidence must match the requested Course and Attempt");
      }
      return attempt;
    },
  };
}
