// Strict same-origin transport for explicitly confirmed Sysadmin Student-data access.

import type { CourseInstanceId } from "../../../generated/api/CourseInstanceId";
import type { ApiClient } from "../client";
import type {
  SysadminStudentAccessClient,
  SysadminStudentDataAccess,
} from "../sysadmin_student_access";
import { decodeSysadminStudentDataAccess } from "../decoders/sysadmin_student_access";
import { ApiProtocolError, ApiRequestError } from "./error";
import { requestSameOrigin, type ApiFetch } from "./request";
import { boundedResponseJson, requireNoStore } from "./response";
import { parseCourseInstanceId } from "../../navigation/public_route";

function studentDataPath(courseInstanceId: CourseInstanceId, rosterId: string): string {
  if (parseCourseInstanceId(courseInstanceId) === null) {
    throw new ApiProtocolError("Course Instance ID must be canonical");
  }
  if (!/^[A-Za-z0-9._-]{1,64}$/u.test(rosterId)) {
    throw new ApiProtocolError("Course roster identifier must be canonical");
  }
  return `/api/sysadmin/course-instances/${encodeURIComponent(courseInstanceId)}/roster/${encodeURIComponent(rosterId)}/student-data`;
}

/** Composes only the confirmed Sysadmin Student-data capability. */
export function createSysadminStudentAccessClient(
  fetchImplementation: ApiFetch,
  basePath: string,
): Pick<ApiClient, keyof SysadminStudentAccessClient> {
  return {
    accessSysadminStudentData: async (
      courseInstanceId,
      rosterId,
    ): Promise<SysadminStudentDataAccess> => {
      const path = studentDataPath(courseInstanceId, rosterId);
      const response = await requestSameOrigin(fetchImplementation, basePath, path, {
        method: "POST",
        body: { administrativeAccessConfirmed: true },
      });
      requireNoStore(response, path);
      if (!response.ok) throw new ApiRequestError(response.status, path);
      if (response.status !== 200) {
        throw new ApiProtocolError(`API response ${path} must use status 200`);
      }
      const result = decodeSysadminStudentDataAccess(await boundedResponseJson(response, path));
      if (result.courseInstanceId !== courseInstanceId || result.rosterId !== rosterId) {
        throw new ApiProtocolError(
          `API response ${path} must identify the requested roster record`,
        );
      }
      return result;
    },
  };
}
