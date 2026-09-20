// Same-origin, no-store transport for real Student time configuration.
import type { AssessmentId } from "../../../generated/api/AssessmentId";
import type { AssessmentStudentTimeAccommodation } from "../../../generated/api/AssessmentStudentTimeAccommodation";
import type { CourseInstanceId } from "../../../generated/api/CourseInstanceId";
import type { AssessmentStudentTimeAccommodationClient } from "../assessment_student_time_accommodation";
import type { SaveAssessmentStudentTimeAccommodationInput } from "../../../generated/api/SaveAssessmentStudentTimeAccommodationInput";
import {
  decodeAssessmentStudentTimeAccommodation,
  decodeAccommodationEditNumber,
  decodeStudentTimeMultiplier,
} from "../decoders/assessment_student_time_accommodation";
import { parseAssessmentId, parseCourseInstanceId } from "../../navigation/public_route";
import { ApiProtocolError, ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";

export function createAssessmentStudentTimeAccommodationClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): AssessmentStudentTimeAccommodationClient {
  async function configuration(
    courseInstanceId: CourseInstanceId,
    assessmentId: AssessmentId,
    rosterId: string,
    input?: SaveAssessmentStudentTimeAccommodationInput,
  ): Promise<AssessmentStudentTimeAccommodation> {
    if (
      parseCourseInstanceId(courseInstanceId) === null ||
      parseAssessmentId(assessmentId) === null ||
      !/^[A-Za-z0-9._-]{1,64}$/u.test(rosterId)
    )
      throw new ApiProtocolError("Student time configuration route is invalid");
    const path = `/api/course-instances/${encodeURIComponent(courseInstanceId)}/assessments/${encodeURIComponent(assessmentId)}/student-time-accommodations/${encodeURIComponent(rosterId)}`;
    const body =
      input === undefined
        ? undefined
        : {
            timeMultiplier: decodeStudentTimeMultiplier(
              input.timeMultiplier,
              "input.timeMultiplier",
            ),
            expectedAccommodationEditNumber: decodeAccommodationEditNumber(
              input.expectedAccommodationEditNumber,
              "input.expectedAccommodationEditNumber",
            ),
          };
    const response = await requestSameOrigin(
      fetchImplementation,
      basePath,
      path,
      body === undefined ? { method: "GET" } : { method: "PUT", body },
    );
    requireNoStore(response, path);
    if (!response.ok) throw new ApiRequestError(response.status, path);
    const value = decodeAssessmentStudentTimeAccommodation(
      await boundedResponseJson(response, path),
    );
    if (
      value.rosterId !== rosterId ||
      (input !== undefined && value.timeMultiplier !== input.timeMultiplier)
    ) {
      throw new ApiProtocolError("Student time configuration response does not match the request");
    }
    return value;
  }
  return {
    getAssessmentStudentTimeAccommodation: configuration,
    saveAssessmentStudentTimeAccommodation: configuration,
  };
}
